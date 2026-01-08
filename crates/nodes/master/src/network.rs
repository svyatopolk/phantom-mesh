use futures_util::{SinkExt, StreamExt};
use tokio::net::TcpStream;
use tokio_tungstenite::{connect_async, WebSocketStream, MaybeTlsStream};
use tokio_tungstenite::tungstenite::protocol::Message;
use protocol::{GhostPacket, CommandType, CommandPayload, MeshMsg, PeerInfo, Registration, GossipMsg};
use serde_json::json;
use crate::crypto::{sign_command, create_payload};
use ed25519_dalek::SigningKey;
use url::Url;
use std::path::PathBuf;
use tokio::io::{AsyncRead, AsyncWrite};
use rand_core::OsRng;
use x25519_dalek::{EphemeralSecret, PublicKey};
// OsRng is at line 12

// Ghost P2P Client (Transient Connection)
// Ghost connects, drops payload, disconnects.
// Refactored to be Generic over the Stream Type (S)
// This allows supporting both direct TCP/TLS (Bootstrap) and SOCKS5 (Hidden Services).
pub struct GhostClient<S> {
    ws_stream: WebSocketStream<S>, 
}


impl GhostClient<MaybeTlsStream<TcpStream>> {
    pub async fn connect(url: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let (ws_stream, _) = connect_async(url).await?;
        Ok(GhostClient { ws_stream })
    }
}

impl GhostClient<tokio_socks::tcp::Socks5Stream<TcpStream>> {
    pub async fn connect_via_tor(onion_host: &str, port: u16, proxy_addr: &str) -> Result<Self, Box<dyn std::error::Error>> {
        use tokio_socks::tcp::Socks5Stream;
        use tokio_tungstenite::client_async;
        use url::Url;

        println!("Connecting via SOCKS5 Proxy: {} -> {}:{}", proxy_addr, onion_host, port);
        
        // 1. Connect via SOCKS5 to Onion Service
        let stream = Socks5Stream::connect(proxy_addr, (onion_host, port)).await?;
        
        // 2. Upgrade to WebSocket
        let url_str = format!("ws://{}:{}/ws", onion_host, port);
        
        let (ws_stream, _) = client_async(url_str, stream).await?;
        
        Ok(GhostClient { ws_stream })
    }
}

// Common methods for any valid Stream
impl<S> GhostClient<S> 
where S: AsyncRead + AsyncWrite + Unpin 
{
    pub async fn handshake(&mut self) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
         let mut rng = OsRng;
         let my_secret = EphemeralSecret::random_from_rng(&mut rng);
         let my_public = PublicKey::from(&my_secret);
         
         let msg = MeshMsg::ClientHello { 
             ephemeral_pub: hex::encode(my_public.as_bytes()) 
         };
         self.ws_stream.send(Message::Text(serde_json::to_string(&msg)?.into())).await?;
         
         // Wait for ServerHello
         while let Some(res) = self.ws_stream.next().await {
            if let Ok(Message::Text(txt)) = res {
                 if let Ok(MeshMsg::ServerHello { ephemeral_pub }) = serde_json::from_str::<MeshMsg>(&txt) {
                     let server_pub_bytes = hex::decode(ephemeral_pub)?;
                     let server_pub_arr: [u8; 32] = server_pub_bytes.try_into().map_err(|_| "Invalid Key Length")?;
                     let server_public = PublicKey::from(server_pub_arr);
                     let shared_secret = my_secret.diffie_hellman(&server_public);
                     return Ok(shared_secret.as_bytes().to_vec());
                 }
            }
         }
         Err("Handshake Timeout/Failure".into())
    }

    pub async fn register(&mut self, pub_hex: &str) -> Result<(), Box<dyn std::error::Error>> {
        // PoW Solver
        use sha2::{Sha256, Digest};
        let mut pow_nonce: u64 = 0;
        let start = std::time::Instant::now();
        println!("Ghost Solving PoW...");
        loop {
            let input = format!("{}{}", pub_hex, pow_nonce);
            let hash = Sha256::digest(input.as_bytes());
            if hash[0] == 0 && hash[1] == 0 {
                break;
            }
            pow_nonce += 1;
        }
        println!("PoW Solved in {:?}", start.elapsed());

        let reg = Registration {
            pub_key: pub_hex.to_string(),
            onion_address: "ghost_transient.onion".to_string(),
            signature: "sig".to_string(),
            pow_nonce,
            timestamp: chrono::Utc::now().timestamp(),
        };
        let msg = MeshMsg::Register(reg);
        self.ws_stream.send(Message::Text(serde_json::to_string(&msg)?.into())).await?;
        Ok(())
    }

    pub async fn get_peers(&mut self) -> Result<Vec<PeerInfo>, Box<dyn std::error::Error>> {
        let msg = MeshMsg::GetPeers;
        self.ws_stream.send(Message::Text(serde_json::to_string(&msg)?.into())).await?;
        
        while let Some(res) = self.ws_stream.next().await {
            if let Ok(Message::Text(txt)) = res {
                if let Ok(MeshMsg::Peers(list)) = serde_json::from_str::<MeshMsg>(&txt) {
                    return Ok(list);
                }
            }
        }
        Ok(vec![])
    }
    
    // Inject Gossip into a connected Node
    pub async fn inject_command(&mut self, payload: CommandPayload, sign_key: &SigningKey, session_key: &[u8]) -> Result<(), Box<dyn std::error::Error>> {
        // Encrypt Payload (Ghost -> Mesh)
        // Note: In real mesh, we might use a shared mesh key or individual recipient? 
        // For 'Broadcast', we use a Shared Swarm Key usually, or we re-encrypt at boundaries.
        // The user spec said "Ghost asks Bootstrap for 1 Node, connects, sends Gossip, disconnects".
        // The Payload is encrypted. Let's assume Shared Swarm Key or Node Key.
        // For simplicity: Using Session Key passed in (which is effectively the Swarm Key here for broadcast).
        
        // Sign
        // 1. Serialize Payload
        let json_payload = serde_json::to_string(&payload)?;

        // 2. Encrypt (ChaCha20Poly1305) using Session Key
        use chacha20poly1305::{ChaCha20Poly1305, Key, KeyInit, AeadCore};
        use chacha20poly1305::aead::Aead;
        
        let mut key_bytes = [0u8; 32];
        if session_key.len() == 32 {
            key_bytes.copy_from_slice(session_key);
        } else {
             return Err("Invalid Session Key length".into());
        }
        let cipher = ChaCha20Poly1305::new(&Key::from(key_bytes));
        let nonce = ChaCha20Poly1305::generate_nonce(&mut OsRng); // 96-bits
        let ciphertext = cipher.encrypt(&nonce, json_payload.as_bytes()).map_err(|_| "Encryption Failed")?;
        
        // 3. Create Packet (Signed)
        // CommandType::StartModule is a generic execution wrapper.
        use protocol::CommandType;
        let mut packet = GhostPacket::new(CommandType::StartModule, ciphertext, sign_key);
        
        // Append Nonce to packet? 
        // GhostPacket definition has `nonce: u64` (salt), but standard ChaCha20Poly1305 uses 12-byte nonce.
        // The `GhostPacket` definition in `packet.rs` has `pub nonce: u64`. This is for Replay Protection usually.
        // The ciphertext ITSELF usually includes the Auth Tag. The Nonce for encryption must be transmitted too!
        // Current `GhostPacket` structure: `data` (Vec<u8>).
        // Best practice: Prepend Nonce to ciphertext in `data`.
        
        let mut final_data = nonce.to_vec();
        final_data.extend(packet.data); 
        packet.data = final_data;
        
        // Wrap in GossipMsg
        let gossip = GossipMsg {
            id: payload.id.clone(),
            packet,
            ttl: 5, // 5 hops
        };
        
        let msg = MeshMsg::Gossip(gossip);
        self.ws_stream.send(Message::Text(serde_json::to_string(&msg)?.into())).await?;
        
        Ok(())
    }
}

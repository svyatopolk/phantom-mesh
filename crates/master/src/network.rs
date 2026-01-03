use futures_util::{SinkExt, StreamExt};
use tokio_tungstenite::{connect_async, tungstenite::Message, WebSocketStream, MaybeTlsStream};
use tokio::net::TcpStream;
use uuid::Uuid;
use protocol::{MeshMsg, PeerInfo, GossipMsg, Registration, CommandPayload, GhostPacket};
use ed25519_dalek::{Signer, SigningKey};

// Mocking the P2P Client for now (Direct connection via Tor proxy in real life)
// Ghost connects, drops payload, disconnects.
pub struct GhostClient {
    // In real P2P, this would be an Arti Tor DataStream
    ws_stream: WebSocketStream<MaybeTlsStream<TcpStream>>, 
}

impl GhostClient {
    pub async fn connect(url: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let (ws_stream, _) = connect_async(url).await?;
        Ok(GhostClient { ws_stream })
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
        let packet = GhostPacket::new(&payload, session_key, |data| {
            let signature = sign_key.sign(data);
            hex::encode(signature.to_bytes())
        });
        
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

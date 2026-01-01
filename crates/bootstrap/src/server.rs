use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use std::net::SocketAddr;
use protocol::{MeshMsg, PeerInfo, Registration};
// For now, we use standard TcpListener, assuming Tor maps port 80 to this local port.
// In a real Arti integration, we would use TorClient::launch_onion_service().
use tokio::net::{TcpListener, TcpStream};
use futures_util::{SinkExt, StreamExt};
use tokio_tungstenite::accept_async;
use tokio_tungstenite::tungstenite::Message;

// Peer Storage: PubKey -> PeerInfo
type PeerMap = Arc<RwLock<HashMap<String, PeerInfo>>>;

pub async fn run_bootstrap_node(port: u16) {
    let peers: PeerMap = Arc::new(RwLock::new(HashMap::new()));
    let addr = format!("127.0.0.1:{}", port);
    
    let listener = TcpListener::bind(&addr).await.expect("Failed to bind");
    println!("Bootstrap (Onion Tracker) listening on: {}", addr);

    while let Ok((stream, addr)) = listener.accept().await {
        let peers = peers.clone();
        tokio::spawn(handle_connection(peers, stream, addr));
    }
}

async fn handle_connection(peers: PeerMap, stream: TcpStream, _addr: SocketAddr) {
    let mut ws_stream = match accept_async(stream).await {
        Ok(ws) => ws,
        Err(_) => return,
    };

    while let Some(msg) = ws_stream.next().await {
        if let Ok(Message::Text(text)) = msg {
            if let Ok(mesh_msg) = serde_json::from_str::<MeshMsg>(&text) {
                match mesh_msg {
                    MeshMsg::Register(reg) => {
                        // TODO: Verify Signature (reg.signature signs "Register:<onion>")
                        // For now, assume valid.
                        let info = PeerInfo {
                            pub_key: reg.pub_key.clone(),
                            onion_address: reg.onion_address,
                            last_seen: chrono::Utc::now().timestamp(),
                        };
                        
                        println!("[+] New Node Registered: {} -> {}", info.pub_key, info.onion_address);
                        peers.write().await.insert(reg.pub_key, info);
                    }
                    MeshMsg::GetPeers => {
                        let reader = peers.read().await;
                        let list: Vec<PeerInfo> = reader.values().cloned().collect();
                        // Return random 50 (Mocking random selection for now)
                        let limit = std::cmp::min(list.len(), 50);
                        let sublist = list[..limit].to_vec();
                        
                        let resp = MeshMsg::Peers(sublist);
                        let json = serde_json::to_string(&resp).unwrap();
                        let _ = ws_stream.send(Message::Text(json.into())).await;
                    }
                    _ => {} // Bootstrap ignores Gossip/Other messages
                }
            }
        }
    }
}

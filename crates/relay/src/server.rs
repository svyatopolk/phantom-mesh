use std::net::SocketAddr;
use futures_util::{SinkExt, StreamExt};
use tokio::net::TcpStream;
use tokio::sync::mpsc;
use tokio_tungstenite::accept_async;
use tokio_tungstenite::tungstenite::Message;
use protocol::SignalMsg;
use crate::state::PeerMap;

pub async fn handle_connection(peers: PeerMap, stream: TcpStream, addr: SocketAddr) {
    let ws_stream = match accept_async(stream).await {
        Ok(ws) => ws,
        Err(e) => {
            eprintln!("Handshake error: {}", e);
            return;
        }
    };
    
    // Split WS
    let (mut ws_tx, mut ws_rx) = ws_stream.split();

    // Internal Channel for this peer
    let (ch_tx, mut ch_rx) = mpsc::unbounded_channel::<Message>();

    // 1. Write Loop (Async Task)
    tokio::spawn(async move {
        while let Some(msg) = ch_rx.recv().await {
            if ws_tx.send(msg).await.is_err() {
                break; 
            }
        }
    });

    // 2. Read Loop (Main Task)
    let mut my_id: Option<String> = None;

    while let Some(msg) = ws_rx.next().await {
        let msg = match msg {
            Ok(m) => m,
            Err(_) => { break; } // Conn error
        };

        if let Message::Text(text) = msg {
            // Use Protocol Crate for Deserialization
            if let Ok(signal) = serde_json::from_str::<SignalMsg>(&text) {
                match signal {
                    SignalMsg::Register { pub_key } => {
                        println!("Registered: {} [{}]", pub_key, addr);
                        my_id = Some(pub_key.clone());
                        peers.insert(pub_key, ch_tx.clone());
                    }
                    SignalMsg::GetPeers => {
                        use rand::seq::IndexedRandom;
                        let keys: Vec<String> = peers.iter().map(|r| r.key().clone()).collect();
                        let mut rng = rand::rng();
                        let limited: Vec<String> = keys
                            .choose_multiple(&mut rng, 20)
                            .cloned()
                            .collect();
                        
                        let resp = SignalMsg::Peers { list: limited };
                        let json = serde_json::to_string(&resp).unwrap();
                        let _ = ch_tx.send(Message::Text(json.into()));
                    }
                    SignalMsg::Signal { target, data } => {
                        if let Some(peer_tx) = peers.get(&target) {
                            let source = my_id.clone().unwrap_or("UNKNOWN".to_string());
                            let fwd = SignalMsg::RelaySignal { source, data };
                            let json = serde_json::to_string(&fwd).unwrap();
                            let _ = peer_tx.send(Message::Text(json.into()));
                        }
                    }
                    _ => {}
                }
            }
        } else if msg.is_close() {
            break;
        }
    }

    // Cleanup
    if let Some(id) = my_id {
        println!("Disconnected: {}", id);
        peers.remove(&id);
    }
}

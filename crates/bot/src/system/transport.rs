use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::time::Instant;
use tokio_tungstenite::WebSocketStream;
use arti_client::{TorClient, DataStream};
use tor_rtcompat::PreferredRuntime;
use tokio_tungstenite::client_async;
use futures_util::SinkExt;
use tokio_tungstenite::tungstenite::Message;

// Max connections in pool to prevent RAM exhaustion
const MAX_POOL_SIZE: usize = 50;

// Type alias for our Stream
// Type alias for our Stream (No TLS inside Tor)
pub type WsStream = WebSocketStream<DataStream>;

pub struct ActivePool {
    // Key: Onion Address
    streams: HashMap<String, Arc<Mutex<WsStream>>>,
    last_activity: HashMap<String, Instant>,
}

impl ActivePool {
    pub fn new() -> Self {
        Self {
            streams: HashMap::new(),
            last_activity: HashMap::new(),
        }
    }

    /// Smart Send Algorithm: Use cached stream or connect new
    pub async fn send_msg(
        &mut self, 
        tor: &TorClient<PreferredRuntime>, 
        onion: &str, 
        msg_json: String,
        protected_peers: &[String]
    ) -> Result<(), String> {
        
        // 1. FAST PATH (Hit)
        let mut failed_cache = false;
        if let Some(stream_mutex) = self.streams.get(onion).cloned() {
            let mut stream = stream_mutex.lock().await;
            if stream.send(Message::Text(msg_json.clone().into())).await.is_ok() {
                self.last_activity.insert(onion.to_string(), Instant::now());
                return Ok(());
            } else {
                failed_cache = true;
            }
        }
        
        if failed_cache {
             self.streams.remove(onion);
             self.last_activity.remove(onion);
        }

        // 2. SLOW PATH (Miss) - Connect
        // Check eviction (Simplified LRU: remove oldest)
        if self.streams.len() >= MAX_POOL_SIZE {
            self.evict_one(protected_peers);
        }

        let host = onion.to_string(); 
        let port = 80;
        let target_url = format!("ws://{}/gossip", onion);

        match tor.connect((host, port)).await {
            Ok(data_stream) => {
                // Handshake
                match client_async(target_url, data_stream).await {
                    Ok((mut ws_stream, _)) => {
                        // Send Msg
                        if let Err(e) = ws_stream.send(Message::Text(msg_json.into())).await {
                             return Err(format!("Send failed: {}", e));
                        }
                        
                        // Cache it
                        self.streams.insert(onion.to_string(), Arc::new(Mutex::new(ws_stream)));
                        self.last_activity.insert(onion.to_string(), Instant::now());
                        Ok(())
                    },
                    Err(e) => Err(format!("Handshake failed: {}", e)),
                }
            },
            Err(e) => Err(format!("Tor Connect failed: {}", e)),
        }
    }

    fn evict_one(&mut self, protected: &[String]) {
        // Find oldest activity that is NOT protected.
        // Eviction Policy: LRU (Least Recently Used) with Whitelist.
        
        let mut oldest_onion = String::new();
        let mut oldest_time = Instant::now() + std::time::Duration::from_secs(3600); // Future
        
        for (k, v) in &self.last_activity {
            // SKIP if Protected
            if protected.contains(k) { continue; }
            
            if *v < oldest_time {
                oldest_time = *v;
                oldest_onion = k.clone();
            }
        }
        
        if !oldest_onion.is_empty() {
             self.streams.remove(&oldest_onion);
             self.last_activity.remove(&oldest_onion);
        } else {
            // All candidates are protected?
            // Fallback: Evict oldest even if protected (Safety Valve to prevent deadlock)
            // Or log warning.
            // Requirement says "NEVER". We will do nothing and let the pool grow slightly or reject?
            // Current strict impl: Do nothing. (Pool overflow by 1 temporarily is better than breaking DHT).
            // Actually, if we don't evict, we just don't add the new one?
            // Wait, we add at the end of send_msg regardless.
            // So we might go over MAX_POOL_SIZE.
            // That's acceptable for "NEVER break DHT".
        }
    }
}

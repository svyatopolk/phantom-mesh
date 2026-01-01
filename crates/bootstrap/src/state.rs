use std::sync::Arc;
use dashmap::DashMap;
use tokio::sync::mpsc;
use tokio_tungstenite::tungstenite::Message;

pub struct PeerContext {
    pub tx: mpsc::UnboundedSender<Message>,
    pub session_key: Option<Vec<u8>>, // Shared Secret
}

// Map Identity (PubKey) -> Context
pub type PeerMap = Arc<DashMap<String, PeerContext>>;

pub fn init_state() -> PeerMap {
    Arc::new(DashMap::new())
}

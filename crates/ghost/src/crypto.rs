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

pub fn create_payload(action: String) -> CommandPayload {
    // Default ExecuteAt = Now + 60s
    let execute_at = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64 + 60;
        
    CommandPayload {
        id: uuid::Uuid::new_v4().to_string(),
        action,
        execute_at,
    }
}

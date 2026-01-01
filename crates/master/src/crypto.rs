use ed25519_dalek::{Signer, SigningKey, VerifyingKey};
use rand::rngs::OsRng;
use std::fs;
use std::path::PathBuf;
use protocol::{CommandPayload, GhostPacket};

pub fn generate_key(output: &PathBuf) -> String {
    let mut csprng = OsRng;
    let signing_key = SigningKey::generate(&mut csprng);
    let verifying_key = VerifyingKey::from(&signing_key);
    
    fs::write(output, signing_key.to_bytes()).expect("Failed to write key");
    hex::encode(verifying_key.to_bytes())
}

pub fn load_key(path: &PathBuf) -> SigningKey {
    let key_bytes = fs::read(path).expect("Failed to read key file");
    let arr: [u8; 32] = key_bytes[0..32].try_into().expect("Invalid key length");
    SigningKey::from_bytes(&arr)
}

pub fn sign_packet(key: &SigningKey, action: String) -> GhostPacket {
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;
        
    let payload = CommandPayload {
        action,
        nonce: rand::random::<u64>(),
        timestamp: ts,
    };
    
    // Use Protocol's E2EE Constructor
    GhostPacket::new(&payload, |data| {
        use ed25519_dalek::Signer;
        let signature = key.sign(data);
        hex::encode(signature.to_bytes())
    })
}

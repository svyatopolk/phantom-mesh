use ed25519_dalek::{SigningKey, Signer, Signature};
use rand::rngs::OsRng;
use std::fs;
use std::path::PathBuf;

pub struct AgentIdentity {
    #[allow(dead_code)]
    pub keypair: SigningKey,
    pub pub_hex: String,
}

pub fn load_or_generate_keys(path: PathBuf) -> AgentIdentity {
    if path.exists() {
        if let Ok(bytes) = fs::read(&path) {
            if let Ok(bytes_array) = bytes.as_slice().try_into() {
                let kp = SigningKey::from_bytes(bytes_array);
                let vk = kp.verifying_key();
                return AgentIdentity {
                    pub_hex: hex::encode(vk.to_bytes()),
                    keypair: kp,
                };
            }
        }
    }

    // Generate New
    let mut csprng = OsRng;
    let keypair = SigningKey::generate(&mut csprng);
    
    // Save
    let _ = fs::write(path, keypair.to_bytes());

    AgentIdentity {
        pub_hex: hex::encode(keypair.verifying_key().to_bytes()),
        keypair,
    }
}

#[allow(dead_code)]
pub fn sign_message(keypair: &SigningKey, message: &str) -> String {
    let signature: Signature = keypair.sign(message.as_bytes());
    hex::encode(signature.to_bytes())
}

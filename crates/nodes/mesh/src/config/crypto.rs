use ed25519_dalek::{SigningKey, Signer, Signature};
use rand::rngs::OsRng;
use std::fs;
use std::path::PathBuf;

pub struct AgentIdentity {
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

    let mut csprng = OsRng;
    let keypair = SigningKey::generate(&mut csprng);
    let _ = fs::write(path, keypair.to_bytes());

    AgentIdentity {
        pub_hex: hex::encode(keypair.verifying_key().to_bytes()),
        keypair,
    }
}

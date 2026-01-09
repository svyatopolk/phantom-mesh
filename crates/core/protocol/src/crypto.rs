use ed25519_dalek::{Signer, Verifier, SigningKey, VerifyingKey, Signature};
use chacha20poly1305::{ChaCha20Poly1305, Key, KeyInit, Nonce};
use chacha20poly1305::aead::Aead;
use rand::RngCore;

/// Encrypt a payload using ChaCha20Poly1305.
/// The nonce is prepended to the ciphertext (12 bytes + ciphertext).
/// For this simplified version, we use a hardcoded key derived from "PhantomQUIC".
/// In production, this would use a proper key exchange.
pub fn encrypt_payload(payload: &[u8]) -> Result<Vec<u8>, String> {
    // Hardcoded session key (in production, derive from ECDH or similar)
    let key_bytes: [u8; 32] = *blake3::hash(b"PhantomQUIC_Session_Key_v1").as_bytes();
    let key = Key::from_slice(&key_bytes);
    let cipher = ChaCha20Poly1305::new(key);
    
    let mut nonce_bytes = [0u8; 12];
    rand::thread_rng().fill_bytes(&mut nonce_bytes);
    let nonce = Nonce::from_slice(&nonce_bytes);
    
    let ciphertext = cipher.encrypt(nonce, payload)
        .map_err(|e| format!("Encrypt: {}", e))?;
    
    // Prepend nonce to ciphertext
    let mut result = Vec::with_capacity(12 + ciphertext.len());
    result.extend_from_slice(&nonce_bytes);
    result.extend_from_slice(&ciphertext);
    Ok(result)
}

/// Decrypt a payload using ChaCha20Poly1305.
/// Expects nonce (12 bytes) prepended to ciphertext.
pub fn decrypt_payload(data: &[u8]) -> Result<Vec<u8>, String> {
    if data.len() < 12 {
        return Err("Data too short".into());
    }
    
    let key_bytes: [u8; 32] = *blake3::hash(b"PhantomQUIC_Session_Key_v1").as_bytes();
    let key = Key::from_slice(&key_bytes);
    let cipher = ChaCha20Poly1305::new(key);
    
    let nonce = Nonce::from_slice(&data[..12]);
    let ciphertext = &data[12..];
    
    let plaintext = cipher.decrypt(nonce, ciphertext)
        .map_err(|e| format!("Decrypt: {}", e))?;
    
    Ok(plaintext)
}

/// Sign a payload with Ed25519
pub fn sign_payload(key: &SigningKey, payload: &[u8]) -> [u8; 64] {
    let signature = key.sign(payload);
    signature.to_bytes()
}

/// Verify a payload signature
pub fn verify_signature(pub_key_bytes: &[u8], payload: &[u8], signature_bytes: &[u8; 64]) -> bool {
    if let Ok(pub_key) = VerifyingKey::from_bytes(pub_key_bytes.try_into().unwrap_or(&[0; 32])) {
        let signature = Signature::from_bytes(signature_bytes);
        return pub_key.verify(payload, &signature).is_ok();
    }
    false
}

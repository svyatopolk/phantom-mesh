// Imports cleaned
#[allow(unused_imports)]
use std::path::Path;

#[allow(dead_code)]
pub fn get_mesh_health() -> f32 {
    #[cfg(windows)]
    {
        use winreg::enums::*;
        use winreg::RegKey;

        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        if let Ok(key) = hkcu.open_subkey("Software\\Microsoft\\Windows\\CurrentVersion\\SystemChek") {
             if let Ok(nodes_str) = key.get_value::<String, _>("Nodes") {
                 let nodes: Vec<&str> = nodes_str.split(';').filter(|s| !s.is_empty()).collect();
                 let total = nodes.len();
                 if total == 0 { return 0.0; }
                 
                 let mut alive = 0;
                 for node in &nodes {
                     if std::path::Path::new(node).exists() {
                         alive += 1;
                     }
                 }
                 return alive as f32 / total as f32;
             }
        }
    }
    // Fallback for non-windows or no registry key
    0.0
}

// Wallet config update removed (Modular Payload handles this)
pub fn update_wallet_config(_new_wallet: &str) -> bool {
    // No-op for Loader
    true
}

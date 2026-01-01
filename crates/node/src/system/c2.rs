use std::time::Duration;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::time;
use lru::LruCache;
use std::num::NonZeroUsize;
use protocol::{MeshMsg, PeerInfo, GhostPacket, CommandPayload, GossipMsg, Registration};
use crate::common::crypto::load_or_generate_keys;
use crate::utils::paths::get_appdata_dir;
use arti_client::{TorClient, TorClientConfig};
use tor_rtcompat::PreferredRuntime;
use rand::seq::SliceRandom;
use futures_util::{SinkExt, StreamExt};
use tokio_tungstenite::{accept_async, client_async, tungstenite::Message};
use url::Url;

struct MeshState {
    peers: HashMap<String, PeerInfo>,
    seen_messages: LruCache<String, i64>,
    my_onion: String,
}

pub async fn start_client() -> Result<(), Box<dyn std::error::Error>> {
    println!("Starting Tor Mesh Node (Arti Native)...");
    
    // 1. Identity
    let key_path = get_appdata_dir().join("sys_keys.dat");
    let identity = load_or_generate_keys(key_path);
    let my_pub_hex = identity.pub_hex.clone();
    
    // 2. Bootstrapping Tor
    let config = TorClientConfig::default();
    let tor_client = TorClient::create_bootstrapped(config).await?;
    
    // 3. Launch Hidden Service (REAL)
    println!("Launching Onion Service...");
    // Create an ephemeral nickname for this session
    let svc_nickname = format!("node-{}", &my_pub_hex[0..8]);
    
    // Import ConfigBuilder from specific path or use default if available easily
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
    
    println!("Hidden Service Active: {}", my_onion);

    let state = Arc::new(RwLock::new(MeshState {
        peers: HashMap::new(),
        seen_messages: LruCache::new(NonZeroUsize::new(1000).unwrap()),
        my_onion: my_onion.clone(),
    }));

    // Register...
    let bootstrap_onion = env!("BOOTSTRAP_ONION");
    println!("Registering with Bootstrap: {}", bootstrap_onion);
    if let Err(e) = register_via_tor(&tor_client, &state, bootstrap_onion, &my_pub_hex, &my_onion, &identity.keypair).await {
         eprintln!("Bootstrap Registration Failed: {}", e);
    }

    let state_clone = state.clone();
    let tor_clone = tor_client.clone();
    
    // Spawn Service Listener
    tokio::spawn(async move {
        println!("Listening for Inbound Gossip...");
        while let Some(rend_req) = stream.next().await {
            let req: tor_hsservice::RendRequest = rend_req;
            
            // Accept the rendezvous (Session)
            let mut session_stream = match req.accept().await {
                Ok(s) => s,
                Err(e) => {
                     eprintln!("Failed to accept rendezvous: {}", e);
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
                             eprintln!("Failed to accept data stream: {}", e);
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

    // 6. Keep-Alive
    loop {
        time::sleep(Duration::from_secs(60)).await;
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
    
    while let Some(msg) = ws_stream.next().await {
        if let Ok(Message::Text(text)) = msg {
            if let Ok(gossip) = serde_json::from_str::<GossipMsg>(&text) {
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
    // Wait, earlier I used ("host", 80) which assumes a map? 
    // No, tor.connect() typically takes a TorAddr. 
    // If 'bootstrap_onion' is "xyz.onion:80", we can parse it.
    // Arti's `connect` is flexible. Let's try parsing the hostname.
    // But `ws://` url parsing gives host.
    
    // Connect to Bootstrap
    // We need to parse port and host from the env var or assume port 80?
    // Env var "BOOTSTRAP_ONION" likely contains "Address:Port".
    // Arti connect takes (host, port).
    // Let's parse it properly.
    
    // Simplified parsing:
    let (host, port) = if let Some(idx) = bootstrap_onion.find(':') {
        (&bootstrap_onion[0..idx], bootstrap_onion[idx+1..].parse::<u16>().unwrap_or(80))
    } else {
        (bootstrap_onion, 80)
    };
    
    let stream = tor.connect((host.to_string(), port)).await?;
    
    let (mut ws_stream, _) = client_async(target_url, stream).await?;
    
    // Create Registration
    let sig_data = format!("Register:{}", my_onion);
    use ed25519_dalek::Signer;
    let signature = hex::encode(signing_key.sign(sig_data.as_bytes()).to_bytes());
    
    let reg = Registration {
        pub_key: my_pub.to_string(),
        onion_address: my_onion.to_string(),
        signature,
        timestamp: chrono::Utc::now().timestamp(),
    };
    
    let msg = MeshMsg::Register(reg);
    let json = serde_json::to_string(&msg)?;
    ws_stream.send(Message::Text(json.into())).await?;
    
    // Receiving logic same as before...
     if let Some(Ok(Message::Text(resp_text))) = ws_stream.next().await {
        if let Ok(MeshMsg::Peers(peers)) = serde_json::from_str::<MeshMsg>(&resp_text) {
            let mut guard = state.write().await;
            for p in peers {
                guard.peers.insert(p.pub_key.clone(), p);
            }
            println!("Bootstrap Success. Received {} peers.", guard.peers.len());
        }
    }
    
    Ok(())
}

async fn handle_gossip(state: Arc<RwLock<MeshState>>, msg: GossipMsg, tor: &TorClient<PreferredRuntime>) {
    let mut guard = state.write().await;
    
    if guard.seen_messages.contains(&msg.id) { return; }
    guard.seen_messages.put(msg.id.clone(), chrono::Utc::now().timestamp());

    let swarm_key_hex = env!("SWARM_KEY");
    let swarm_key = hex::decode(swarm_key_hex).unwrap_or(vec![0u8; 32]);
    
    if let Some(cmd) = packet_verify_and_decrypt(&msg.packet, &swarm_key) {
        // Secure Time Check (NTP)
        let now = get_secure_time().await;
        
        // Allow 30s drift
        if cmd.execute_at <= now + 30 {
             process_command(&cmd);
        } else {
             println!("Command Timelocked until {} (Server Time). Current: {}", cmd.execute_at, now);
             tokio::spawn(async move {
                 let wait_s = if cmd.execute_at > now { (cmd.execute_at - now) as u64 } else { 0 };
                 time::sleep(Duration::from_secs(wait_s)).await;
                 process_command(&cmd);
             });
        }
    } else {
        return; // Invalid signature
    }

    if msg.ttl > 0 {
        let peers: Vec<String> = guard.peers.values().map(|p| p.onion_address.clone()).collect();
        let targets = select_gossip_targets(peers);
        println!("Gossip Fanout: Selected {}/{} peers", targets.len(), guard.peers.len());
        
        let next_msg = GossipMsg { ttl: msg.ttl - 1, ..msg };
        for target in targets {
            let m = next_msg.clone();
            let t = tor.clone();
            tokio::spawn(async move {
                send_gossip(t, target, m).await;
            });
        }
    }
}

async fn get_secure_time() -> i64 {
    // Attempt NTP sync
    let time_res = tokio::task::spawn_blocking(|| {
        let socket = std::net::UdpSocket::bind("0.0.0.0:0").ok()?;
        socket.set_read_timeout(Some(Duration::from_secs(5))).ok()?;
        
        match sntpc::simple_get_time("pool.ntp.org:123", &socket) {
            Ok(t) => {
                let ntp_sec = t.sec();
                let unix_sec = ntp_sec as i64 - 2_208_988_800;
                Some(unix_sec)
            },
            Err(_) => None,
        }
    }).await;
    
    // Fallback to local time if NTP fails
    if let Ok(Some(ntp_time)) = time_res {
        ntp_time
    } else {
        println!("[-] NTP Sync Failed. Using System Time.");
        chrono::Utc::now().timestamp()
    }
}

async fn send_gossip(tor: TorClient<PreferredRuntime>, target_onion: String, msg: GossipMsg) {
    let target_url = format!("ws://{}/gossip", target_onion);
    // Parse Host/Port 
    let host = target_onion; 
    let port = 80;

    match tor.connect((host, port)).await { 
        Ok(stream) => {
            match client_async(target_url, stream).await {
                Ok((mut ws_stream, _)) => {
                    let json = serde_json::to_string(&msg).unwrap();
                    let _ = ws_stream.send(Message::Text(json.into())).await;
                },
                Err(_) => {},
            }
        },
        Err(_) => {},
    }
}

fn select_gossip_targets(peers: Vec<String>) -> Vec<String> {
    let total = peers.len();
    if total == 0 { return vec![]; }
    
    // Improved Logic: 100% Fanout if network is sparse (< 10 peers)
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
    println!("EXECUTING: {} [{}]", cmd.action, cmd.id);
}

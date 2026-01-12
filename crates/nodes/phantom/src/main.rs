use std::io::{self, Write};
use network::GhostClient;
use std::sync::{Arc, Mutex};
use std::collections::HashSet;
use clap::Parser;
use std::path::PathBuf;
use libp2p::identity::Keypair;

mod crypto;
mod network;
mod commands;
mod discovery;

#[derive(Parser, Debug)]
#[command(name = "phantom")]
#[command(about = "Phantom Mesh Controller", long_about = None)]
struct Args {
    /// Path to the private key file (required for running)
    #[arg(short, long, required_unless_present = "keygen")]
    key: Option<PathBuf>,

    /// Generate a new keypair and save to this path
    #[arg(long)]
    keygen: Option<PathBuf>,
}

#[tokio::main]
async fn main() {
    env_logger::init();
    
    let args = Args::parse();
    
    // 0. Key Generation Mode
    if let Some(path) = args.keygen {
        println!("[*] Generating new Ed25519 Keypair...");
        let pub_hex = crypto::generate_key(&path);
        println!("[+] Private Key saved to: {:?}", path);
        println!("[+] Public Key (Hex): {}", pub_hex);
        return;
    }

    // 1. Load Identity
    let key_path = args.key.expect("Key path is required");
    println!("[*] Loading Identity from: {:?}", key_path);
    
    // Load raw dalek Key
    let signing_key = crypto::load_key(&key_path);
    let key_bytes = signing_key.to_bytes();
    
    // Convert to Libp2p Keypair
    let mut key_bytes_mut = key_bytes;
    let libp2p_key = libp2p::identity::Keypair::ed25519_from_bytes(&mut key_bytes_mut)
        .expect("Failed to convert key to libp2p identity");

    println!("[*] Initializing Network Layer...");

    // 2. Initialize Network (Persisted)
    let mut client = match GhostClient::new(libp2p_key).await {
        Ok(c) => c,
        Err(e) => {
            eprintln!("[-] Fatal: Failed to start Network: {}", e);
            return;
        }
    };
    
    // 3. Setup Local Discovery (Stealth mDNS)
    // Use mpsc channel to send discovered peers to main loop for dialing
    let local_peers = Arc::new(Mutex::new(HashSet::<String>::new()));
    let peer_id = client.get_peer_id();
    
    // Channel for discovered peers
    let (lan_tx, mut lan_rx) = tokio::sync::mpsc::channel::<String>(32);

    match common::discovery::local::LocalDiscovery::new(peer_id.clone(), 0).await {
        Ok(mut ld) => {
             let peers_clone = local_peers.clone();
             let tx = lan_tx.clone();
             tokio::spawn(async move {
                 println!("[Local] Discovery Listener Active (UDP 5353).");
                 loop {
                     if let Some(peer) = ld.next_event().await {
                         // Use TCP for libp2p signaling (not UDP/QUIC)
                         let addr_str = format!("/ip4/{}/tcp/{}", peer.addr.ip(), peer.addr.port());
                         
                         // Check and insert in scoped block to drop MutexGuard before await
                         let should_dial = {
                             let mut guard = peers_clone.lock().unwrap();
                             if !guard.contains(&addr_str) {
                                 println!("[Local] NEW Peer Discovered: {} @ {}", peer.peer_id, peer.addr);
                                 guard.insert(addr_str.clone());
                                 true
                             } else {
                                 // Already known peer - silent skip
                                 false
                             }
                         }; // MutexGuard dropped here
                         
                         // Send to main loop for dialing (outside of lock scope)
                         if should_dial {
                             println!("[Local] Sending dial request via channel: {}", addr_str);
                             match tx.send(addr_str.clone()).await {
                                 Ok(_) => println!("[Local] Channel send SUCCESS"),
                                 Err(e) => eprintln!("[Local] Channel send FAILED: {}", e),
                             }
                         }
                     }
                 }
             });
        },
        Err(e) => {
            eprintln!("[-] Local Discovery Init Failed: {}", e);
        }
    }
    
    println!("[+] Network Initialized. Type 'help' for commands.");

    // 4. Interactive Loop with async LAN peer dialing
    use tokio::io::AsyncBufReadExt;
    let stdin = tokio::io::stdin();
    let reader = tokio::io::BufReader::new(stdin);
    let mut lines = reader.lines();
    
    print!("ghost> ");
    io::stdout().flush().unwrap();
    
    loop {
        tokio::select! {
            // Handle LAN peer discovery - dial immediately
            Some(peer_addr) = lan_rx.recv() => {
                println!("\n[Local] Auto-dialing discovered peer: {}", peer_addr);
                match client.dial(&peer_addr).await {
                    Ok(_) => println!("[Local] Dial SUCCESS: {}", peer_addr),
                    Err(e) => eprintln!("[Local] Dial FAILED: {} - {}", peer_addr, e),
                }
                print!("ghost> ");
                io::stdout().flush().unwrap();
            }
            
            // Handle user input
            result = lines.next_line() => {
                match result {
                    Ok(Some(input)) => {
                        let input = input.trim();
                        let parts: Vec<&str> = input.split_whitespace().collect();
                        
                        if parts.is_empty() {
                            print!("ghost> ");
                            io::stdout().flush().unwrap();
                            continue;
                        }

                        match parts[0] {
                            "ping" => {
                               commands::handle_ping(&mut client, local_peers.clone()).await;
                            },
                            "scan" => {
                                commands::handle_scan().await;
                            },
                            "help" => {
                                println!("Available commands:");
                                println!("  ping  - Discover peers (DHT + LAN) and check connectivity");
                                println!("  scan  - Run Parasitic DHT discovery (Legacy DGA)");
                                println!("  exit  - Shutdown node");
                            },
                            "exit" | "quit" => {
                                println!("[*] Shutting down...");
                                break;
                            },
                            _ => {
                                println!("Unknown command: {}", parts[0]);
                            }
                        }
                        print!("ghost> ");
                        io::stdout().flush().unwrap();
                    },
                    Ok(None) => break, // EOF
                    Err(e) => {
                        eprintln!("Input error: {}", e);
                        break;
                    }
                }
            }
        }
    }
}

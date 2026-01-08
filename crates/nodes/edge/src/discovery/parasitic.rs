use mainline::{Dht, Id};
use std::net::SocketAddr;
use crate::discovery::oracle::Oracle;

pub struct ParasiticDiscovery {
    dht: Dht,
}

impl ParasiticDiscovery {
    pub fn new() -> Self {
        Self { dht: Dht::default() }
    }

    /// EDGE ROLE: Find Mesh nodes from the DHT network
    pub async fn edge_role_find_peers(&self) -> Result<Vec<SocketAddr>, Box<dyn std::error::Error>> {
        // 1. Get InfoHash
        let info_hash = Oracle::generate_daily_info_hash()?;
        let info_hash_id = Id::from_bytes(&info_hash).unwrap();

        println!("* Seeking Peers on InfoHash: {}", hex::encode(info_hash));

        // 2. Get Peers
        let mut peers = Vec::new();
        
        // Real Mainline DHT Search
        println!("* DHT: Searching for Mesh Infrastructure...");
        let mut count = 0;
        
        let response = self.dht.get_peers(info_hash_id);
        for peer in response.closest_nodes {
             println!("+ Found Mesh Peer (DHT): {:?}", peer);
             peers.push(peer.address);
             count += 1;
             if count >= 5 { break; } 
        }

        if peers.is_empty() {
             eprintln!("- DHT: No Mesh Nodes found.");
        }

        Ok(peers)
    }
}

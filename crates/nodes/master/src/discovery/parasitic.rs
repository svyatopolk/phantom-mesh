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

    /// SEEKER ROLE: Find Mesh nodes from the DHT network
    pub async fn find_mesh_nodes(&self) -> Result<Vec<SocketAddr>, Box<dyn std::error::Error>> {
        // 1. Get InfoHash
        let info_hash = Oracle::generate_daily_info_hash()?;
        let info_hash_id = Id::from_bytes(&info_hash).unwrap();

        println!("* Seeking Peers on InfoHash: {}", hex::encode(info_hash));

        // 2. Get Peers
        let mut peers = Vec::new();
        
        // Real Mainline DHT Search
        println!("* DHT: Searching for providers...");
        let mut count = 0;
        
        // In this Mainline version, get_peers returns the nodes handling the infohash.
        // For a parasitic bot, these nodes ARE likely the Mesh nodes we want (or they know them).
        let response = self.dht.get_peers(info_hash_id);
        for peer in response.closest_nodes {
             println!("+ Found Peer (DHT Node): {:?}", peer);
             peers.push(peer.address); // Correct field 'address'
             count += 1;
             if count >= 10 { break; } 
        }

        if peers.is_empty() {
             eprintln!("- DHT: No peers found for InfoHash.");
        }

        Ok(peers)
    }
}

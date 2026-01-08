use mainline::{Dht, Id};
use crate::discovery::oracle::Oracle;

pub struct ParasiticDiscovery {
    dht: Dht,
}

impl ParasiticDiscovery {
    pub fn new() -> Self {
        Self { dht: Dht::default() }
    }

    /// MESH ROLE: Announce self to the DHT network
    pub async fn map_role_announce(&self, port: u16) -> Result<(), Box<dyn std::error::Error>> {
        // 1. Get InfoHash
        let info_hash = Oracle::generate_daily_info_hash()?;
        let info_hash_id = Id::from_bytes(&info_hash).unwrap();

        println!("* Announcing Presence on InfoHash: {}", hex::encode(info_hash));

        // 2. Announce
        // We use mainline dht to announce our port on this infohash
        self.dht.announce_peer(info_hash_id, Some(port))?;
        
        Ok(())
    }
}

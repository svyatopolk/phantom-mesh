use sha1::{Sha1, Digest};
use protocol::PeerInfo;

const K_BUCKET_SIZE: usize = 10; // Optimized for Tor

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct NodeId([u8; 20]); // SHA-1 is 20 bytes

impl NodeId {
    pub fn new(onion: &str) -> Self {
        let mut hasher = Sha1::new();
        hasher.update(onion.as_bytes());
        let result = hasher.finalize();
        let mut arr = [0u8; 20];
        arr.copy_from_slice(&result);
        NodeId(arr)
    }

    pub fn distance(&self, other: &NodeId) -> NodeId {
        let mut res = [0u8; 20];
        for i in 0..20 {
            res[i] = self.0[i] ^ other.0[i];
        }
        NodeId(res)
    }
    
    pub fn leading_zeros(&self) -> u32 {
        let mut zeros = 0;
        for byte in self.0 {
            if byte == 0 {
                zeros += 8;
            } else {
                zeros += byte.leading_zeros();
                break;
            }
        }
        zeros
    }
}

pub struct RoutingTable {
    my_id: NodeId,
    // Buckets: Key is distance (leading zeros roughly) or just generic buckets
    // Canonical Kademlia uses buckets based on common prefix length (0..160).
    // buckets[i] contains nodes with common prefix length i.
    buckets: Vec<Vec<PeerInfo>>, 
}

impl RoutingTable {
    pub fn new(my_onion: &str) -> Self {
        Self {
            my_id: NodeId::new(my_onion),
            buckets: vec![Vec::new(); 160], // 0 to 159
        }
    }

    pub fn insert(&mut self, peer: PeerInfo) {
        let other_id = NodeId::new(&peer.onion_address);
        if other_id == self.my_id { return; }
        
        let dist = self.my_id.distance(&other_id);
        let prefix_len = dist.leading_zeros() as usize;
        let bucket_idx = if prefix_len >= 160 { 159 } else { prefix_len };
        
        let bucket = &mut self.buckets[bucket_idx];
        
        // Check if exists
        if let Some(pos) = bucket.iter().position(|p| p.onion_address == peer.onion_address) {
            // Update
            bucket[pos] = peer;
        } else {
            // Add if space
            if bucket.len() < K_BUCKET_SIZE {
                bucket.push(peer);
            } else {
                // If full, in real Kademlia we ping least recently seen.
                // Here we just drop new one (Time-optimized Tor approach per report suggestion of avoiding complexity?)
                // Report says "K=10 optimized". Doesn't specify eviction strictly.
                // We'll replace the oldest (first) just to keep it fresh? 
                // Or standard: prefer old.
                // Let's Keep Old (Standard Kademlia stability preference).
            }
        }
    }
    
    pub fn get_closest_peers(&self, target_onion: &str, count: usize) -> Vec<PeerInfo> {
        let target_id = NodeId::new(target_onion);
        let mut all_peers: Vec<(NodeId, PeerInfo)> = Vec::new();
        
        for bucket in &self.buckets {
            for peer in bucket {
                let pid = NodeId::new(&peer.onion_address);
                all_peers.push((pid, peer.clone()));
            }
        }
        
        // Sort by distance to target
        all_peers.sort_by(|a, b| {
            let da = a.0.distance(&target_id);
            let db = b.0.distance(&target_id);
            da.cmp(&db)
        });
        
        all_peers.into_iter().take(count).map(|(_, p)| p).collect()
    }
    
    pub fn all_peers(&self) -> Vec<PeerInfo> {
        self.buckets.iter().flat_map(|b| b.clone()).collect()
    }
}

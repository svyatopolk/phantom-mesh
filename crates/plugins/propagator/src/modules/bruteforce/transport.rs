use tokio::net::TcpStream;
use tokio::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

pub async fn try_telnet_login(ip: &str, port: u16, user: &str, pass: &str) -> bool {
    let addr = format!("{}:{}", ip, port);
    
    // Connect with timeout
    if let Ok(Ok(mut stream)) = tokio::time::timeout(Duration::from_secs(5), TcpStream::connect(addr)).await {
        let mut buf = [0u8; 1024];

        // 1. Wait for Banner / Login prompt
        if tokio::time::timeout(Duration::from_secs(3), stream.read(&mut buf)).await.is_err() {
            return false;
        }
        
        // 2. Send User
        if stream.write_all(format!("{}\r\n", user).as_bytes()).await.is_err() { return false; }
        tokio::time::sleep(Duration::from_millis(500)).await; // Wait for processing
        
        // 3. Read Password prompt
        if tokio::time::timeout(Duration::from_secs(3), stream.read(&mut buf)).await.is_err() {
            // Some dumb telnets might login immediately if no pass?
            // continue to shell check
        }
        
        // 4. Send Pass
        if stream.write_all(format!("{}\r\n", pass).as_bytes()).await.is_err() { return false; }
        
        // 5. Check Shell
        if let Ok(Ok(n)) = tokio::time::timeout(Duration::from_secs(4), stream.read(&mut buf)).await {
             let response = String::from_utf8_lossy(&buf[..n]);
             // Common shell indicators
             if response.contains('#') || response.contains('$') || response.contains('>') || response.contains('%') {
                 // Check if it's just a "Login Failed" message that happens to have a prompt char
                 if response.to_lowercase().contains("fail") || response.to_lowercase().contains("bad") {
                     return false;
                 }
                 return true;
             }
        }
    }
    false
}

use tokio::net::TcpStream;
use tokio::time::Duration;
use log::debug;

pub async fn check_service(ip: &str, port: u16) -> bool {
    let addr = format!("{}:{}", ip, port);
    debug!("* [Scanner] Checking Banner for {}", addr);
    
    // Very basic check: Can we connect?
    // In future, we can read the banner and check for "Telnet" or specific busybox versions.
    if let Ok(Ok(_)) = tokio::time::timeout(Duration::from_millis(1500), TcpStream::connect(addr)).await {
        return true;
    }
    false
}

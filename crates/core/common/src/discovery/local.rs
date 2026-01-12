use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::time::Duration;
use socket2::{Socket, Domain, Type, Protocol};
use tokio::net::UdpSocket;
use std::sync::Arc;
use tokio::sync::mpsc;

// Multicast Address: 224.0.0.251 (mDNS)
const MC_ADDR: Ipv4Addr = Ipv4Addr::new(224, 0, 0, 251);
const MC_PORT: u16 = 5353;

// Stealth Service Name (Pretend to be a Printer)
const FAKE_SERVICE: &str = "_ipp._tcp.local"; 

#[derive(Clone, Debug)]
pub struct LocalPeer {
    pub peer_id: String,
    pub addr: SocketAddr,
}

pub struct LocalDiscovery {
    rx: mpsc::Receiver<LocalPeer>,
    // Holding the sending socket
    tx_socket: Arc<UdpSocket>, 
}

impl LocalDiscovery {
    pub async fn new(my_peer_id: String, _my_port: u16) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let (tx, rx) = mpsc::channel(32);
        
        let socket = create_multicast_socket(MC_ADDR, MC_PORT)?;
        let udp_socket = UdpSocket::from_std(socket)?;
        let udp_arc = Arc::new(udp_socket);
        
        let listener_socket = udp_arc.clone();
        
        // Spawn Listener Task
        tokio::spawn(async move {
            let mut buf = [0u8; 1024];
            loop {
                if let Ok((len, addr)) = listener_socket.recv_from(&mut buf).await {
                    let data = &buf[..len];
                    
                    // Debug: Log raw packet receive
                    // println!("[Local Debug] Received {} bytes from {}", len, addr);
                    
                    if let Some(peer) = parse_packet(data, addr) {
                        if peer.peer_id == my_peer_id {
                            continue; // Self-discovery
                        }
                        // Debug: confirm packet parsing works
                        println!("[Local Debug] RX peer: {} @ {}", peer.peer_id, peer.addr);
                        let _ = tx.send(peer).await;
                    }
                }
            }
        });
        
        Ok(Self {
            rx,
            tx_socket: udp_arc,
        })
    }

    pub async fn announce(&self, my_peer_id: &str, my_port: u16) {
        let packet = build_packet(my_peer_id, my_port);
        let dest = SocketAddr::new(IpAddr::V4(MC_ADDR), MC_PORT);
        let _ = self.tx_socket.send_to(&packet, dest).await;
    }

    pub async fn next_event(&mut self) -> Option<LocalPeer> {
        self.rx.recv().await
    }
}

// --- Helpers ---

fn create_multicast_socket(addr: Ipv4Addr, port: u16) -> std::io::Result<std::net::UdpSocket> {
    let socket = Socket::new(Domain::IPV4, Type::DGRAM, Some(Protocol::UDP))?;
    
    // Allow reusing the address/port
    #[cfg(unix)]
    {
        socket.set_reuse_port(true)?;
        socket.set_reuse_address(true)?;
    }
    #[cfg(windows)]
    {
         socket.set_reuse_address(true)?;
    }
    
    // Bind
    let bind_addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), port);
    socket.bind(&bind_addr.into())?;
    
    // Set TTL to max to ensure local propagation
    socket.set_multicast_ttl_v4(255)?;
    socket.set_multicast_loop_v4(true)?;
    
    // Join Multicast Group on ALL interfaces
    // This fixes the issue where default route (e.g. Docker/VPN) captures multicast
    let mut joined_count = 0;
    
    if let Ok(ifaces) = local_ip_address::list_afinet_netifas() {
        for (_, ip) in ifaces {
            if let IpAddr::V4(ipv4) = ip {
                if ipv4.is_loopback() { continue; } // Optional: Skip loopback if desired, but harmless to join
                
                match socket.join_multicast_v4(&addr, &ipv4) {
                    Ok(_) => joined_count += 1,
                    Err(e) => eprintln!("[Debug] Failed to join multicast on {}: {}", ipv4, e),
                }
            }
        }
    }
    
    // Fallback: If no interfaces found or joined, try default
    if joined_count == 0 {
        eprintln!("[Debug] Warning: No specific interfaces joined. Using default.");
        socket.join_multicast_v4(&addr, &Ipv4Addr::UNSPECIFIED)?;
    }
    
    Ok(socket.into())
}

const MAGIC_BYTES: u64 = 0xDEADBEEFCAFEBABE;

fn build_packet(peer_id: &str, port: u16) -> Vec<u8> {
    let mut buf = Vec::new();
    
    // Fake Header
    buf.extend_from_slice(FAKE_SERVICE.as_bytes());
    buf.push(0);
    
    // Magic
    buf.extend_from_slice(&MAGIC_BYTES.to_le_bytes());
    
    // Payload
    let p_bytes = peer_id.as_bytes();
    buf.push(p_bytes.len() as u8);
    buf.extend_from_slice(p_bytes);
    
    buf.extend_from_slice(&port.to_le_bytes());
    
    buf
}

fn parse_packet(data: &[u8], src: SocketAddr) -> Option<LocalPeer> {
    let prefix = FAKE_SERVICE.as_bytes();
    if data.len() < prefix.len() + 1 + 8 + 1 + 2 {
        return None; 
    }
    
    if !data.starts_with(prefix) {
        return None;
    }
    
    let offset = prefix.len() + 1;
    
    let magic_slice = &data[offset..offset+8];
    let magic = u64::from_le_bytes(magic_slice.try_into().ok()?);
    if magic != MAGIC_BYTES {
        return None;
    }
    
    let len_idx = offset + 8;
    let p_len = data[len_idx] as usize;
    let p_start = len_idx + 1;
    let p_end = p_start + p_len;
    
    if data.len() < p_end + 2 {
        return None;
    }
    
    let peer_id = String::from_utf8_lossy(&data[p_start..p_end]).to_string();
    
    let port_slice = &data[p_end..p_end+2];
    let port = u16::from_le_bytes(port_slice.try_into().ok()?);
    
    let dest_ip = src.ip();
    let dest_addr = SocketAddr::new(dest_ip, port);
    
    Some(LocalPeer {
        peer_id,
        addr: dest_addr,
    })
}

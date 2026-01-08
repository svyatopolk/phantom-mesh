use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::time::Instant;
use quinn::{Endpoint, ClientConfig, Connection};
use std::net::SocketAddr;
use protocol::quic::PhantomFrame;
use rustls::client::danger::{ServerCertVerifier, ServerCertVerified};
use rustls::pki_types::{CertificateDer, ServerName, UnixTime};

const MAX_POOL_SIZE: usize = 50;

pub struct QuicPool {
    endpoint: Endpoint,
    connections: HashMap<String, Arc<Connection>>,
    last_activity: HashMap<String, Instant>,
}

impl QuicPool {
    pub fn new(endpoint: Endpoint) -> Self {
        Self {
            endpoint,
            connections: HashMap::new(),
            last_activity: HashMap::new(),
        }
    }

    pub async fn send_msg(
        &mut self, 
        addr_str: &str, 
        msg_payload: Vec<u8>,
        stream_id: u32,
        signature: [u8; 64],
        protected_peers: &[String]
    ) -> Result<(), String> {
        
        self.cleanup();

        let conn = if let Some(c) = self.connections.get(addr_str) {
             c.clone()
        } else {
             if self.connections.len() >= MAX_POOL_SIZE {
                 self.evict_one(protected_peers);
             }
             
             let addr: SocketAddr = addr_str.parse().map_err(|e| format!("Bad Addr: {}", e))?;
             
             let connect = self.endpoint.connect(addr, "www.google.com")
                 .map_err(|e| format!("Connect Config Error: {}", e))?;
                 
             match connect.await {
                 Ok(c) => {
                     let c_arc = Arc::new(c);
                     self.connections.insert(addr_str.to_string(), c_arc.clone());
                     c_arc
                 },
                 Err(e) => return Err(format!("QUIC Connect Failed: {}", e)),
             }
        };

        match conn.open_uni().await {
            Ok(mut send_stream) => {
                // Encrypt Payload using ChaCha20Poly1305
                let ciphertext = protocol::crypto::encrypt_payload(&msg_payload).map_err(|e| format!("Encrypt: {}", e))?;


                let frame = PhantomFrame::new(stream_id, ciphertext, signature);
                let bytes = frame.to_bytes();
                
                if let Err(e) = send_stream.write_all(&bytes).await {
                     return Err(format!("Stream Write Failed: {}", e));
                }
                if let Err(e) = send_stream.finish() {
                     return Err(format!("Stream Finish Failed: {}", e));
                }
                
                self.last_activity.insert(addr_str.to_string(), Instant::now());
                Ok(())
            },
            Err(e) => {
                self.connections.remove(addr_str);
                Err(format!("Open Stream Failed: {}", e))
            }
        }
    }

    fn cleanup(&mut self) {
        let dead: Vec<String> = self.connections.iter()
            .filter(|(_, c)| c.close_reason().is_some())
            .map(|(k, _)| k.clone())
            .collect();
        for k in dead { self.connections.remove(&k); }
    }

    fn evict_one(&mut self, protected: &[String]) {
        let mut oldest_key = String::new();
        let mut oldest_time = Instant::now() + std::time::Duration::from_secs(3600);
        
        for (k, v) in &self.last_activity {
            if protected.contains(k) { continue; }
            if *v < oldest_time {
                 oldest_time = *v;
                 oldest_key = k.clone();
            }
        }
        
        if !oldest_key.is_empty() {
             self.connections.remove(&oldest_key);
             self.last_activity.remove(&oldest_key);
        }
    }
}

pub fn make_client_config() -> ClientConfig {
    #[derive(Debug)]
    struct SkipServerVerification;
    impl ServerCertVerifier for SkipServerVerification {
        fn verify_server_cert(
            &self,
            _end_entity: &CertificateDer,
            _intermediates: &[CertificateDer],
            _server_name: &ServerName,
            _ocsp_response: &[u8],
            _now: UnixTime,
        ) -> Result<ServerCertVerified, rustls::Error> {
            Ok(ServerCertVerified::assertion())
        }

        fn verify_tls12_signature(
            &self,
            _message: &[u8],
            _cert: &CertificateDer<'_>,
            _dss: &rustls::DigitallySignedStruct,
        ) -> Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
            Ok(rustls::client::danger::HandshakeSignatureValid::assertion())
        }

        fn verify_tls13_signature(
            &self,
            _message: &[u8],
            _cert: &CertificateDer<'_>,
            _dss: &rustls::DigitallySignedStruct,
        ) -> Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
            Ok(rustls::client::danger::HandshakeSignatureValid::assertion())
        }

        fn supported_verify_schemes(&self) -> Vec<rustls::SignatureScheme> {
            vec![
                rustls::SignatureScheme::RSA_PKCS1_SHA1,
                rustls::SignatureScheme::ECDSA_NISTP256_SHA256,
                rustls::SignatureScheme::RSA_PSS_SHA256,
                rustls::SignatureScheme::ED25519,
            ]
        }
    }

    let crypto = rustls::ClientConfig::builder()
        .dangerous()
        .with_custom_certificate_verifier(Arc::new(SkipServerVerification))
        .with_no_client_auth();
    
    // Wrap for Quinn compatibility
    let quic_crypto = quinn::crypto::rustls::QuicClientConfig::try_from(crypto).unwrap();
    ClientConfig::new(Arc::new(quic_crypto))
}

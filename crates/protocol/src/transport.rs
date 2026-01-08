use quinn::{ClientConfig, Endpoint, ServerConfig, TransportConfig};
use rustls::client::danger::{ServerCertVerifier, ServerCertVerified};
use rustls::pki_types::{CertificateDer, PrivateKeyDer, PrivatePkcs8KeyDer, ServerName, UnixTime};
use std::sync::Arc;
use std::error::Error;
use std::net::SocketAddr;
use std::time::Duration;

pub struct PhantomTransport {
    pub endpoint: Endpoint,
}

impl PhantomTransport {
    pub fn new_client() -> Result<Self, Box<dyn Error>> {
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

        // Wrap struct for Quinn compatibility
        let quic_crypto = quinn::crypto::rustls::QuicClientConfig::try_from(crypto)?;
        let mut client_config = ClientConfig::new(Arc::new(quic_crypto));

        let mut transport_config = TransportConfig::default();
        transport_config.keep_alive_interval(Some(Duration::from_secs(10)));
        transport_config.max_idle_timeout(Some(Duration::from_secs(30).try_into().unwrap()));
        
        client_config.transport_config(Arc::new(transport_config));

        let mut endpoint = Endpoint::client("0.0.0.0:0".parse().unwrap())?;
        endpoint.set_default_client_config(client_config);

        Ok(Self { endpoint })
    }

    pub fn new_server(port: u16) -> Result<Self, Box<dyn Error>> {
        let cert = rcgen::generate_simple_self_signed(vec!["www.google.com".to_string()])?;
        let cert_der = cert.cert.der().to_vec();
        // Fixed: signing_key
        let key_der = cert.signing_key.serialize_der();
        
        let cert_chain = vec![CertificateDer::from(cert_der)];
        let priv_key = PrivateKeyDer::Pkcs8(PrivatePkcs8KeyDer::from(key_der));
        
        let server_config = ServerConfig::with_single_cert(cert_chain, priv_key)?;
        
        let mut transport_config = TransportConfig::default();
        transport_config.keep_alive_interval(Some(Duration::from_secs(10)));
        transport_config.max_idle_timeout(Some(Duration::from_secs(30).try_into().unwrap()));
        
        let addr = SocketAddr::from(([0, 0, 0, 0], port));
        let endpoint = Endpoint::server(server_config, addr)?;

        Ok(Self { endpoint })
    }
}

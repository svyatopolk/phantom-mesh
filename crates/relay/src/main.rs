use std::env;
use std::io::Error;
use tokio::net::TcpListener;

mod server;
mod state;

#[tokio::main]
async fn main() -> Result<(), Error> {
    env_logger::init();
    let addr = env::args().nth(1).unwrap_or_else(|| "0.0.0.0:8080".to_string());

    let state = state::init_state();

    let listener = TcpListener::bind(&addr).await.expect("Failed to bind");
    println!("RELAY (Rendezvous) listening on: {}", addr);

    while let Ok((stream, addr)) = listener.accept().await {
        tokio::spawn(server::handle_connection(state.clone(), stream, addr));
    }

    Ok(())
}

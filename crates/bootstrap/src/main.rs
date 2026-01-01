mod server;

#[tokio::main]
async fn main() {
    let port = std::env::var("PORT").unwrap_or("8080".to_string()).parse().unwrap();
    server::run_bootstrap_node(port).await;
}

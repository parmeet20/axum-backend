use axum::{extract::Path, routing::get, Router};
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::pubkey::Pubkey;
use std::str::FromStr;

#[tokio::main]
async fn main() {
    let app = Router::new().route("/balance/{publickey}", get(get_balance_handler));

    let listener = tokio::net::TcpListener::bind("127.0.0.1:8080")
        .await
        .unwrap();

    axum::serve(listener, app).await.unwrap();
}

async fn get_balance_handler(Path(publickey): Path<String>) -> String {
    let client = RpcClient::new("https://api.devnet.solana.com".to_string());
    let key = match Pubkey::from_str(&publickey) {
        Ok(key) => key,
        Err(_) => return "Invalid public key".to_string(),
    };
    match client.get_balance(&key).await {
        Ok(balance) => format!("balance is: {} SOL", balance as f64 / 1_000_000_000.0),
        Err(e) => format!("Error: {}", e),
    }
}
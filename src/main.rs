use axum::{Json, Router, extract::Path, routing::get};
use serde::{Deserialize, Serialize};
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::pubkey::Pubkey;
use std::str::FromStr;

#[tokio::main]
async fn main() {
    let app = Router::new().route("/balance/{publickey}", get(get_balance_handler));

    let listener = tokio::net::TcpListener::bind("0.0.0.0:8080").await.unwrap();

    axum::serve(listener, app).await.unwrap();
}

#[derive(Serialize, Deserialize)]
struct BalanceResp {
    balance: f64,
    success: bool,
    message: String,
}

async fn get_balance_handler(Path(publickey): Path<String>) -> Json<BalanceResp> {
    let client = RpcClient::new("https://api.devnet.solana.com".to_string());
    let key = match Pubkey::from_str(&publickey) {
        Ok(key) => key,
        Err(e) => {
            return Json::from(BalanceResp {
                balance: 0.0,
                success: false,
                message: format!("Invalid public key: {}", e),
            });
        }
    };
    match client.get_balance(&key).await {
        Ok(balance) => Json::from(BalanceResp {
            balance: balance as f64 / 1_000_000_000.0,
            success: true,
            message: format!("success"),
        }),
        Err(e) => Json::from(BalanceResp {
            balance: 0.0,
            success: false,
            message: format!("Error fetching balance: {}", e),
        }),
    }
}

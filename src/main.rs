use axum::{routing::post, Json, Router};
use reqwest;
use serde::{Deserialize, Serialize};
use tokio::net::TcpListener;
use tower_http::cors::{Any, CorsLayer};

#[derive(Deserialize)]
struct GoogleAuthRequest {
    id_token: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct GoogleTokenInfo {
    email: String,
    name: String,
    picture: String,
    sub: String,
}

async fn receive_token(Json(payload): Json<GoogleAuthRequest>) -> String {
    let verification_url = format!(
        "https://oauth2.googleapis.com/tokeninfo?id_token={}",
        payload.id_token
    );

    match reqwest::get(&verification_url).await {
        Ok(response) => {
            if response.status().is_success() {
                match response.json::<GoogleTokenInfo>().await {
                    Ok(user_info) => {
                        println!("Token Verified! User: {:?}", user_info);
                        format!("Valid Token! Welcome, {}!", user_info.name)
                    }
                    Err(_) => "Invalid JSON".to_string(),
                }
            } else {
                "Invalid Token".to_string()
            }
        }
        Err(_) => "Failed to verify token".to_string(),
    }
}

#[tokio::main]
async fn main() {
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let app = Router::new()
        .route("/auth/google", post(receive_token))
        .layer(cors);

    println!("Listening on port 3000...");
    let listener = TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

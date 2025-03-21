use axum::{
    Json, Router,
    extract::State,
    http::HeaderValue,
    routing::{get, post},
};
use http::Method;
use reqwest;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use sqlx::postgres::PgPoolOptions;
use std::env;
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

#[derive(Clone)]
struct AppState {
    db_pool: PgPool,
}

async fn hello_world() -> &'static str {
    "Hello, World!"
}

async fn receive_token(
    State(state): State<AppState>,
    Json(payload): Json<GoogleAuthRequest>,
) -> String {
    let verification_url = format!(
        "https://oauth2.googleapis.com/tokeninfo?id_token={}",
        payload.id_token
    );

    match reqwest::get(&verification_url).await {
        Ok(response) => {
            if response.status().is_success() {
                match response.json::<GoogleTokenInfo>().await {
                    Ok(user_info) => {
                        let insert_result = sqlx::query(
                            "INSERT INTO users (email, name, picture, sub) VALUES ($1, $2, $3, $4)",
                        )
                        .bind(&user_info.email)
                        .bind(&user_info.name)
                        .bind(&user_info.picture)
                        .bind(&user_info.sub)
                        .execute(&state.db_pool)
                        .await;

                        match insert_result {
                            Ok(result) => {
                                println!("Token verified and inserted user: {:?}", user_info);
                                format!(
                                    "Valid Token! Welcome, {}! (Inserted {} row(s))",
                                    user_info.name,
                                    result.rows_affected()
                                )
                            }
                            Err(e) => {
                                println!("Error inserting into database: {:?}", e);
                                "Token verified but failed to save email.".to_string()
                            }
                        }
                    }
                    Err(e) => {
                        println!("Error parsing token info: {:?}", e);
                        "Invalid JSON received from token verification.".to_string()
                    }
                }
            } else {
                "Invalid Token".to_string()
            }
        }
        Err(e) => {
            println!("Error verifying token: {:?}", e);
            "Failed to verify token.".to_string()
        }
    }
}

#[tokio::main]
async fn main() {
    dotenv::dotenv().ok();

    let cors = CorsLayer::new()
        .allow_origin([
            "https://proud-adaptation-staging.up.railway.app"
                .parse::<HeaderValue>()
                .unwrap(),
            "http://localhost:5173".parse::<HeaderValue>().unwrap(),
        ])
        .allow_methods([Method::POST, Method::GET])
        .allow_headers(Any);

    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await
        .expect("Failed to create pool");

    let app_state = AppState { db_pool: pool };

    let app = Router::new()
        .route("/hello", get(hello_world))
        .route("/auth/google", post(receive_token))
        .with_state(app_state)
        .layer(cors);

    println!("Listening on port 12345...");
    let listener = TcpListener::bind("0.0.0.0:12345").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

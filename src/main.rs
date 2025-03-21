use axum::{
    Json, Router,
    extract::State,
    http::HeaderValue,
    http::Method,
    routing::{get, post},
};
use serde::{Deserialize, Serialize};
use sqlx::{PgPool, postgres::PgPoolOptions};
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
        Ok(response) if response.status().is_success() => {
            match response.json::<GoogleTokenInfo>().await {
                Ok(user_info) => handle_user_info(user_info, state).await,
                Err(e) => log_error("Token parsing", e),
            }
        }
        Ok(_) => "Invalid Token".to_string(),
        Err(e) => log_error("Token verification", e),
    }
}

async fn handle_user_info(user_info: GoogleTokenInfo, state: AppState) -> String {
    match sqlx::query("INSERT INTO users (email, name, picture, sub) VALUES ($1, $2, $3, $4)")
        .bind(&user_info.email)
        .bind(&user_info.name)
        .bind(&user_info.picture)
        .bind(&user_info.sub)
        .execute(&state.db_pool)
        .await
    {
        Ok(result) => format!(
            "Welcome {}! (Inserted {} row(s))",
            user_info.name,
            result.rows_affected()
        ),
        Err(e) => {
            eprintln!("Database error: {:?}", e);
            "Failed to save user".to_string()
        }
    }
}

fn log_error(context: &str, error: impl std::fmt::Debug) -> String {
    eprintln!("{} error: {:?}", context, error);
    "Authentication error".to_string()
}

#[tokio::main]
async fn main() {
    dotenv::dotenv().ok();

    let frontend_url = env::var("FRONTEND_URL").unwrap_or_else(|_| "http://localhost:5173".into());
    let port = env::var("PORT").unwrap_or_else(|_| "12345".into());
    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");

    let cors = CorsLayer::new()
        .allow_origin(frontend_url.parse::<HeaderValue>().unwrap())
        .allow_methods([Method::POST, Method::GET])
        .allow_headers(Any);

    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await
        .expect("Failed to create pool");

    let app = Router::new()
        .route("/hello", get(hello_world))
        .route("/auth/google", post(receive_token))
        .with_state(AppState { db_pool: pool })
        .layer(cors);

    let addr = format!("0.0.0.0:{}", port);
    println!("Server starting on {}", addr);
    let listener = TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

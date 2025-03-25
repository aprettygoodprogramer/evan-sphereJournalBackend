mod handlers;
mod models;
use chrono::Duration as ChronoDuration;

use axum::{
    Router,
    http::{HeaderValue, Method},
    routing::{get, post},
};
use handlers::{hello_world, receive_token};
use models::AppState;
use sqlx::{PgPool, postgres::PgPoolOptions};
use std::env;
use tokio::net::TcpListener;
use tower_http::cors::{Any, CorsLayer};

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
    let app_state: AppState = AppState {
        db_pool: pool,
        session_ttl: ChronoDuration::days(7),
    };
    let app = Router::new()
        .route("/hello", get(hello_world))
        .route("/auth/google", post(receive_token))
        .with_state(app_state)
        .layer(cors);

    let addr = format!("0.0.0.0:{}", port);
    println!("Server starting on {}", addr);
    let listener = TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

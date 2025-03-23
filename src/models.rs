use serde::{Deserialize, Serialize};
use sqlx::PgPool;

#[derive(Deserialize)]
pub struct GoogleAuthRequest {
    pub id_token: String,
}

#[derive(Deserialize, Serialize)]
pub struct AuthResponse {
    pub success: bool,
    pub message: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct GoogleTokenInfo {
    pub email: String,
    pub name: String,
    pub picture: String,
    pub sub: String,
}

#[derive(Clone)]
pub struct AppState {
    pub db_pool: PgPool,
}

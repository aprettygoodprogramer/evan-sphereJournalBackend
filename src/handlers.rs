use axum::{Json, extract::State};
use reqwest;
use serde::{Deserialize, Serialize};
use std::fmt;

use crate::models::{AppState, AuthResponse, GoogleAuthRequest, GoogleTokenInfo};

pub async fn hello_world() -> &'static str {
    "Hello, World!"
}

pub async fn receive_token(
    State(state): State<AppState>,
    Json(payload): Json<GoogleAuthRequest>,
) -> Json<AuthResponse> {
    let verification_url = format!(
        "https://oauth2.googleapis.com/tokeninfo?id_token={}",
        payload.id_token
    );

    match reqwest::get(&verification_url).await {
        Ok(response) if response.status().is_success() => {
            match response.json::<GoogleTokenInfo>().await {
                Ok(user_info) => handle_user_info(user_info, state).await,
                Err(e) => {
                    let message = log_error("Token parsing", e);
                    Json(AuthResponse {
                        success: false,
                        message,
                    })
                }
            }
        }
        Ok(_) => Json(AuthResponse {
            success: false,
            message: "Invalid Token".to_string(),
        }),
        Err(e) => {
            let message = log_error("Token verification", e);
            Json(AuthResponse {
                success: false,
                message,
            })
        }
    }
}

async fn handle_user_info(user_info: GoogleTokenInfo, state: AppState) -> Json<AuthResponse> {
    match sqlx::query("INSERT INTO users (email, name, picture, sub) VALUES ($1, $2, $3, $4)")
        .bind(&user_info.email)
        .bind(&user_info.name)
        .bind(&user_info.picture)
        .bind(&user_info.sub)
        .execute(&state.db_pool)
        .await
    {
        Ok(result) => Json(AuthResponse {
            success: true,
            message: format!(
                "Welcome Big Boi!! {}! (Inserted {} row(s))",
                user_info.name,
                result.rows_affected()
            ),
        }),
        Err(e) => {
            eprintln!("Database error: {:?}", e);
            Json(AuthResponse {
                success: false,
                message: "Failed to save user".to_string(),
            })
        }
    }
}

fn log_error(context: &str, error: impl fmt::Debug) -> String {
    eprintln!("{} error: {:?}", context, error);
    "Authentication error".to_string()
}

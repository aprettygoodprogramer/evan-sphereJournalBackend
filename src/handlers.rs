use crate::models::{
    AppState, AuthResponse, GoogleAuthRequest, GoogleTokenInfo, VerifyResponse, Verify_Session_Request
};
use axum::{Json, extract::State};
use chrono::Utc;
use reqwest;
use serde::{Deserialize, Serialize};
use sqlx::types::chrono::DateTime;
use std::fmt;
use uuid::Uuid;
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
                        session_id: Uuid::new_v4(),
                    })
                }
            }
        }
        Ok(_) => Json(AuthResponse {
            success: false,
            message: "Invalid Token".to_string(),
            session_id: Uuid::new_v4(),
        }),
        Err(e) => {
            let message = log_error("Token verification", e);
            Json(AuthResponse {
                success: false,
                message,
                session_id: Uuid::new_v4(),
            })
        }
    }
}

async fn handle_user_info(user_info: GoogleTokenInfo, state: AppState) -> Json<AuthResponse> {
    let userSub = user_info.sub.to_string();
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
            session_id: create_session(state, userSub).await,
        }),
        Err(e) => {
            eprintln!("Database error: {:?}", e);
            Json(AuthResponse {
                success: false,
                message: "Failed to save user".to_string(),
                session_id: Uuid::new_v4(),
            })
        }
    }
}

fn log_error(context: &str, error: impl fmt::Debug) -> String {
    eprintln!("{} error: {:?}", context, error);
    "Authentication error".to_string()
}

async fn create_session(state: AppState, sub: String) -> Uuid {
    let session_id: Uuid = Uuid::new_v4();
    let expires_at = Utc::now() + state.session_ttl;
    match sqlx::query("INSERT INTO sessions (session_id, user_id, expires_at) VALUES ($1, $2, $3)")
        .bind(&session_id)
        .bind(&sub)
        .bind(&expires_at)
        .execute(&state.db_pool)
        .await
    {
        Ok(result) => session_id,
        Err(e) => {
            eprintln!("Database error: {:?}", e);
            session_id
        }
    }
}
pub async fn verify_session(
    State(state): State<AppState>,
    Json(payload): Json<Verify_Session_Request>,
) -> Json<VerifyResponse> {
    match sqlx::query_as::<_, (Uuid, DateTime<Utc>)>(
        "SELECT session_id, expires_at FROM sessions WHERE session_id = $1 AND user_id = $2",
    )
    .bind(&payload.session_id)
    .bind(&payload.sub)
    .fetch_one(&state.db_pool)
    .await
    {
        Ok((session_id, expires_at)) if expires_at > Utc::now() => Json(VerifyResponse {
            success: true,
        }),
        Ok(_) => Json(VerifyResponse {
            success: false,
        }),
        Err(e) => {
            eprintln!("Database error: {:?}", e);
            Json(VerifyResponse {
                success: false,
            })
        }
    }
}
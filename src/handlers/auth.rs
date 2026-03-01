use crate::dto::{LoginRequest, SignupRequest};
use crate::error::AppError;
use crate::server::AppState;
use crate::services::auth_service;
use axum::{extract::State, http::StatusCode, routing::post, Json, Router};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/signup", post(signup))
        .route("/login", post(login))
}

/// POST /signup - Register a new user
async fn signup(
    State(state): State<AppState>,
    Json(request): Json<SignupRequest>,
) -> Result<(StatusCode, Json<serde_json::Value>), AppError> {
    let auth_data = auth_service::signup(&state.db, request).await?;

    Ok((
        StatusCode::CREATED,
        Json(serde_json::json!({
            "success": true,
            "data": auth_data
        })),
    ))
}

/// POST /login - Authenticate a user
async fn login(
    State(state): State<AppState>,
    Json(request): Json<LoginRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    let auth_data = auth_service::login(&state.db, request).await?;

    Ok(Json(serde_json::json!({
        "success": true,
        "data": auth_data
    })))
}

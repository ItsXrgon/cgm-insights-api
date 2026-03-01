use crate::dto::{AuthDataResponse, LoginRequest, SignupRequest};
use crate::error::AppError;
use crate::server::AppState;
use crate::services::auth_service;
use axum::{extract::State, http::StatusCode, routing::post, Json, Router};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/signup", post(signup))
        .route("/login", post(login))
}

/// Register a new user and create CGM credential
#[utoipa::path(
    post,
    path = "/api/auth/signup",
    tag = "Auth",
    request_body = SignupRequest,
    responses(
        (status = 201, description = "User created", body = AuthDataResponse),
        (status = 400, description = "Bad request", body = crate::dto::ErrorResponse),
    ),
)]
pub async fn signup(
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

/// Authenticate a user and return JWT
#[utoipa::path(
    post,
    path = "/api/auth/login",
    tag = "Auth",
    request_body = LoginRequest,
    responses(
        (status = 200, description = "Login success", body = AuthDataResponse),
        (status = 401, description = "Invalid credentials", body = crate::dto::ErrorResponse),
    ),
)]
pub async fn login(
    State(state): State<AppState>,
    Json(request): Json<LoginRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    let auth_data = auth_service::login(&state.db, request).await?;

    Ok(Json(serde_json::json!({
        "success": true,
        "data": auth_data
    })))
}

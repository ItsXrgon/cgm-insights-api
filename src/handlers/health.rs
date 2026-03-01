use crate::{dto::HealthResponse, server::AppState};
use axum::{routing::get, Json, Router};

pub fn routes() -> Router<AppState> {
    Router::new().route("/health", get(health_check))
}

/// Health check endpoint
#[utoipa::path(
    get,
    path = "/health",
    tag = "Health",
    responses((status = 200, description = "Service is healthy", body = HealthResponse)),
)]
pub async fn health_check() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "OK",
        timestamp: chrono::Utc::now().timestamp(),
    })
}

use crate::{dto::HealthResponse, server::AppState};
use axum::{routing::get, Json, Router};

pub fn routes() -> Router<AppState> {
    Router::new().route("/health", get(health_check))
}

async fn health_check() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "OK",
        timestamp: chrono::Utc::now().timestamp(),
    })
}

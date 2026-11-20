use crate::{dto::HealthResponse, server::AppState};
use axum::{extract::State, http::StatusCode, routing::get, Json, Router};
use std::time::Duration;

pub fn routes() -> Router<AppState> {
    Router::new().route("/health", get(health_check))
}

/// Health check: 200 when the database answers within 2s, otherwise 503
#[utoipa::path(
    get,
    path = "/health",
    tag = "Health",
    responses(
        (status = 200, description = "Service and database are up", body = HealthResponse),
        (status = 503, description = "Database unreachable", body = HealthResponse),
    ),
)]
pub async fn health_check(State(state): State<AppState>) -> (StatusCode, Json<HealthResponse>) {
    let db_up = tokio::time::timeout(
        Duration::from_secs(2),
        sqlx::query("SELECT 1").execute(state.db.as_ref()),
    )
    .await
    .is_ok_and(|r| r.is_ok());

    let (status, body_status, database) = if db_up {
        (StatusCode::OK, "OK", "up")
    } else {
        (StatusCode::SERVICE_UNAVAILABLE, "UNAVAILABLE", "down")
    };
    (
        status,
        Json(HealthResponse {
            status: body_status,
            database,
            timestamp: chrono::Utc::now().timestamp(),
        }),
    )
}

use crate::error::AppError;
use crate::repositories::cgm_repository;
use crate::server::AppState;
use crate::services::auth_service::Claims;
use axum::{extract::State, http::StatusCode, routing::post, Extension, Json, Router};

pub fn routes() -> Router<AppState> {
    Router::new().route("/sync", post(trigger_sync))
}

/// Manually trigger a sync for the active CGM credential
#[utoipa::path(
    post,
    path = "/api/sync",
    tag = "Sync",
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Sync completed"),
        (status = 401, description = "Unauthorized", body = crate::dto::ErrorResponse),
        (status = 500, description = "No active credential or sync failed", body = crate::dto::ErrorResponse),
    ),
)]
pub async fn trigger_sync(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
) -> Result<(StatusCode, Json<serde_json::Value>), AppError> {
    println!("🔄 Manual sync triggered for user {}...", claims.sub);

    // Find active credential for the user
    let creds = cgm_repository::find_by_user_id(&state.db, claims.sub).await?;
    let active_cred = creds.into_iter().find(|c| c.is_active).ok_or_else(|| {
        AppError::ConfigError("No active CGM credential found. Please add one first.".to_string())
    })?;

    let count = state.sync_service.sync_for_credential(&active_cred).await?;

    Ok((
        StatusCode::OK,
        Json(serde_json::json!({
            "success": true,
            "message": format!("Successfully synced {} readings for {}", count, active_cred.cgm_type),
            "readings_synced": count,
            "note": "Background scheduler runs every 5 minutes automatically"
        })),
    ))
}

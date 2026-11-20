use crate::dto::SyncResponse;
use crate::error::AppError;
use crate::repositories::cgm_repository;
use crate::server::AppState;
use crate::services::{auth_service::Claims, SyncOutcome};
use axum::{extract::State, routing::post, Extension, Json, Router};

pub fn routes() -> Router<AppState> {
    Router::new().route("/sync", post(trigger_sync))
}

/// Sync the active CGM credential now (at most once per cooldown, default 60s)
#[utoipa::path(
    post,
    path = "/api/sync",
    tag = "Sync",
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Sync completed", body = SyncResponse),
        (status = 401, description = "Unauthorized", body = crate::dto::ErrorResponse),
        (status = 404, description = "No active credential", body = crate::dto::ErrorResponse),
        (status = 422, description = "CGM platform rejected the credentials", body = crate::dto::ErrorResponse),
        (status = 429, description = "Synced too recently, or CGM account locked", body = crate::dto::ErrorResponse),
        (status = 502, description = "CGM platform unavailable", body = crate::dto::ErrorResponse),
    ),
)]
pub async fn trigger_sync(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
) -> Result<Json<SyncResponse>, AppError> {
    tracing::info!(user_id = claims.sub, "Manual sync triggered");

    let cred = cgm_repository::find_active_for_user(&state.db, claims.sub)
        .await?
        .ok_or_else(|| {
            AppError::NotFound("No active CGM credential found. Please add one first.".to_string())
        })?;

    match state.sync_service.sync_credential(&cred).await? {
        SyncOutcome::Synced(count) => Ok(Json(SyncResponse {
            success: true,
            message: format!(
                "Successfully synced {} readings for {}",
                count, cred.cgm_type
            ),
            readings_synced: count,
        })),
        SyncOutcome::Skipped => Err(AppError::RateLimited(format!(
            "A sync ran less than {} seconds ago; try again shortly",
            state.config.sync_cooldown_secs
        ))),
    }
}

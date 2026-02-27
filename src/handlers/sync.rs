use crate::error::AppError;
use crate::server::AppState;
use axum::{extract::State, http::StatusCode, routing::post, Json, Router};

pub fn routes() -> Router<AppState> {
    Router::new().route("/sync", post(trigger_sync))
}

/// POST /sync - Manually trigger a sync with LibreLink Up API
async fn trigger_sync(
    State(_state): State<AppState>,
) -> Result<(StatusCode, Json<serde_json::Value>), AppError> {
    println!("🔄 Manual sync triggered via API...");

    // PLACEHOLDER: In production, this would call the sync service
    // For now, just return a success message

    Ok((
        StatusCode::OK,
        Json(serde_json::json!({
            "success": true,
            "message": "Sync triggered (placeholder - will fetch from LibreLink Up API)",
            "note": "Background scheduler runs every 1 hour automatically"
        })),
    ))
}

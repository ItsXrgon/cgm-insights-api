use crate::dto::CreateGlucoseReadingRequest;
use crate::error::AppError;
use crate::server::AppState;
use crate::services::auth_service::Claims;
use crate::services::glucose_service;
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    routing::{delete, get, post},
    Extension, Json, Router,
};
use serde::Deserialize;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/glucose", post(create_reading))
        .route("/glucose", get(get_readings))
        .route("/glucose/{id}", get(get_reading))
        .route("/glucose/{id}", delete(delete_reading))
}

#[derive(Debug, Deserialize)]
struct ListQuery {
    limit: Option<i64>,
}

/// POST /glucose - Create a new glucose reading
async fn create_reading(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Json(request): Json<CreateGlucoseReadingRequest>,
) -> Result<(StatusCode, Json<serde_json::Value>), AppError> {
    let reading = glucose_service::create_reading(&state.db, claims.sub, request).await?;

    Ok((
        StatusCode::CREATED,
        Json(serde_json::json!({
            "success": true,
            "data": reading
        })),
    ))
}

/// GET /glucose - Get all glucose readings for the current user
async fn get_readings(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Query(query): Query<ListQuery>,
) -> Result<Json<serde_json::Value>, AppError> {
    let readings = glucose_service::get_readings(&state.db, claims.sub, query.limit).await?;

    Ok(Json(serde_json::json!({
        "success": true,
        "data": readings,
        "count": readings.len()
    })))
}

/// GET /glucose/:id - Get a specific glucose reading
async fn get_reading(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<i32>,
) -> Result<Json<serde_json::Value>, AppError> {
    let reading = glucose_service::get_reading_by_id(&state.db, claims.sub, id).await?;

    Ok(Json(serde_json::json!({
        "success": true,
        "data": reading
    })))
}

/// DELETE /glucose/:id - Delete a glucose reading
async fn delete_reading(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<i32>,
) -> Result<(StatusCode, Json<serde_json::Value>), AppError> {
    let deleted = glucose_service::delete_reading(&state.db, claims.sub, id).await?;

    if deleted {
        Ok((
            StatusCode::OK,
            Json(serde_json::json!({
                "success": true,
                "message": "Reading deleted successfully"
            })),
        ))
    } else {
        Ok((
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({
                "success": false,
                "message": "Reading not found"
            })),
        ))
    }
}

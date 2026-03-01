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
use utoipa::{IntoParams, ToSchema};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/glucose", post(create_reading))
        .route("/glucose", get(get_readings))
        .route("/glucose/{id}", get(get_reading))
        .route("/glucose/{id}", delete(delete_reading))
}

#[derive(Debug, Deserialize, IntoParams, ToSchema)]
pub struct ListQuery {
    pub limit: Option<i64>,
}

/// Create a new glucose reading
#[utoipa::path(
    post,
    path = "/api/glucose",
    tag = "Glucose",
    security(("bearer_auth" = [])),
    request_body = CreateGlucoseReadingRequest,
    responses(
        (status = 201, description = "Reading created"),
        (status = 401, description = "Unauthorized", body = crate::dto::ErrorResponse),
    ),
)]
pub async fn create_reading(
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

/// Get all glucose readings for the current user
#[utoipa::path(
    get,
    path = "/api/glucose",
    tag = "Glucose",
    security(("bearer_auth" = [])),
    params(ListQuery),
    responses(
        (status = 200, description = "List of readings"),
        (status = 401, description = "Unauthorized", body = crate::dto::ErrorResponse),
    ),
)]
pub async fn get_readings(
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

/// Get a specific glucose reading by ID
#[utoipa::path(
    get,
    path = "/api/glucose/{id}",
    tag = "Glucose",
    security(("bearer_auth" = [])),
    params(("id" = i32, Path, description = "Reading ID")),
    responses(
        (status = 200, description = "Reading found"),
        (status = 401, description = "Unauthorized", body = crate::dto::ErrorResponse),
        (status = 404, description = "Not found", body = crate::dto::ErrorResponse),
    ),
)]
pub async fn get_reading(
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

/// Delete a glucose reading
#[utoipa::path(
    delete,
    path = "/api/glucose/{id}",
    tag = "Glucose",
    security(("bearer_auth" = [])),
    params(("id" = i32, Path, description = "Reading ID")),
    responses(
        (status = 200, description = "Reading deleted"),
        (status = 401, description = "Unauthorized", body = crate::dto::ErrorResponse),
        (status = 404, description = "Not found", body = crate::dto::ErrorResponse),
    ),
)]
pub async fn delete_reading(
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

use crate::dto::{
    ApiMessageResponse, CreateGlucoseReadingRequest, DataResponse, GlucoseListQuery,
    GlucoseListResponse, GlucoseReadingResponse,
};
use crate::error::AppError;
use crate::server::AppState;
use crate::services::auth_service::Claims;
use crate::services::glucose_service;
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    routing::get,
    Extension, Json, Router,
};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/glucose", get(get_readings).post(create_reading))
        .route("/glucose/{id}", get(get_reading).delete(delete_reading))
}

/// Create a new glucose reading
#[utoipa::path(
    post,
    path = "/api/glucose",
    tag = "Glucose",
    security(("bearer_auth" = [])),
    request_body = CreateGlucoseReadingRequest,
    responses(
        (status = 201, description = "Reading created", body = DataResponse<GlucoseReadingResponse>),
        (status = 400, description = "Invalid value", body = crate::dto::ErrorResponse),
        (status = 401, description = "Unauthorized", body = crate::dto::ErrorResponse),
    ),
)]
pub async fn create_reading(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Json(request): Json<CreateGlucoseReadingRequest>,
) -> Result<(StatusCode, Json<DataResponse<GlucoseReadingResponse>>), AppError> {
    let reading = glucose_service::create_reading(&state.db, claims.sub, request).await?;
    Ok((StatusCode::CREATED, Json(DataResponse::new(reading))))
}

/// List readings in a time window, newest first, paginated with `nextTo`
#[utoipa::path(
    get,
    path = "/api/glucose",
    tag = "Glucose",
    security(("bearer_auth" = [])),
    params(GlucoseListQuery),
    responses(
        (status = 200, description = "A page of readings", body = GlucoseListResponse),
        (status = 400, description = "Invalid window", body = crate::dto::ErrorResponse),
        (status = 401, description = "Unauthorized", body = crate::dto::ErrorResponse),
    ),
)]
pub async fn get_readings(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Query(query): Query<GlucoseListQuery>,
) -> Result<Json<GlucoseListResponse>, AppError> {
    let page =
        glucose_service::get_readings(&state.db, claims.sub, query.from, query.to, query.limit)
            .await?;
    Ok(Json(page))
}

/// Get a specific glucose reading by ID
#[utoipa::path(
    get,
    path = "/api/glucose/{id}",
    tag = "Glucose",
    security(("bearer_auth" = [])),
    params(("id" = i32, Path, description = "Reading ID")),
    responses(
        (status = 200, description = "Reading found", body = DataResponse<GlucoseReadingResponse>),
        (status = 401, description = "Unauthorized", body = crate::dto::ErrorResponse),
        (status = 404, description = "Not found", body = crate::dto::ErrorResponse),
    ),
)]
pub async fn get_reading(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<i32>,
) -> Result<Json<DataResponse<GlucoseReadingResponse>>, AppError> {
    let reading = glucose_service::get_reading_by_id(&state.db, claims.sub, id).await?;
    Ok(Json(DataResponse::new(reading)))
}

/// Delete a glucose reading
#[utoipa::path(
    delete,
    path = "/api/glucose/{id}",
    tag = "Glucose",
    security(("bearer_auth" = [])),
    params(("id" = i32, Path, description = "Reading ID")),
    responses(
        (status = 200, description = "Reading deleted", body = ApiMessageResponse),
        (status = 401, description = "Unauthorized", body = crate::dto::ErrorResponse),
        (status = 404, description = "Not found", body = crate::dto::ErrorResponse),
    ),
)]
pub async fn delete_reading(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<i32>,
) -> Result<Json<ApiMessageResponse>, AppError> {
    glucose_service::delete_reading(&state.db, claims.sub, id).await?;
    Ok(Json(ApiMessageResponse::ok("Reading deleted successfully")))
}

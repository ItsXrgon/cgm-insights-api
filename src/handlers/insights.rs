use crate::dto::{DataResponse, InsightsQuery, InsightsResponse};
use crate::error::AppError;
use crate::server::AppState;
use crate::services::{auth_service::Claims, insights_service};
use axum::{
    extract::{Query, State},
    routing::get,
    Extension, Json, Router,
};

pub fn routes() -> Router<AppState> {
    Router::new().route("/insights", get(get_insights))
}

/// Glucose statistics: mean, variability, GMI and time in ranges
#[utoipa::path(
    get,
    path = "/api/insights",
    tag = "Glucose",
    security(("bearer_auth" = [])),
    params(InsightsQuery),
    responses(
        (status = 200, description = "Statistics for the window", body = DataResponse<InsightsResponse>),
        (status = 400, description = "Invalid window", body = crate::dto::ErrorResponse),
        (status = 401, description = "Unauthorized", body = crate::dto::ErrorResponse),
    ),
)]
pub async fn get_insights(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Query(query): Query<InsightsQuery>,
) -> Result<Json<DataResponse<InsightsResponse>>, AppError> {
    let insights =
        insights_service::get_insights(&state.db, claims.sub, query.from, query.to).await?;
    Ok(Json(DataResponse::new(insights)))
}

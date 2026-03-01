use crate::{dto::ApiInfoResponse, server::AppState};
use axum::{routing::get, Json, Router};

pub fn routes() -> Router<AppState> {
    Router::new().route("/", get(api_info))
}

/// API information and available endpoints
#[utoipa::path(
    get,
    path = "/api",
    tag = "Health",
    responses((status = 200, description = "API info", body = ApiInfoResponse)),
)]
pub async fn api_info() -> Json<ApiInfoResponse> {
    Json(ApiInfoResponse {
        name: "CGM Insights Server",
        version: env!("CARGO_PKG_VERSION"),
        description: "A Rust server for CGM integrations (LibreLinkUp, Dexcom, etc.)",
        endpoints: vec![
            "/health",
            "/api",
            "/swagger-ui",
            "/api-docs/openapi.json",
            "POST /api/auth/signup",
            "POST /api/auth/login",
            "POST /api/glucose",
            "GET /api/glucose?limit=100",
            "GET /api/glucose/:id",
            "DELETE /api/glucose/:id",
            "POST /api/sync",
            "GET /api/cgm",
            "POST /api/cgm",
            "PATCH /api/cgm/:id",
            "POST /api/cgm/:id/active",
            "DELETE /api/cgm/:id",
        ],
    })
}

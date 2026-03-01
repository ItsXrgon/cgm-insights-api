use crate::{dto::ApiInfoResponse, server::AppState};
use axum::{routing::get, Json, Router};

pub fn routes() -> Router<AppState> {
    Router::new().route("/", get(api_info))
}

async fn api_info() -> Json<ApiInfoResponse> {
    Json(ApiInfoResponse {
        name: "CGM Insights Server",
        version: env!("CARGO_PKG_VERSION"),
        description: "A Rust server for CGM integrations (LibreLinkUp, Dexcom, etc.)",
        endpoints: vec![
            "/health",
            "/api",
            "POST /glucose",
            "GET /glucose?limit=100",
            "GET /glucose/:id",
            "DELETE /glucose/:id",
            "POST /sync - Manually trigger LibreLink Up sync",
        ],
    })
}

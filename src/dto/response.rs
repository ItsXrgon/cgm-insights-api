use serde::Serialize;

/// Health check response
#[derive(Debug, Serialize)]
pub struct HealthResponse {
    pub status: &'static str,
    pub timestamp: i64,
}

/// API information response
#[derive(Debug, Serialize)]
pub struct ApiInfoResponse {
    pub name: &'static str,
    pub version: &'static str,
    pub description: &'static str,
    pub endpoints: Vec<&'static str>,
}

/// Glucose reading response
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GlucoseReadingResponse {
    pub id: i32,
    pub value_mg_dl: f64,
    pub timestamp: String,
    pub device_id: Option<String>,
    pub notes: Option<String>,
    pub created_at: String,
}

/// Generic error response
#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    pub error: String,
    pub message: Option<String>,
}

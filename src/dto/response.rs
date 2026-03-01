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

/// Auth response with token
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthResponse {
    pub token: String,
    pub user: UserResponse,
}

/// User response (no password)
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UserResponse {
    pub id: i32,
    pub username: String,
    pub created_at: String,
}

/// CGM Credential response
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CgmCredentialResponse {
    pub id: i32,
    pub user_id: i32,
    pub cgm_type: String,
    pub username: String,
    pub region: Option<String>,
    pub is_active: bool,
    pub created_at: String,
    pub updated_at: String,
}

/// Generic error response
#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    pub error: String,
    pub message: Option<String>,
}

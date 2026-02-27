use chrono::{DateTime, Utc};
use serde::Deserialize;

/// Login request payload
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

/// Create glucose reading request
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateGlucoseReadingRequest {
    pub value_mg_dl: f64,
    #[serde(default = "chrono::Utc::now")]
    pub timestamp: DateTime<Utc>,
    pub device_id: Option<String>,
    pub notes: Option<String>,
}

/// Query parameters for glucose readings
#[derive(Debug, Deserialize)]
pub struct GlucoseQueryParams {
    pub start_time: Option<i64>,
    pub end_time: Option<i64>,
    pub limit: Option<u32>,
}

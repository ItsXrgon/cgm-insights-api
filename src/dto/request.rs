use chrono::{DateTime, Utc};
use serde::Deserialize;
use utoipa::ToSchema;

/// Signup request payload
#[derive(Debug, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct SignupRequest {
    pub username: String,
    pub password: String,
    pub cgm_username: String,
    pub cgm_password: String,
    pub cgm_type: String, // "FreeStyle" or "Dexcom"
    pub cgm_region: Option<String>,
}

/// Login request payload
#[derive(Debug, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

/// Create glucose reading request
#[derive(Debug, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CreateGlucoseReadingRequest {
    pub value_mg_dl: f64,
    #[serde(default = "chrono::Utc::now")]
    pub timestamp: DateTime<Utc>,
    pub device_id: Option<String>,
    pub notes: Option<String>,
}

/// Update CGM credentials request
#[derive(Debug, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct UpdateCgmCredentialRequest {
    pub cgm_type: Option<String>,
    pub cgm_username: Option<String>,
    pub cgm_password: Option<String>,
    pub cgm_region: Option<String>,
    pub is_active: Option<bool>,
}

/// Create CGM credential request
#[derive(Debug, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CreateCgmCredentialRequest {
    pub cgm_type: String,
    pub cgm_username: String,
    pub cgm_password: String,
    pub cgm_region: Option<String>,
    pub is_active: bool,
}

/// Query parameters for glucose readings
#[derive(Debug, Deserialize, ToSchema)]
pub struct GlucoseQueryParams {
    pub start_time: Option<i64>,
    pub end_time: Option<i64>,
    pub limit: Option<u32>,
}

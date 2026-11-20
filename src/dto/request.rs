use chrono::{DateTime, Utc};
use serde::Deserialize;
use utoipa::{IntoParams, ToSchema};

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
    #[serde(default)]
    pub is_high: bool,
    #[serde(default)]
    pub is_low: bool,
    pub trend: Option<String>,
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

/// Query for listing readings: `[from, to)`, newest first
#[derive(Debug, Deserialize, IntoParams, ToSchema)]
#[into_params(parameter_in = Query)]
pub struct GlucoseListQuery {
    /// Inclusive lower bound, RFC 3339 (e.g. `2026-11-01T00:00:00Z`)
    pub from: Option<DateTime<Utc>>,
    /// Exclusive upper bound, RFC 3339; use `nextTo` from the previous page to paginate
    pub to: Option<DateTime<Utc>>,
    /// Page size, 1–1000 (default 100)
    pub limit: Option<i64>,
}

/// Window for insights; defaults to the 14 days before `to` (default now), max 90 days
#[derive(Debug, Deserialize, IntoParams, ToSchema)]
#[into_params(parameter_in = Query)]
pub struct InsightsQuery {
    /// Inclusive lower bound, RFC 3339
    pub from: Option<DateTime<Utc>>,
    /// Exclusive upper bound, RFC 3339
    pub to: Option<DateTime<Utc>>,
}

/// Confirm account deletion with the account password
#[derive(Debug, Deserialize, ToSchema)]
pub struct DeleteAccountRequest {
    pub password: String,
}

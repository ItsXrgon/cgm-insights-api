use crate::services::insights_service::GlucoseStats;
use chrono::{DateTime, Utc};
use serde::Serialize;
use utoipa::ToSchema;

/// Standard success envelope: `{ "success": true, "data": ... }`
#[derive(Debug, Serialize, ToSchema)]
pub struct DataResponse<T> {
    pub success: bool,
    pub data: T,
}

impl<T> DataResponse<T> {
    pub fn new(data: T) -> Self {
        Self {
            success: true,
            data,
        }
    }
}

/// Health check response
#[derive(Debug, Serialize, ToSchema)]
pub struct HealthResponse {
    /// `OK` or `UNAVAILABLE`
    pub status: &'static str,
    /// `up` or `down`
    pub database: &'static str,
    pub timestamp: i64,
}

/// API information response
#[derive(Debug, Serialize, ToSchema)]
pub struct ApiInfoResponse {
    pub name: &'static str,
    pub version: &'static str,
    pub description: &'static str,
    pub endpoints: Vec<&'static str>,
}

/// Glucose reading response
#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct GlucoseReadingResponse {
    pub id: i32,
    pub value_mg_dl: f64,
    pub timestamp: String,
    pub is_high: bool,
    pub is_low: bool,
    pub trend: Option<String>,
    pub device_id: Option<String>,
    pub notes: Option<String>,
    pub created_at: String,
}

/// A page of readings, newest first
#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct GlucoseListResponse {
    pub success: bool,
    pub data: Vec<GlucoseReadingResponse>,
    pub count: usize,
    /// Pass as `to` to fetch the next (older) page; absent on the last page
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_to: Option<String>,
}

/// Auth response with token
#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct AuthResponse {
    pub token: String,
    pub user: UserResponse,
}

/// User response (no password)
#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct UserResponse {
    pub id: i32,
    pub username: String,
    pub created_at: String,
}

/// CGM Credential response (never includes the password)
#[derive(Debug, Serialize, ToSchema)]
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
    /// When the latest sync attempt started
    pub last_sync_at: Option<String>,
    /// `ok`, `credentials_rejected`, `account_locked` or `failed`
    pub last_sync_status: Option<String>,
    /// Why the latest sync failed, suitable to show the user
    pub last_sync_error: Option<String>,
}

/// Result of a manual sync
#[derive(Debug, Serialize, ToSchema)]
pub struct SyncResponse {
    pub success: bool,
    pub message: String,
    pub readings_synced: usize,
}

/// Glucose statistics over a time window
#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct InsightsResponse {
    pub from: String,
    pub to: String,
    /// Readings in the window; the other fields are null when this is 0
    pub count: usize,
    pub mean_mg_dl: Option<f64>,
    pub sd_mg_dl: Option<f64>,
    /// Coefficient of variation, %
    pub cv_percent: Option<f64>,
    /// Glucose management indicator (estimated A1c), %
    pub gmi_percent: Option<f64>,
    pub min_mg_dl: Option<f64>,
    pub max_mg_dl: Option<f64>,
    pub time_in_ranges: Option<TimeInRanges>,
}

/// Share of readings per consensus range, %
#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct TimeInRanges {
    /// < 54 mg/dL
    pub very_low: f64,
    /// 54–69 mg/dL
    pub low: f64,
    /// 70–180 mg/dL
    pub in_range: f64,
    /// 181–250 mg/dL
    pub high: f64,
    /// > 250 mg/dL
    pub very_high: f64,
}

impl InsightsResponse {
    pub fn new(from: DateTime<Utc>, to: DateTime<Utc>, stats: Option<GlucoseStats>) -> Self {
        let round = |v: f64| (v * 10.0).round() / 10.0;
        Self {
            from: from.to_rfc3339(),
            to: to.to_rfc3339(),
            count: stats.as_ref().map_or(0, |s| s.count),
            mean_mg_dl: stats.as_ref().map(|s| round(s.mean_mg_dl)),
            sd_mg_dl: stats.as_ref().map(|s| round(s.sd_mg_dl)),
            cv_percent: stats.as_ref().map(|s| round(s.cv_percent)),
            gmi_percent: stats.as_ref().map(|s| round(s.gmi_percent)),
            min_mg_dl: stats.as_ref().map(|s| s.min_mg_dl),
            max_mg_dl: stats.as_ref().map(|s| s.max_mg_dl),
            time_in_ranges: stats.as_ref().map(|s| TimeInRanges {
                very_low: round(s.very_low_percent),
                low: round(s.low_percent),
                in_range: round(s.in_range_percent),
                high: round(s.high_percent),
                very_high: round(s.very_high_percent),
            }),
        }
    }
}

/// Everything stored about the account (passwords excluded)
#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct AccountExport {
    pub exported_at: String,
    pub user: UserResponse,
    pub cgm_credentials: Vec<CgmCredentialResponse>,
    pub glucose_readings: Vec<GlucoseReadingResponse>,
}

/// Wrapper for API responses that return `{ success, message }`
#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ApiMessageResponse {
    pub success: bool,
    pub message: String,
}

impl ApiMessageResponse {
    pub fn ok(message: impl Into<String>) -> Self {
        Self {
            success: true,
            message: message.into(),
        }
    }
}

/// Error response: every non-2xx response has this shape
#[derive(Debug, Serialize, ToSchema)]
pub struct ErrorResponse {
    pub error: String,
}

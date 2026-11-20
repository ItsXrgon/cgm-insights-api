use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

/// Upper bound for a stored glucose value. Sensors saturate well below this
/// (Libre/Sibionics report "HI" around 500 mg/dL).
pub const MAX_MG_DL: f64 = 1000.0;

/// Whether a mg/dL value could be a real measurement. Rejects NaN/inf, zero/negative
/// sentinels (sensor warm-up, errors) and absurd values, for both manual entries and
/// provider data.
pub fn is_plausible_mg_dl(value: f64) -> bool {
    value.is_finite() && value > 0.0 && value <= MAX_MG_DL
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct GlucoseReading {
    pub id: i32,
    pub user_id: Option<i32>,
    pub value_mg_dl: f64,
    pub timestamp: DateTime<Utc>,
    #[serde(rename = "isHigh")]
    pub is_high: bool,
    #[serde(rename = "isLow")]
    pub is_low: bool,
    pub trend: Option<String>,
    pub device_id: Option<String>,
    pub notes: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewGlucoseReading {
    pub user_id: Option<i32>,
    pub value_mg_dl: f64,
    pub timestamp: DateTime<Utc>,
    #[serde(rename = "isHigh")]
    pub is_high: bool,
    #[serde(rename = "isLow")]
    pub is_low: bool,
    pub trend: Option<String>,
    pub device_id: Option<String>,
    pub notes: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plausibility_bounds() {
        assert!(is_plausible_mg_dl(1.0));
        assert!(is_plausible_mg_dl(110.0));
        assert!(is_plausible_mg_dl(MAX_MG_DL));
        assert!(!is_plausible_mg_dl(0.0));
        assert!(!is_plausible_mg_dl(-5.0));
        assert!(!is_plausible_mg_dl(MAX_MG_DL + 0.1));
        assert!(!is_plausible_mg_dl(f64::NAN));
        assert!(!is_plausible_mg_dl(f64::INFINITY));
    }
}

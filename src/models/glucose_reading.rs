use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct GlucoseReading {
    pub id: i32,
    pub user_id: Option<i32>,
    pub value_mg_dl: f64,
    pub timestamp: DateTime<Utc>,
    pub device_id: Option<String>,
    pub notes: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewGlucoseReading {
    pub user_id: Option<i32>,
    pub value_mg_dl: f64,
    pub timestamp: DateTime<Utc>,
    pub device_id: Option<String>,
    pub notes: Option<String>,
}

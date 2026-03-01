use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "cgm_type", rename_all = "lowercase")]
pub enum CgmType {
    FreeStyle,
    Dexcom,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct CgmCredential {
    pub id: i32,
    pub user_id: i32,
    pub cgm_type: String,
    pub username: String,
    pub password: String,
    pub region: Option<String>,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewCgmCredential {
    pub user_id: i32,
    pub cgm_type: String,
    pub username: String,
    pub password: String,
    pub region: Option<String>,
    pub is_active: bool,
}

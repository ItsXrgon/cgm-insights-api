use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use std::fmt;
use std::str::FromStr;

/// Supported CGM platforms.
///
/// Stored in `cgm_credentials.cgm_type` as the canonical name from [`CgmType::as_str`].
/// Parsing is case-insensitive so rows written before validation existed
/// (e.g. `"freestyle"`) still resolve. Adding a platform means adding a variant here;
/// the compiler then points at every place that must handle it (see `SyncService`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CgmType {
    FreeStyle,
    Dexcom,
}

impl CgmType {
    pub const ALL: &'static [CgmType] = &[CgmType::FreeStyle, CgmType::Dexcom];

    pub fn as_str(self) -> &'static str {
        match self {
            CgmType::FreeStyle => "FreeStyle",
            CgmType::Dexcom => "Dexcom",
        }
    }
}

impl fmt::Display for CgmType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for CgmType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = s.trim();
        CgmType::ALL
            .iter()
            .copied()
            .find(|t| t.as_str().eq_ignore_ascii_case(s))
            .ok_or_else(|| {
                let supported: Vec<&str> = CgmType::ALL.iter().map(|t| t.as_str()).collect();
                format!(
                    "Unsupported CGM type '{}'. Supported: {}",
                    s,
                    supported.join(", ")
                )
            })
    }
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
    /// When the latest sync attempt started (also the sync cooldown anchor)
    pub last_sync_at: Option<DateTime<Utc>>,
    /// Outcome of the latest sync attempt, see [`SyncStatus`]
    pub last_sync_status: Option<String>,
    /// User-facing reason when the latest sync did not succeed
    pub last_sync_error: Option<String>,
}

/// Outcome of a sync attempt, stored in `cgm_credentials.last_sync_status`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SyncStatus {
    Ok,
    /// Wrong password, region, or terms to accept in the vendor app
    CredentialsRejected,
    /// Vendor temporarily locked the account
    AccountLocked,
    /// Anything else (vendor down, our bug); retried on the next run
    Failed,
}

impl SyncStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            SyncStatus::Ok => "ok",
            SyncStatus::CredentialsRejected => "credentials_rejected",
            SyncStatus::AccountLocked => "account_locked",
            SyncStatus::Failed => "failed",
        }
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_case_insensitively_and_trims() {
        assert_eq!("FreeStyle".parse::<CgmType>(), Ok(CgmType::FreeStyle));
        assert_eq!("freestyle".parse::<CgmType>(), Ok(CgmType::FreeStyle));
        assert_eq!("  FREESTYLE ".parse::<CgmType>(), Ok(CgmType::FreeStyle));
        assert_eq!("dexcom".parse::<CgmType>(), Ok(CgmType::Dexcom));
    }

    #[test]
    fn rejects_unknown_type_listing_supported() {
        let err = "medtronic".parse::<CgmType>().unwrap_err();
        assert!(err.contains("medtronic"));
        assert!(err.contains("FreeStyle"));
        assert!("".parse::<CgmType>().is_err());
    }

    #[test]
    fn canonical_name_round_trips() {
        for t in CgmType::ALL {
            assert_eq!(t.as_str().parse::<CgmType>(), Ok(*t));
            assert_eq!(t.to_string(), t.as_str());
        }
    }
}

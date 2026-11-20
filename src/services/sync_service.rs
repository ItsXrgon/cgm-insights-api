use crate::error::AppError;
use crate::models::{is_plausible_mg_dl, CgmCredential, CgmType, NewGlucoseReading};
use crate::repositories::glucose_repository;
use crate::services::LibreLinkClient;
use crate::DbPool;
use std::sync::Arc;
use tracing::{info, warn};

/// Region used for LibreLink when the credential has none.
const LIBRE_DEFAULT_REGION: &str = "eu";

/// Sync service that fetches data from CGM platforms and stores it in the database
pub struct SyncService {
    db: Arc<DbPool>,
}

impl SyncService {
    pub fn new(db: Arc<DbPool>) -> Self {
        Self { db }
    }

    /// Fetch latest readings for a CGM credential and upsert them.
    /// Returns the number of rows inserted or updated.
    pub async fn sync_for_credential(&self, cred: &CgmCredential) -> Result<usize, AppError> {
        info!(user_id = cred.user_id, cgm_type = %cred.cgm_type, "Starting CGM sync");

        let cgm_type: CgmType = cred.cgm_type.parse().map_err(|e| {
            AppError::ConfigError(format!("Credential {} has invalid cgm_type: {}", cred.id, e))
        })?;

        let fetched = fetch_readings(cgm_type, cred).await?;
        let fetched_count = fetched.len();
        let readings = prepare_readings(fetched, cred.user_id);

        if readings.len() < fetched_count {
            warn!(
                user_id = cred.user_id,
                dropped = fetched_count - readings.len(),
                "Dropped implausible readings from CGM"
            );
        }
        if readings.is_empty() {
            warn!(user_id = cred.user_id, "No readings returned from CGM");
            return Ok(0);
        }

        let stored_count = glucose_repository::insert_many(self.db.as_ref(), readings).await?;
        Ok(stored_count as usize)
    }
}

/// The only place that knows which client serves which platform.
/// To add a platform: add a `CgmType` variant, a client that returns
/// `Vec<NewGlucoseReading>` in mg/dL, and an arm here.
async fn fetch_readings(
    cgm_type: CgmType,
    cred: &CgmCredential,
) -> Result<Vec<NewGlucoseReading>, AppError> {
    match cgm_type {
        CgmType::FreeStyle => {
            let region = cred
                .region
                .clone()
                .unwrap_or_else(|| LIBRE_DEFAULT_REGION.to_string());
            LibreLinkClient::new(cred.username.clone(), cred.password.clone(), Some(region))?
                .fetch_latest_readings()
                .await
        }
        CgmType::Dexcom => Err(AppError::ValidationError(
            "Dexcom sync is not yet implemented".to_string(),
        )),
    }
}

/// Provider-agnostic cleanup applied to every platform's output:
/// attach the owning user and drop values that can't be real measurements.
pub(crate) fn prepare_readings(
    readings: Vec<NewGlucoseReading>,
    user_id: i32,
) -> Vec<NewGlucoseReading> {
    readings
        .into_iter()
        .filter(|r| is_plausible_mg_dl(r.value_mg_dl))
        .map(|r| NewGlucoseReading {
            user_id: Some(user_id),
            ..r
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    fn reading(value: f64, user_id: Option<i32>) -> NewGlucoseReading {
        NewGlucoseReading {
            user_id,
            value_mg_dl: value,
            timestamp: Utc::now(),
            is_high: false,
            is_low: false,
            trend: Some("flat".into()),
            device_id: None,
            notes: None,
        }
    }

    #[test]
    fn prepare_assigns_owner_and_overrides_provider_user() {
        let out = prepare_readings(vec![reading(100.0, None), reading(110.0, Some(99))], 5);
        assert_eq!(out.len(), 2);
        assert!(out.iter().all(|r| r.user_id == Some(5)));
    }

    #[test]
    fn prepare_drops_implausible_values() {
        let out = prepare_readings(
            vec![
                reading(0.0, None),
                reading(-1.0, None),
                reading(f64::NAN, None),
                reading(5_000.0, None),
                reading(85.0, None),
            ],
            1,
        );
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].value_mg_dl, 85.0);
    }

    fn credential(cgm_type: &str) -> CgmCredential {
        CgmCredential {
            id: 1,
            user_id: 1,
            cgm_type: cgm_type.into(),
            username: "u".into(),
            password: "p".into(),
            region: None,
            is_active: true,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    #[tokio::test]
    async fn dexcom_is_rejected_without_network() {
        let err = fetch_readings(CgmType::Dexcom, &credential("Dexcom"))
            .await
            .unwrap_err();
        assert!(matches!(err, AppError::ValidationError(_)));
    }
}

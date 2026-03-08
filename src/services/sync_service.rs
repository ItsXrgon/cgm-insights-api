use crate::error::AppError;
use crate::models::CgmCredential;
use crate::repositories::glucose_repository;
use crate::services::LibreLinkClient;
use crate::DbPool;
use std::sync::Arc;
use tracing::{info, warn};

/// Sync service that fetches data from CGM platforms and stores it in the database
pub struct SyncService {
    db: Arc<DbPool>,
}

impl SyncService {
    pub fn new(db: Arc<DbPool>) -> Self {
        Self { db }
    }

    /// Fetch latest readings for a specific CGM credential and store them in the database.
    /// Returns the number of new readings stored (duplicates by user_id+timestamp are skipped).
    pub async fn sync_for_credential(&self, cred: &CgmCredential) -> Result<usize, AppError> {
        info!(
            user_id = cred.user_id,
            cgm_type = %cred.cgm_type,
            "Starting CGM sync"
        );

        let readings = match cred.cgm_type.to_lowercase().as_str() {
            "freestyle" => {
                let client = LibreLinkClient::new(
                    cred.username.clone(),
                    cred.password.clone(),
                    cred.region.clone().unwrap_or_else(|| "eu".to_string()),
                )?;
                client.fetch_latest_readings(Some(cred.user_id)).await?
            }
            "dexcom" => {
                return Err(AppError::ApiError(
                    "Dexcom sync is not yet implemented".to_string(),
                ));
            }
            _ => {
                return Err(AppError::ConfigError(format!(
                    "Unsupported CGM type: {}",
                    cred.cgm_type
                )));
            }
        };

        if readings.is_empty() {
            warn!(user_id = cred.user_id, "No readings returned from CGM");
            return Ok(0);
        }

        let stored_count = glucose_repository::insert_many(self.db.as_ref(), readings).await?;
        Ok(stored_count as usize)
    }
}

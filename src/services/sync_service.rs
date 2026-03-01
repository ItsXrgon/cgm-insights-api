use crate::error::AppError;
use crate::models::CgmCredential;
use crate::repositories::glucose_repository;
use crate::services::LibreLinkClient;
use sqlx::{Pool, Postgres};

/// Sync service that fetches data from CGM platforms and stores it in the database
pub struct SyncService {
    db: Pool<Postgres>,
}

impl SyncService {
    pub fn new(db: Pool<Postgres>) -> Self {
        Self { db }
    }

    /// Fetch latest readings for a specific CGM credential and store them in the database
    pub async fn sync_for_credential(&self, cred: &CgmCredential) -> Result<usize, AppError> {
        println!(
            "🔄 Starting sync for user {} (CGM: {})",
            cred.user_id, cred.cgm_type
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
            println!("   No new readings to sync for user {}", cred.user_id);
            return Ok(0);
        }

        // Store readings in database using bulk insert
        let stored_count = glucose_repository::insert_many(&self.db, readings).await?;

        Ok(stored_count as usize)
    }

    /// Legacy method for backward compatibility if needed, or just remove it
    pub async fn sync_readings(&self) -> Result<usize, AppError> {
        // This is now problematic since we don't have a global client.
        // We should probably remove it or make it sync all active credentials.
        Ok(0)
    }
}

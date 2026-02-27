use crate::error::AppError;
use crate::repositories::glucose_repository;
use crate::services::LibreLinkClient;
use sqlx::{Pool, Postgres};
use std::sync::Arc;

/// Sync service that fetches data from LibreLink Up and stores it in the database
pub struct SyncService {
    client: Arc<LibreLinkClient>,
    db: Pool<Postgres>,
}

impl SyncService {
    pub fn new(client: LibreLinkClient, db: Pool<Postgres>) -> Self {
        Self {
            client: Arc::new(client),
            db,
        }
    }

    /// Fetch latest readings from LibreLink Up and store them in the database
    pub async fn sync_readings(&self) -> Result<usize, AppError> {
        println!("🔄 Starting sync process...");

        // PLACEHOLDER: Authenticate with API
        self.client.authenticate().await?;

        // PLACEHOLDER: Fetch latest readings from API
        let readings = self.client.fetch_latest_readings().await?;

        if readings.is_empty() {
            println!("   No new readings to sync");
            return Ok(0);
        }

        // PLACEHOLDER: Store readings in database
        let mut stored_count = 0;
        for reading in readings {
            match glucose_repository::insert(&self.db, reading).await {
                Ok(_) => {
                    stored_count += 1;
                }
                Err(e) => {
                    eprintln!("   ⚠️  Failed to store reading: {}", e);
                }
            }
        }

        println!("   ✅ Synced {} readings", stored_count);
        Ok(stored_count)
    }
}

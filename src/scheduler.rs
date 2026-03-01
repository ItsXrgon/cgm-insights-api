use crate::repositories::cgm_repository;
use crate::services::SyncService;
use sqlx::{Pool, Postgres};
use std::sync::Arc;
use std::time::Duration;

/// Start the background scheduler that syncs data every 5 minutes
pub async fn start_sync_scheduler(sync_service: Arc<SyncService>, db: Pool<Postgres>) {
    println!("⏰ Starting background scheduler (syncs all active users every 5 minutes)...");

    // Spawn background task
    tokio::spawn(async move {
        // Initial sync
        run_all_syncs(&sync_service, &db).await;

        // Create interval timer for 5 minutes
        let mut interval = tokio::time::interval(Duration::from_secs(300));
        interval.tick().await; // Skip first tick

        loop {
            interval.tick().await;
            run_all_syncs(&sync_service, &db).await;
        }
    });

    println!("✅ Background scheduler started");
}

async fn run_all_syncs(sync_service: &SyncService, db: &Pool<Postgres>) {
    println!("⏰ Running scheduled sync for all active credentials...");

    match cgm_repository::find_all_active(db).await {
        Ok(creds) => {
            for cred in creds {
                if let Err(e) = sync_service.sync_for_credential(&cred).await {
                    eprintln!(
                        "❌ Sync failed for user {} ({}): {}",
                        cred.user_id, cred.cgm_type, e
                    );
                }
            }
        }
        Err(e) => {
            eprintln!("❌ Failed to fetch active CGM credentials: {}", e);
        }
    }
}

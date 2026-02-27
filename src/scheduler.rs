use crate::services::SyncService;
use std::sync::Arc;
use std::time::Duration;

/// Start the background scheduler that syncs data every hour
pub async fn start_sync_scheduler(sync_service: Arc<SyncService>) {
    println!("⏰ Starting background scheduler (syncs every 1 hour)...");

    // Spawn background task
    tokio::spawn(async move {
        // Sync immediately on startup
        println!("🚀 Running initial sync...");
        if let Err(e) = sync_service.sync_readings().await {
            eprintln!("❌ Initial sync failed: {}", e);
        }

        // Create interval timer for 1 hour
        let mut interval = tokio::time::interval(Duration::from_secs(3600)); // 3600 seconds = 1 hour
        interval.tick().await; // Skip first tick (already ran initial sync)

        loop {
            interval.tick().await;
            println!("⏰ Scheduled sync triggered...");

            match sync_service.sync_readings().await {
                Ok(count) => {
                    if count > 0 {
                        println!("✅ Scheduled sync completed: {} readings stored", count);
                    }
                }
                Err(e) => {
                    eprintln!("❌ Scheduled sync failed: {}", e);
                }
            }
        }
    });

    println!("✅ Background scheduler started");
}

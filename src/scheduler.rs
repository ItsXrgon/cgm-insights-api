use crate::repositories::cgm_repository;
use crate::services::SyncService;
use crate::DbPool;
use std::sync::Arc;
use std::time::Duration;
use tracing::{error, info};

/// Start the background scheduler that syncs CGM data for all active credentials at a fixed interval.
/// Runs an initial sync immediately, then every `interval_secs` seconds (default 1h).
pub async fn start_sync_scheduler(
    sync_service: Arc<SyncService>,
    db: Arc<DbPool>,
    interval_secs: u64,
) {
    let interval = Duration::from_secs(interval_secs);
    info!(
        interval_secs = interval_secs,
        "Starting CGM sync scheduler (syncs all active credentials every {} min)",
        interval_secs / 60
    );

    tokio::spawn(async move {
        // Initial sync shortly after startup
        run_all_syncs(&sync_service, &db).await;

        let mut ticker = tokio::time::interval(interval);
        ticker.tick().await; // Skip first immediate tick

        loop {
            ticker.tick().await;
            run_all_syncs(&sync_service, &db).await;
        }
    });

    info!("CGM sync scheduler started");
}

async fn run_all_syncs(sync_service: &SyncService, db: &Arc<DbPool>) {
    info!("Running scheduled CGM sync for all active credentials");

    let creds = match cgm_repository::find_all_active(db.as_ref()).await {
        Ok(c) => c,
        Err(e) => {
            error!(error = %e, "Failed to load active CGM credentials");
            sentry::capture_error(&e);
            return;
        }
    };

    if creds.is_empty() {
        info!("No active CGM credentials to sync");
        return;
    }

    for cred in &creds {
        match sync_service.sync_for_credential(cred).await {
            Ok(count) => {
                info!(
                    user_id = cred.user_id,
                    cgm_type = %cred.cgm_type,
                    readings_stored = count,
                    "CGM sync completed"
                );
            }
            Err(e) => {
                let msg = e.to_string();
                // LibreLink temporarily locks after failed logins; log as warning and retry next cycle
                if msg.contains("temporarily locked") {
                    tracing::warn!(
                        user_id = cred.user_id,
                        cgm_type = %cred.cgm_type,
                        error = %e,
                        "CGM sync skipped: account temporarily locked (will retry on next run)"
                    );
                } else {
                    error!(
                        user_id = cred.user_id,
                        cgm_type = %cred.cgm_type,
                        error = %e,
                        "CGM sync failed"
                    );
                    sentry::configure_scope(|scope| {
                        scope.set_tag("user_id", cred.user_id.to_string());
                        scope.set_tag("cgm_type", &cred.cgm_type);
                    });
                    sentry::capture_error(&e);
                }
            }
        }
    }
}

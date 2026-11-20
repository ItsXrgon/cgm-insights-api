use crate::repositories::cgm_repository;
use crate::services::{SyncOutcome, SyncService};
use crate::DbPool;
use std::future::Future;
use std::sync::Arc;
use std::time::Duration;
use tracing::{error, info, warn};

/// Sync all active credentials now and then every `interval_secs`, until `shutdown`
/// resolves. Runs as a background task of the API process. A run in progress always
/// finishes; shutdown is only checked between runs.
pub async fn run_sync_scheduler(
    sync_service: Arc<SyncService>,
    db: Arc<DbPool>,
    interval_secs: u64,
    shutdown: impl Future<Output = ()>,
) {
    info!(interval_secs, "CGM sync scheduler started");

    let mut ticker = tokio::time::interval(Duration::from_secs(interval_secs));
    // A slow run shouldn't cause a burst of catch-up runs.
    ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
    tokio::pin!(shutdown);

    loop {
        tokio::select! {
            _ = &mut shutdown => break,
            _ = ticker.tick() => run_all_syncs(&sync_service, &db).await,
        }
    }

    info!("CGM sync scheduler stopped");
}

async fn run_all_syncs(sync_service: &SyncService, db: &DbPool) {
    let creds = match cgm_repository::find_all_active(db).await {
        Ok(c) => c,
        Err(e) => {
            error!(error = %e, "Failed to load active CGM credentials");
            sentry::capture_error(&e);
            return;
        }
    };

    let ids: Vec<i32> = creds.iter().map(|c| c.id).collect();
    sync_service.retain_clients(&ids);
    info!(credentials = creds.len(), "Running scheduled CGM sync");

    for cred in &creds {
        match sync_service.sync_credential(cred).await {
            Ok(SyncOutcome::Synced(count)) => {
                info!(
                    user_id = cred.user_id,
                    cgm_type = %cred.cgm_type,
                    readings_stored = count,
                    "CGM sync completed"
                );
            }
            Ok(SyncOutcome::Skipped) => {
                info!(user_id = cred.user_id, "CGM sync skipped: synced recently");
            }
            Err(e) if e.is_cgm_account_issue() => {
                // Wrong credentials / vendor lockout: the user has to act, not us.
                // The reason is stored on the credential for the app to show.
                warn!(
                    user_id = cred.user_id,
                    cgm_type = %cred.cgm_type,
                    error = %e,
                    "CGM sync skipped: account needs attention"
                );
            }
            Err(e) => {
                error!(
                    user_id = cred.user_id,
                    cgm_type = %cred.cgm_type,
                    error = %e,
                    "CGM sync failed"
                );
                sentry::with_scope(
                    |scope| {
                        scope.set_tag("user_id", cred.user_id.to_string());
                        scope.set_tag("cgm_type", &cred.cgm_type);
                    },
                    || sentry::capture_error(&e),
                );
            }
        }
    }
}

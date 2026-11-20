use cgm_insights_api::server::{shutdown_signal, AppState};
use cgm_insights_api::{create_app, db, scheduler, services::SyncService, telemetry, Config};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::watch;

fn main() -> anyhow::Result<()> {
    let config = Arc::new(Config::from_env()?);
    // Must outlive the runtime so Sentry flushes on exit.
    let _sentry = telemetry::init(&config);

    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?
        .block_on(run(config))
}

/// One process serves the API and runs the sync scheduler. Running several instances
/// is safe: each sync is claimed in the database first, so instances skip credentials
/// another one just synced.
async fn run(config: Arc<Config>) -> anyhow::Result<()> {
    let pool = Arc::new(db::connect(&config.database_url).await?);
    let sync_service = Arc::new(SyncService::new(
        Arc::clone(&pool),
        config.sync_cooldown_secs,
    ));

    // One signal stops both the scheduler and the server.
    let (stop_tx, stop_rx) = watch::channel(false);
    tokio::spawn(async move {
        shutdown_signal().await;
        let _ = stop_tx.send(true);
    });
    let stopped = |mut rx: watch::Receiver<bool>| async move {
        let _ = rx.wait_for(|stop| *stop).await;
    };

    tracing::info!("CGM sync every {} min", config.sync_interval_secs / 60);
    let scheduler = tokio::spawn(scheduler::run_sync_scheduler(
        Arc::clone(&sync_service),
        Arc::clone(&pool),
        config.sync_interval_secs,
        stopped(stop_rx.clone()),
    ));

    let app = create_app(AppState {
        db: pool,
        sync_service,
        config: Arc::clone(&config),
    });
    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", config.port)).await?;
    tracing::info!(
        "Server running on http://127.0.0.1:{}/ (Swagger UI at /swagger-ui/)",
        config.port
    );

    // Finish in-flight requests, then let a running sync complete before exiting.
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .with_graceful_shutdown(stopped(stop_rx))
    .await?;
    tracing::info!("Server stopped");

    if let Err(e) = scheduler.await {
        tracing::error!(error = %e, "Sync scheduler task failed");
    }
    Ok(())
}

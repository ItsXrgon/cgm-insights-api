use cgm_insights_api::server::{shutdown_signal, AppState};
use cgm_insights_api::{create_app, db, scheduler, services::SyncService, telemetry, Config};
use std::net::SocketAddr;
use std::sync::Arc;

fn main() -> anyhow::Result<()> {
    let config = Arc::new(Config::from_env()?);
    // Must outlive the runtime so Sentry flushes on exit.
    let _sentry = telemetry::init(&config);

    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let pool = Arc::new(db::connect(&config.database_url).await?);
            let sync_service = Arc::new(SyncService::new(
                Arc::clone(&pool),
                config.sync_cooldown_secs,
            ));

            if std::env::var("FLY_PROCESS_GROUP").as_deref() == Ok("worker") {
                run_worker(pool, sync_service, &config).await;
                Ok(())
            } else {
                run_app(AppState {
                    db: pool,
                    sync_service,
                    config: Arc::clone(&config),
                })
                .await
            }
        })
}

/// Worker process: scheduled CGM syncs until SIGTERM/Ctrl-C
async fn run_worker(pool: Arc<cgm_insights_api::DbPool>, sync: Arc<SyncService>, config: &Config) {
    tracing::info!(
        "Worker running (CGM sync every {} min)",
        config.sync_interval_secs / 60
    );
    scheduler::run_sync_scheduler(sync, pool, config.sync_interval_secs, shutdown_signal()).await;
}

/// App process: HTTP API until SIGTERM/Ctrl-C, letting in-flight requests finish
async fn run_app(state: AppState) -> anyhow::Result<()> {
    let port = state.config.port;
    let app = create_app(state);

    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", port)).await?;
    tracing::info!(
        "Server running on http://127.0.0.1:{}/ (Swagger UI at /swagger-ui/)",
        port
    );

    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .with_graceful_shutdown(shutdown_signal())
    .await?;

    tracing::info!("Server stopped");
    Ok(())
}

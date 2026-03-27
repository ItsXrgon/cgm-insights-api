use cgm_insights_api::{create_app, db, repositories, scheduler, services, Config};
use std::sync::Arc;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

const EXCLUDED_TRANSACTION_ROUTES: &[&str] = &["/health", "/swagger-ui", "/api-docs"];

fn main() -> anyhow::Result<()> {
    let config = Config::from_env();

    // Initialize Sentry and Tracing
    let _guard = if let Some(dsn) = &config.sentry_dsn {
        println!(
            "🎯 Sentry initialized for environment: {}",
            config.environment
        );
        let release = config
            .sentry_release
            .clone()
            .map(std::borrow::Cow::Owned)
            .or_else(|| {
                let rev = env!("GIT_REV_SHORT");
                Some(std::borrow::Cow::Owned(format!(
                    "{}@{}",
                    env!("CARGO_PKG_NAME"),
                    if rev == "unknown" {
                        env!("CARGO_PKG_VERSION").to_string()
                    } else {
                        format!("{}-{}", env!("CARGO_PKG_VERSION"), rev)
                    }
                )))
            });

        let traces_sample_rate = config.sentry_traces_sample_rate;

        let guard = sentry::init((
            dsn.as_str(),
            sentry::ClientOptions {
                release,
                environment: Some(config.environment.clone().into()),
                send_default_pii: true,
                attach_stacktrace: true,
                traces_sample_rate,
                traces_sampler: Some(Arc::new(move |tx| {
                    if is_excluded_transaction_route(tx.name(), EXCLUDED_TRANSACTION_ROUTES) {
                        0.0
                    } else {
                        traces_sample_rate
                    }
                })),
                auto_session_tracking: true,
                session_mode: sentry::SessionMode::Request,
                ..Default::default()
            },
        ));

        // Initialize tracing with Sentry support.
        // This handles app logs (info!, error!, etc.) and sends them to Sentry.
        tracing_subscriber::registry()
            .with(tracing_subscriber::fmt::layer())
            .with(tracing_subscriber::EnvFilter::from_default_env())
            .with(sentry::integrations::tracing::layer())
            .init();

        Some(guard)
    } else {
        println!("ℹ️ Sentry disabled (SENTRY_DSN not found)");
        // Initialize tracing for console only.
        tracing_subscriber::registry()
            .with(tracing_subscriber::fmt::layer())
            .with(tracing_subscriber::EnvFilter::from_default_env())
            .init();
        None
    };

    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let pool = Arc::new(db::connect(&config.database_url).await);

            repositories::user_repository::init_table(pool.as_ref())
                .await
                .expect("Failed to initialize users table");

            repositories::cgm_repository::init_table(pool.as_ref())
                .await
                .expect("Failed to initialize cgm_credentials table");

            repositories::glucose_repository::init_table(pool.as_ref())
                .await
                .expect("Failed to initialize glucose_readings table");

            let is_worker = std::env::var("FLY_PROCESS_GROUP").as_deref() == Ok("worker");

            if is_worker {
                run_worker(pool, &config).await
            } else {
                run_app(pool, &config).await
            }
        })
}

/// Worker process: runs scheduled jobs
async fn run_worker(
    pool: Arc<cgm_insights_api::DbPool>,
    config: &cgm_insights_api::Config,
) -> anyhow::Result<()> {
    let sync_service = Arc::new(services::SyncService::new(Arc::clone(&pool)));
    scheduler::start_sync_scheduler(sync_service, Arc::clone(&pool), config.sync_interval_secs)
        .await;
    println!(
        "🔄 Worker running (CGM sync every {} min)",
        config.sync_interval_secs / 60
    );
    std::future::pending::<anyhow::Result<()>>().await
}

/// App process: HTTP APIs
async fn run_app(
    pool: Arc<cgm_insights_api::DbPool>,
    config: &cgm_insights_api::Config,
) -> anyhow::Result<()> {
    let sync_service = Arc::new(services::SyncService::new(Arc::clone(&pool)));
    let app = create_app(pool, sync_service, config.sentry_dsn.is_some());

    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", config.port)).await?;
    println!("🚀 Server running on http://127.0.0.1:{}/", config.port);
    println!("📊 Health check: http://127.0.0.1:{}/health", config.port);
    println!("📚 API info: http://127.0.0.1:{}/api", config.port);
    println!(
        "📖 Swagger UI: http://127.0.0.1:{}/swagger-ui/",
        config.port
    );

    axum::serve(listener, app).await?;

    Ok(())
}

fn is_excluded_transaction_route(tx_name: &str, excluded_routes: &[&str]) -> bool {
    let path_with_query = tx_name.split_whitespace().last().unwrap_or(tx_name);
    let path = path_with_query.split('?').next().unwrap_or(path_with_query);

    excluded_routes.iter().any(|route| {
        path == *route || path.starts_with(&format!("{}/", route.trim_end_matches('/')))
    })
}

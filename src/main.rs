use cgm_insights_api::{create_app, db, repositories, scheduler, services, Config};
use std::sync::Arc;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

fn main() -> anyhow::Result<()> {
    let config = Config::from_env();

    // Initialize Sentry and Tracing
    let _guard = if let Some(dsn) = &config.sentry_dsn {
        println!(
            "🎯 Sentry initialized for environment: {}",
            config.environment
        );
        let guard = sentry::init((
            dsn.as_str(),
            sentry::ClientOptions {
                release: sentry::release_name!(),
                environment: Some(config.environment.clone().into()),
                send_default_pii: true,
                ..Default::default()
            },
        ));

        // Initialize tracing with Sentry support.
        // This handles app logs (info!, error!, etc.) and sends them to Sentry.
        tracing_subscriber::registry()
            .with(tracing_subscriber::fmt::layer())
            .with(tracing_subscriber::EnvFilter::from_default_env())
            .with(sentry::integrations::tracing::SentryLayer::default())
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
            let pool = db::connect(&config.database_url).await;

            repositories::user_repository::init_table(&pool)
                .await
                .expect("Failed to initialize users table");

            repositories::cgm_repository::init_table(&pool)
                .await
                .expect("Failed to initialize cgm_credentials table");

            repositories::glucose_repository::init_table(&pool)
                .await
                .expect("Failed to initialize glucose_readings table");

            // Initialize sync service
            let sync_service = Arc::new(services::SyncService::new(pool.clone()));

            // Start background scheduler
            scheduler::start_sync_scheduler(sync_service.clone(), pool.clone()).await;

            let app: axum::Router = create_app(pool, sync_service, config.sentry_dsn.is_some());

            let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await?;
            println!("🚀 Server running on http://0.0.0.0:3000");
            println!("📊 Health check: http://0.0.0.0:3000/health");
            println!("📚 API info: http://0.0.0.0:3000/api");

            axum::serve(listener, app).await?;

            Ok(())
        })
}

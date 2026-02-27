use cgm_insights_api::{create_app, db, repositories, scheduler, services, Config};
use std::sync::Arc;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    env_logger::init();

    let config = Config::from_env();

    let pool = db::connect(&config.database_url).await;

    repositories::glucose_repository::init_table(&pool)
        .await
        .expect("Failed to initialize glucose_readings table");

    // Initialize LibreLink client (uses libre_link_up_api_client crate)
    let libre_link_client = services::LibreLinkClient::new(
        config.cgm_username.clone(),
        config.cgm_password.clone(),
        config.cgm_region.clone(),
    )
    .expect("Failed to create LibreLink client (check CGM_USERNAME, CGM_PASSWORD, CGM_REGION)");

    // Initialize sync service
    let sync_service = Arc::new(services::SyncService::new(libre_link_client, pool.clone()));

    // Start background scheduler
    scheduler::start_sync_scheduler(sync_service).await;

    let app: axum::Router = create_app(pool);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await?;
    println!("🚀 Server running on http://0.0.0.0:3000");
    println!("📊 Health check: http://0.0.0.0:3000/health");
    println!("📚 API info: http://0.0.0.0:3000/api");

    axum::serve(listener, app).await?;

    Ok(())
}

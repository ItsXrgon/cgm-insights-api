use sqlx::postgres::Postgres;
use sqlx_tracing::Pool as TracingPool;

/// Traced database pool for Sentry query tracking. Wraps sqlx with tracing spans.
pub type DbPool = TracingPool<Postgres>;

pub async fn connect(database_url: &str) -> DbPool {
    let pool = sqlx::postgres::PgPoolOptions::new()
        .connect(database_url)
        .await
        .expect("❌ Failed to connect to database");
    TracingPool::from(pool)
}

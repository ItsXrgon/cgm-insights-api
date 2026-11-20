use sqlx::migrate::Migrator;
use sqlx::postgres::{PgPool, PgPoolOptions, Postgres};
use sqlx_tracing::Pool as TracingPool;

/// Traced database pool for Sentry query tracking. Wraps sqlx with tracing spans.
pub type DbPool = TracingPool<Postgres>;

/// Migrations embedded at compile time from `migrations/`.
pub static MIGRATOR: Migrator = sqlx::migrate!();

/// Connect and bring the schema up to date. Safe to run from several processes at
/// once: sqlx serializes migrations with a Postgres advisory lock.
pub async fn connect(database_url: &str) -> anyhow::Result<DbPool> {
    let pool = PgPoolOptions::new().connect(database_url).await?;
    migrate(&pool).await?;
    Ok(TracingPool::from(pool))
}

pub async fn migrate(pool: &PgPool) -> anyhow::Result<()> {
    MIGRATOR.run(pool).await?;
    Ok(())
}

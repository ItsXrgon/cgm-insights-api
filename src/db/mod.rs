use sqlx::postgres::PgPoolOptions;

pub async fn get_db_pool(database_url: &str) -> sqlx::Pool<sqlx::Postgres> {
    PgPoolOptions::new()
        .max_connections(5)
        .connect(database_url)
        .await
        .expect("❌ Failed to connect to database")
}

pub async fn connect(database_url: &str) -> sqlx::Pool<sqlx::Postgres> {
    sqlx::postgres::PgPoolOptions::new()
        .connect(database_url)
        .await
        .expect("❌ Failed to connect to database")
}

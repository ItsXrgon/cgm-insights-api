use crate::error::AppError;
use crate::models::{GlucoseReading, NewGlucoseReading};
use sqlx::{Pool, Postgres};

/// Initialize the glucose_readings table if it doesn't exist
pub async fn init_table(pool: &Pool<Postgres>) -> Result<(), AppError> {
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS glucose_readings (
            id SERIAL PRIMARY KEY,
            value_mg_dl DOUBLE PRECISION NOT NULL,
            timestamp TIMESTAMPTZ NOT NULL,
            device_id VARCHAR(255),
            notes TEXT,
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
        )
        "#,
    )
    .execute(pool)
    .await?;

    // Create index on timestamp for faster queries
    sqlx::query(
        r#"
        CREATE INDEX IF NOT EXISTS idx_glucose_readings_timestamp 
        ON glucose_readings(timestamp DESC)
        "#,
    )
    .execute(pool)
    .await?;

    Ok(())
}

/// Insert a new glucose reading
pub async fn insert(
    pool: &Pool<Postgres>,
    reading: NewGlucoseReading,
) -> Result<GlucoseReading, AppError> {
    let record = sqlx::query_as::<_, GlucoseReading>(
        r#"
        INSERT INTO glucose_readings (value_mg_dl, timestamp, device_id, notes)
        VALUES ($1, $2, $3, $4)
        RETURNING id, value_mg_dl, timestamp, device_id, notes, created_at
        "#,
    )
    .bind(reading.value_mg_dl)
    .bind(reading.timestamp)
    .bind(reading.device_id)
    .bind(reading.notes)
    .fetch_one(pool)
    .await?;

    Ok(record)
}

/// Get all glucose readings with optional limit
pub async fn find_all(
    pool: &Pool<Postgres>,
    limit: Option<i64>,
) -> Result<Vec<GlucoseReading>, AppError> {
    let limit = limit.unwrap_or(100).min(1000); // Default 100, max 1000

    let records = sqlx::query_as::<_, GlucoseReading>(
        r#"
        SELECT id, value_mg_dl, timestamp, device_id, notes, created_at
        FROM glucose_readings
        ORDER BY timestamp DESC
        LIMIT $1
        "#,
    )
    .bind(limit)
    .fetch_all(pool)
    .await?;

    Ok(records)
}

/// Get glucose reading by ID
pub async fn find_by_id(
    pool: &Pool<Postgres>,
    id: i32,
) -> Result<Option<GlucoseReading>, AppError> {
    let record = sqlx::query_as::<_, GlucoseReading>(
        r#"
        SELECT id, value_mg_dl, timestamp, device_id, notes, created_at
        FROM glucose_readings
        WHERE id = $1
        "#,
    )
    .bind(id)
    .fetch_optional(pool)
    .await?;

    Ok(record)
}

/// Delete a glucose reading
pub async fn delete(pool: &Pool<Postgres>, id: i32) -> Result<bool, AppError> {
    let result = sqlx::query(
        r#"
        DELETE FROM glucose_readings
        WHERE id = $1
        "#,
    )
    .bind(id)
    .execute(pool)
    .await?;

    Ok(result.rows_affected() > 0)
}

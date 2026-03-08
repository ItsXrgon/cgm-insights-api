use crate::error::AppError;
use crate::models::{GlucoseReading, NewGlucoseReading};
use crate::DbPool;
use sqlx::{Postgres, QueryBuilder};
use tracing::instrument;

/// Initialize the glucose_readings table if it doesn't exist
#[instrument(skip(pool))]
pub async fn init_table(pool: &DbPool) -> Result<(), AppError> {
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS glucose_readings (
            id SERIAL PRIMARY KEY,
            user_id INTEGER REFERENCES users(id) ON DELETE CASCADE,
            value_mg_dl DOUBLE PRECISION NOT NULL,
            timestamp TIMESTAMPTZ NOT NULL,
            is_high BOOLEAN NOT NULL DEFAULT FALSE,
            is_low BOOLEAN NOT NULL DEFAULT FALSE,
            trend VARCHAR(50),
            device_id VARCHAR(255),
            notes TEXT,
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            CONSTRAINT glucose_readings_user_timestamp_unique UNIQUE (user_id, timestamp)
        )
        "#,
    )
    .execute(pool)
    .await?;

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

/// Insert a new glucose reading (upsert: on conflict overrides existing row)
#[instrument(skip(pool))]
pub async fn insert(
    pool: &DbPool,
    reading: NewGlucoseReading,
) -> Result<GlucoseReading, AppError> {
    let record = sqlx::query_as::<_, GlucoseReading>(
        r#"
        INSERT INTO glucose_readings (user_id, value_mg_dl, timestamp, is_high, is_low, trend, device_id, notes)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
        ON CONFLICT (user_id, timestamp) DO UPDATE SET
            value_mg_dl = EXCLUDED.value_mg_dl,
            is_high = EXCLUDED.is_high,
            is_low = EXCLUDED.is_low,
            trend = EXCLUDED.trend,
            device_id = EXCLUDED.device_id,
            notes = EXCLUDED.notes
        RETURNING id, user_id, value_mg_dl, timestamp, is_high, is_low, trend, device_id, notes, created_at
        "#,
    )
    .bind(reading.user_id)
    .bind(reading.value_mg_dl)
    .bind(reading.timestamp)
    .bind(reading.is_high)
    .bind(reading.is_low)
    .bind(reading.trend)
    .bind(reading.device_id)
    .bind(reading.notes)
    .fetch_one(pool)
    .await?;

    Ok(record)
}

/// Insert multiple glucose readings (bulk upsert: on conflict overrides existing rows).
/// Deduplicates by (user_id, timestamp) within the batch to avoid PostgreSQL error
/// "cannot affect row a second time" when the API returns duplicate timestamps.
#[instrument(skip(pool, readings))]
pub async fn insert_many(
    pool: &DbPool,
    readings: Vec<NewGlucoseReading>,
) -> Result<u64, AppError> {
    if readings.is_empty() {
        return Ok(0);
    }

    // Keep last occurrence per (user_id, timestamp) so we don't conflict within the same INSERT
    let mut seen = std::collections::HashMap::new();
    for r in readings {
        let key = (r.user_id, r.timestamp.timestamp_millis());
        seen.insert(key, r);
    }
    let readings: Vec<NewGlucoseReading> = seen.into_values().collect();

    if readings.is_empty() {
        return Ok(0);
    }

    let mut query_builder: QueryBuilder<Postgres> = QueryBuilder::new(
        "INSERT INTO glucose_readings (user_id, value_mg_dl, timestamp, is_high, is_low, trend, device_id, notes) ",
    );

    query_builder.push_values(readings, |mut b, reading| {
        b.push_bind(reading.user_id)
            .push_bind(reading.value_mg_dl)
            .push_bind(reading.timestamp)
            .push_bind(reading.is_high)
            .push_bind(reading.is_low)
            .push_bind(reading.trend)
            .push_bind(reading.device_id)
            .push_bind(reading.notes);
    });

    query_builder.push(
        " ON CONFLICT (user_id, timestamp) DO UPDATE SET \
         value_mg_dl = EXCLUDED.value_mg_dl, \
         is_high = EXCLUDED.is_high, \
         is_low = EXCLUDED.is_low, \
         trend = EXCLUDED.trend, \
         device_id = EXCLUDED.device_id, \
         notes = EXCLUDED.notes",
    );

    let result = query_builder.build().execute(pool).await?;
    Ok(result.rows_affected())
}

/// Get all glucose readings for a user with optional limit
#[instrument(skip(pool))]
pub async fn find_all(
    pool: &DbPool,
    user_id: i32,
    limit: Option<i64>,
) -> Result<Vec<GlucoseReading>, AppError> {
    let limit = limit.unwrap_or(100).min(1000); // Default 100, max 1000

    let records = sqlx::query_as::<_, GlucoseReading>(
        r#"
        SELECT id, user_id, value_mg_dl, timestamp, is_high, is_low, trend, device_id, notes, created_at
        FROM glucose_readings
        WHERE user_id = $1
        ORDER BY timestamp DESC
        LIMIT $2
        "#,
    )
    .bind(user_id)
    .bind(limit)
    .fetch_all(pool)
    .await?;

    Ok(records)
}

/// Get glucose reading by ID and user_id
#[instrument(skip(pool))]
pub async fn find_by_id(
    pool: &DbPool,
    user_id: i32,
    id: i32,
) -> Result<Option<GlucoseReading>, AppError> {
    let record = sqlx::query_as::<_, GlucoseReading>(
        r#"
        SELECT id, user_id, value_mg_dl, timestamp, is_high, is_low, trend, device_id, notes, created_at
        FROM glucose_readings
        WHERE id = $1 AND user_id = $2
        "#,
    )
    .bind(id)
    .bind(user_id)
    .fetch_optional(pool)
    .await?;

    Ok(record)
}

/// Delete a glucose reading
#[instrument(skip(pool))]
pub async fn delete(pool: &DbPool, user_id: i32, id: i32) -> Result<bool, AppError> {
    let result = sqlx::query(
        r#"
        DELETE FROM glucose_readings
        WHERE id = $1 AND user_id = $2
        "#,
    )
    .bind(id)
    .bind(user_id)
    .execute(pool)
    .await?;

    Ok(result.rows_affected() > 0)
}

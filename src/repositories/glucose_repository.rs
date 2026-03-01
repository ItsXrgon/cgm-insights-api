use crate::error::AppError;
use crate::models::{GlucoseReading, NewGlucoseReading};
use sqlx::{Pool, Postgres, QueryBuilder};

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

    // Add user_id if it doesn't exist
    sqlx::query(
        r#"
        ALTER TABLE glucose_readings 
        ADD COLUMN IF NOT EXISTS user_id INTEGER REFERENCES users(id) ON DELETE CASCADE
        "#,
    )
    .execute(pool)
    .await?;

    // Update unique constraint
    sqlx::query(
        r#"
        ALTER TABLE glucose_readings 
        DROP CONSTRAINT IF EXISTS glucose_readings_timestamp_key
        "#,
    )
    .execute(pool)
    .await?;

    sqlx::query(
        r#"
        ALTER TABLE glucose_readings 
        ADD CONSTRAINT glucose_readings_user_timestamp_unique UNIQUE (user_id, timestamp)
        "#,
    )
    .execute(pool)
    .await
    .ok(); // Ignore if constraint already exists

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

/// Insert a new glucose reading (with upsert/skip on conflict)
pub async fn insert(
    pool: &Pool<Postgres>,
    reading: NewGlucoseReading,
) -> Result<Option<GlucoseReading>, AppError> {
    let record = sqlx::query_as::<_, GlucoseReading>(
        r#"
        INSERT INTO glucose_readings (user_id, value_mg_dl, timestamp, device_id, notes)
        VALUES ($1, $2, $3, $4, $5)
        ON CONFLICT (user_id, timestamp) DO NOTHING
        RETURNING id, user_id, value_mg_dl, timestamp, device_id, notes, created_at
        "#,
    )
    .bind(reading.user_id)
    .bind(reading.value_mg_dl)
    .bind(reading.timestamp)
    .bind(reading.device_id)
    .bind(reading.notes)
    .fetch_optional(pool)
    .await?;

    Ok(record)
}

/// Insert multiple glucose readings (bulk insert)
pub async fn insert_many(
    pool: &Pool<Postgres>,
    readings: Vec<NewGlucoseReading>,
) -> Result<u64, AppError> {
    if readings.is_empty() {
        return Ok(0);
    }

    let mut query_builder: QueryBuilder<Postgres> = QueryBuilder::new(
        "INSERT INTO glucose_readings (user_id, value_mg_dl, timestamp, device_id, notes) ",
    );

    query_builder.push_values(readings, |mut b, reading| {
        b.push_bind(reading.user_id)
            .push_bind(reading.value_mg_dl)
            .push_bind(reading.timestamp)
            .push_bind(reading.device_id)
            .push_bind(reading.notes);
    });

    query_builder.push(" ON CONFLICT (user_id, timestamp) DO NOTHING");

    let result = query_builder.build().execute(pool).await?;
    Ok(result.rows_affected())
}

/// Get all glucose readings for a user with optional limit
pub async fn find_all(
    pool: &Pool<Postgres>,
    user_id: i32,
    limit: Option<i64>,
) -> Result<Vec<GlucoseReading>, AppError> {
    let limit = limit.unwrap_or(100).min(1000); // Default 100, max 1000

    let records = sqlx::query_as::<_, GlucoseReading>(
        r#"
        SELECT id, user_id, value_mg_dl, timestamp, device_id, notes, created_at
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
pub async fn find_by_id(
    pool: &Pool<Postgres>,
    user_id: i32,
    id: i32,
) -> Result<Option<GlucoseReading>, AppError> {
    let record = sqlx::query_as::<_, GlucoseReading>(
        r#"
        SELECT id, user_id, value_mg_dl, timestamp, device_id, notes, created_at
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
pub async fn delete(pool: &Pool<Postgres>, user_id: i32, id: i32) -> Result<bool, AppError> {
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

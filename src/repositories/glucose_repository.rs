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
pub async fn insert(pool: &DbPool, reading: NewGlucoseReading) -> Result<GlucoseReading, AppError> {
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

/// Keep the last reading per (user_id, timestamp). Postgres rejects an upsert batch that
/// touches the same row twice ("cannot affect row a second time"), and providers do
/// return duplicates (e.g. LibreLink's "current" reading is also in its history).
/// Output is sorted by timestamp for deterministic statements.
pub fn dedupe_by_user_and_timestamp(readings: Vec<NewGlucoseReading>) -> Vec<NewGlucoseReading> {
    let mut by_key = std::collections::HashMap::new();
    for r in readings {
        // timestamptz stores microseconds, so that's the DB's notion of "same timestamp"
        by_key.insert((r.user_id, r.timestamp.timestamp_micros()), r);
    }
    let mut out: Vec<NewGlucoseReading> = by_key.into_values().collect();
    out.sort_by_key(|r| (r.timestamp, r.user_id));
    out
}

const INSERT_CHUNK_ROWS: usize = 1_000;

/// Insert multiple glucose readings (bulk upsert: on conflict overrides existing rows).
/// Returns rows inserted or updated.
#[instrument(skip(pool, readings))]
pub async fn insert_many(pool: &DbPool, readings: Vec<NewGlucoseReading>) -> Result<u64, AppError> {
    if readings.is_empty() {
        return Ok(0);
    }

    let readings = dedupe_by_user_and_timestamp(readings);

    let mut affected = 0;
    // 8 binds per row; Postgres caps a statement at 65_535 parameters.
    for chunk in readings.chunks(INSERT_CHUNK_ROWS) {
        let mut query_builder: QueryBuilder<Postgres> = QueryBuilder::new(
            "INSERT INTO glucose_readings (user_id, value_mg_dl, timestamp, is_high, is_low, trend, device_id, notes) ",
        );

        query_builder.push_values(chunk, |mut b, reading| {
            b.push_bind(reading.user_id)
                .push_bind(reading.value_mg_dl)
                .push_bind(reading.timestamp)
                .push_bind(reading.is_high)
                .push_bind(reading.is_low)
                .push_bind(reading.trend.clone())
                .push_bind(reading.device_id.clone())
                .push_bind(reading.notes.clone());
        });

        query_builder.push(
            " ON CONFLICT (user_id, timestamp) DO UPDATE SET              value_mg_dl = EXCLUDED.value_mg_dl,              is_high = EXCLUDED.is_high,              is_low = EXCLUDED.is_low,              trend = EXCLUDED.trend,              device_id = EXCLUDED.device_id,              notes = EXCLUDED.notes",
        );

        affected += query_builder.build().execute(pool).await?.rows_affected();
    }
    Ok(affected)
}

/// Most recent glucose readings for a user, newest first
#[instrument(skip(pool))]
pub async fn find_all(
    pool: &DbPool,
    user_id: i32,
    limit: i64,
) -> Result<Vec<GlucoseReading>, AppError> {
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

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{TimeZone, Utc};

    fn reading(user_id: i32, secs: i64, value: f64) -> NewGlucoseReading {
        NewGlucoseReading {
            user_id: Some(user_id),
            value_mg_dl: value,
            timestamp: Utc.timestamp_opt(secs, 0).unwrap(),
            is_high: false,
            is_low: false,
            trend: None,
            device_id: None,
            notes: None,
        }
    }

    #[test]
    fn dedupe_keeps_last_per_user_and_timestamp() {
        let out = dedupe_by_user_and_timestamp(vec![
            reading(1, 100, 90.0),
            reading(1, 200, 95.0),
            reading(1, 100, 99.0),  // duplicate of first: last wins
            reading(2, 100, 120.0), // same time, other user: kept
        ]);
        assert_eq!(out.len(), 3);
        let u1_t100 = out
            .iter()
            .find(|r| r.user_id == Some(1) && r.timestamp.timestamp() == 100)
            .unwrap();
        assert_eq!(u1_t100.value_mg_dl, 99.0);
        assert!(out.windows(2).all(|w| w[0].timestamp <= w[1].timestamp));
    }

    #[test]
    fn dedupe_distinguishes_sub_second_timestamps() {
        let a = reading(1, 100, 90.0);
        let mut b = reading(1, 100, 91.0);
        b.timestamp += chrono::Duration::milliseconds(500);
        assert_eq!(dedupe_by_user_and_timestamp(vec![a, b]).len(), 2);
    }

    #[test]
    fn dedupe_empty() {
        assert!(dedupe_by_user_and_timestamp(vec![]).is_empty());
    }
}

use crate::dto::{CreateGlucoseReadingRequest, GlucoseReadingResponse};
use crate::error::AppError;
use crate::models::{GlucoseReading, NewGlucoseReading};
use crate::repositories::glucose_repository;
use sqlx::{Pool, Postgres};

/// Convert model to response DTO
fn to_response(reading: GlucoseReading) -> GlucoseReadingResponse {
    GlucoseReadingResponse {
        id: reading.id,
        value_mg_dl: reading.value_mg_dl,
        timestamp: reading.timestamp.to_rfc3339(),
        device_id: reading.device_id,
        notes: reading.notes,
        created_at: reading.created_at.to_rfc3339(),
    }
}

/// Create a new glucose reading
pub async fn create_reading(
    pool: &Pool<Postgres>,
    request: CreateGlucoseReadingRequest,
) -> Result<GlucoseReadingResponse, AppError> {
    // Validation
    if request.value_mg_dl < 0.0 || request.value_mg_dl > 1000.0 {
        return Err(AppError::ConfigError(
            "Glucose value must be between 0 and 1000 mg/dL".to_string(),
        ));
    }

    let new_reading = NewGlucoseReading {
        value_mg_dl: request.value_mg_dl,
        timestamp: request.timestamp,
        device_id: request.device_id,
        notes: request.notes,
    };

    let reading = glucose_repository::insert(pool, new_reading).await?;

    Ok(to_response(reading))
}

/// Get all glucose readings
pub async fn get_readings(
    pool: &Pool<Postgres>,
    limit: Option<i64>,
) -> Result<Vec<GlucoseReadingResponse>, AppError> {
    let readings = glucose_repository::find_all(pool, limit).await?;

    Ok(readings.into_iter().map(to_response).collect())
}

/// Get glucose reading by ID
pub async fn get_reading_by_id(
    pool: &Pool<Postgres>,
    id: i32,
) -> Result<GlucoseReadingResponse, AppError> {
    let reading = glucose_repository::find_by_id(pool, id)
        .await?
        .ok_or_else(|| AppError::ConfigError("Reading not found".to_string()))?;

    Ok(to_response(reading))
}

/// Delete a glucose reading
pub async fn delete_reading(pool: &Pool<Postgres>, id: i32) -> Result<bool, AppError> {
    glucose_repository::delete(pool, id).await
}

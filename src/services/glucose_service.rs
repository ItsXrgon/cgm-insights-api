use crate::dto::{CreateGlucoseReadingRequest, GlucoseReadingResponse};
use crate::error::AppError;
use crate::models::{GlucoseReading, NewGlucoseReading};
use crate::repositories::glucose_repository;
use crate::DbPool;

/// Convert model to response DTO
fn to_response(reading: GlucoseReading) -> GlucoseReadingResponse {
    GlucoseReadingResponse {
        id: reading.id,
        value_mg_dl: reading.value_mg_dl,
        timestamp: reading.timestamp.to_rfc3339(),
        is_high: reading.is_high,
        is_low: reading.is_low,
        trend: reading.trend,
        device_id: reading.device_id,
        notes: reading.notes,
        created_at: reading.created_at.to_rfc3339(),
    }
}

/// Create a new glucose reading
pub async fn create_reading(
    pool: &DbPool,
    user_id: i32,
    request: CreateGlucoseReadingRequest,
) -> Result<GlucoseReadingResponse, AppError> {
    // Validation
    if request.value_mg_dl < 0.0 || request.value_mg_dl > 1000.0 {
        return Err(AppError::ConfigError(
            "Glucose value must be between 0 and 1000 mg/dL".to_string(),
        ));
    }

    let new_reading = NewGlucoseReading {
        user_id: Some(user_id),
        value_mg_dl: request.value_mg_dl,
        timestamp: request.timestamp,
        is_high: request.is_high,
        is_low: request.is_low,
        trend: request.trend,
        device_id: request.device_id,
        notes: request.notes,
    };

    let reading = glucose_repository::insert(pool, new_reading).await?;
    Ok(to_response(reading))
}

/// Get all glucose readings for a user
pub async fn get_readings(
    pool: &DbPool,
    user_id: i32,
    limit: Option<i64>,
) -> Result<Vec<GlucoseReadingResponse>, AppError> {
    let readings = glucose_repository::find_all(pool, user_id, limit).await?;

    Ok(readings.into_iter().map(to_response).collect())
}

/// Get glucose reading by ID and user_id
pub async fn get_reading_by_id(
    pool: &DbPool,
    user_id: i32,
    id: i32,
) -> Result<GlucoseReadingResponse, AppError> {
    let reading = glucose_repository::find_by_id(pool, user_id, id)
        .await?
        .ok_or_else(|| AppError::ConfigError("Reading not found".to_string()))?;

    Ok(to_response(reading))
}

/// Delete a glucose reading
pub async fn delete_reading(
    pool: &DbPool,
    user_id: i32,
    id: i32,
) -> Result<bool, AppError> {
    glucose_repository::delete(pool, user_id, id).await
}

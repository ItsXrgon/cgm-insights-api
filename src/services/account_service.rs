use crate::dto::{AccountExport, UserResponse};
use crate::error::AppError;
use crate::repositories::{cgm_repository, glucose_repository, user_repository};
use crate::services::{cgm_service, glucose_service, verify_password};
use crate::DbPool;
use chrono::Utc;

fn user_gone() -> AppError {
    AppError::NotFound("Account not found".to_string())
}

/// Everything stored for the user, for data portability. CGM passwords are never included.
pub async fn export(pool: &DbPool, user_id: i32) -> Result<AccountExport, AppError> {
    let user = user_repository::find_by_id(pool, user_id)
        .await?
        .ok_or_else(user_gone)?;
    let credentials = cgm_repository::find_by_user_id(pool, user_id).await?;
    let readings = glucose_repository::find_all_for_user(pool, user_id).await?;

    Ok(AccountExport {
        exported_at: Utc::now().to_rfc3339(),
        user: UserResponse {
            id: user.id,
            username: user.username,
            created_at: user.created_at.to_rfc3339(),
        },
        cgm_credentials: credentials
            .into_iter()
            .map(cgm_service::to_response)
            .collect(),
        glucose_readings: readings
            .into_iter()
            .map(glucose_service::to_response)
            .collect(),
    })
}

/// Permanently delete the account, its CGM credentials and readings.
/// Requires the account password so a stolen token alone can't wipe the data.
pub async fn delete(pool: &DbPool, user_id: i32, password: &str) -> Result<(), AppError> {
    let user = user_repository::find_by_id(pool, user_id)
        .await?
        .ok_or_else(user_gone)?;
    if !verify_password(password, &user.password_hash)? {
        return Err(AppError::AuthError("Password is incorrect".to_string()));
    }
    if user_repository::delete(pool, user_id).await? {
        Ok(())
    } else {
        Err(user_gone())
    }
}

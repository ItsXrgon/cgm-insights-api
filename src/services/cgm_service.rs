use crate::dto::{CgmCredentialResponse, CreateCgmCredentialRequest, UpdateCgmCredentialRequest};
use crate::error::AppError;
use crate::models::{CgmCredential, NewCgmCredential};
use crate::repositories::cgm_repository;
use sqlx::{Pool, Postgres};

fn to_response(cred: CgmCredential) -> CgmCredentialResponse {
    CgmCredentialResponse {
        id: cred.id,
        user_id: cred.user_id,
        cgm_type: cred.cgm_type,
        username: cred.username,
        region: cred.region,
        is_active: cred.is_active,
        created_at: cred.created_at.to_rfc3339(),
        updated_at: cred.updated_at.to_rfc3339(),
    }
}

pub async fn list_credentials(
    pool: &Pool<Postgres>,
    user_id: i32,
) -> Result<Vec<CgmCredentialResponse>, AppError> {
    let creds = cgm_repository::find_by_user_id(pool, user_id).await?;
    Ok(creds.into_iter().map(to_response).collect())
}

pub async fn add_credential(
    pool: &Pool<Postgres>,
    user_id: i32,
    request: CreateCgmCredentialRequest,
) -> Result<CgmCredentialResponse, AppError> {
    if request.is_active {
        cgm_repository::deactivate_all_for_user(pool, user_id).await?;
    }

    let new_cred = NewCgmCredential {
        user_id,
        cgm_type: request.cgm_type,
        username: request.cgm_username,
        password: request.cgm_password,
        region: request.cgm_region,
        is_active: request.is_active,
    };

    let cred = cgm_repository::insert(pool, new_cred).await?;
    Ok(to_response(cred))
}

pub async fn update_credential(
    pool: &Pool<Postgres>,
    user_id: i32,
    credential_id: i32,
    request: UpdateCgmCredentialRequest,
) -> Result<CgmCredentialResponse, AppError> {
    // Verify ownership
    let existing = cgm_repository::find_by_id(pool, credential_id)
        .await?
        .ok_or_else(|| AppError::ConfigError("Credential not found".to_string()))?;

    if existing.user_id != user_id {
        return Err(AppError::AuthError(
            "Unauthorized access to credential".to_string(),
        ));
    }

    if let Some(true) = request.is_active {
        cgm_repository::deactivate_all_for_user(pool, user_id).await?;
    }

    let updated = cgm_repository::update(
        pool,
        credential_id,
        request.cgm_type,
        request.cgm_username,
        request.cgm_password,
        request.cgm_region,
        request.is_active,
    )
    .await?;

    Ok(to_response(updated))
}

pub async fn set_active_credential(
    pool: &Pool<Postgres>,
    user_id: i32,
    credential_id: i32,
) -> Result<CgmCredentialResponse, AppError> {
    // Verify ownership
    let existing = cgm_repository::find_by_id(pool, credential_id)
        .await?
        .ok_or_else(|| AppError::ConfigError("Credential not found".to_string()))?;

    if existing.user_id != user_id {
        return Err(AppError::AuthError(
            "Unauthorized access to credential".to_string(),
        ));
    }

    cgm_repository::deactivate_all_for_user(pool, user_id).await?;
    let updated =
        cgm_repository::update(pool, credential_id, None, None, None, None, Some(true)).await?;

    Ok(to_response(updated))
}

pub async fn delete_credential(
    pool: &Pool<Postgres>,
    user_id: i32,
    credential_id: i32,
) -> Result<bool, AppError> {
    // Verify ownership
    let existing = cgm_repository::find_by_id(pool, credential_id)
        .await?
        .ok_or_else(|| AppError::ConfigError("Credential not found".to_string()))?;

    if existing.user_id != user_id {
        return Err(AppError::AuthError(
            "Unauthorized access to credential".to_string(),
        ));
    }

    cgm_repository::delete(pool, credential_id).await
}

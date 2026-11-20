use crate::error::AppError;
use crate::models::{NewCgmCredential, NewUser, User};
use crate::DbPool;
use tracing::instrument;

fn map_unique_username(e: sqlx::Error) -> AppError {
    match &e {
        sqlx::Error::Database(db) if db.is_unique_violation() => {
            AppError::Conflict("Username already exists".to_string())
        }
        _ => AppError::from(e),
    }
}

/// Create a user together with their first CGM credential in one statement, so a
/// failure can never leave a user without a credential. `credential.user_id` is ignored.
#[instrument(skip(pool, user, credential))]
pub async fn insert_with_credential(
    pool: &DbPool,
    user: NewUser,
    credential: NewCgmCredential,
) -> Result<User, AppError> {
    let record = sqlx::query_as::<_, User>(
        r#"
        WITH new_user AS (
            INSERT INTO users (username, password_hash)
            VALUES ($1, $2)
            RETURNING id, username, password_hash, created_at, updated_at
        ),
        new_credential AS (
            INSERT INTO cgm_credentials (user_id, cgm_type, username, password, region, is_active)
            SELECT id, $3, $4, $5, $6, $7 FROM new_user
        )
        SELECT id, username, password_hash, created_at, updated_at FROM new_user
        "#,
    )
    .bind(user.username)
    .bind(user.password_hash)
    .bind(credential.cgm_type)
    .bind(credential.username)
    .bind(credential.password)
    .bind(credential.region)
    .bind(credential.is_active)
    .fetch_one(pool)
    .await
    .map_err(map_unique_username)?;

    Ok(record)
}

/// Delete a user; credentials and readings go with it (ON DELETE CASCADE)
#[instrument(skip(pool))]
pub async fn delete(pool: &DbPool, id: i32) -> Result<bool, AppError> {
    let result = sqlx::query("DELETE FROM users WHERE id = $1")
        .bind(id)
        .execute(pool)
        .await?;

    Ok(result.rows_affected() > 0)
}

/// Find user by username
#[instrument(skip(pool))]
pub async fn find_by_username(pool: &DbPool, username: &str) -> Result<Option<User>, AppError> {
    let record = sqlx::query_as::<_, User>(
        r#"
        SELECT id, username, password_hash, created_at, updated_at
        FROM users
        WHERE username = $1
        "#,
    )
    .bind(username)
    .fetch_optional(pool)
    .await?;

    Ok(record)
}

/// Find user by ID
#[instrument(skip(pool))]
pub async fn find_by_id(pool: &DbPool, id: i32) -> Result<Option<User>, AppError> {
    let record = sqlx::query_as::<_, User>(
        r#"
        SELECT id, username, password_hash, created_at, updated_at
        FROM users
        WHERE id = $1
        "#,
    )
    .bind(id)
    .fetch_optional(pool)
    .await?;

    Ok(record)
}

use crate::error::AppError;
use crate::models::{CgmCredential, NewCgmCredential};
use crate::DbPool;
use tracing::instrument;

/// Initialize the cgm_credentials table if it doesn't exist
#[instrument(skip(pool))]
pub async fn init_table(pool: &DbPool) -> Result<(), AppError> {
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS cgm_credentials (
            id SERIAL PRIMARY KEY,
            user_id INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
            cgm_type VARCHAR(50) NOT NULL,
            username VARCHAR(255) NOT NULL,
            password VARCHAR(255) NOT NULL,
            region VARCHAR(50),
            is_active BOOLEAN NOT NULL DEFAULT FALSE,
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
        )
        "#,
    )
    .execute(pool)
    .await?;

    Ok(())
}

/// Insert a new CGM credential
#[instrument(skip(pool, credential))]
pub async fn insert(
    pool: &DbPool,
    credential: NewCgmCredential,
) -> Result<CgmCredential, AppError> {
    let record = sqlx::query_as::<_, CgmCredential>(
        r#"
        INSERT INTO cgm_credentials (user_id, cgm_type, username, password, region, is_active)
        VALUES ($1, $2, $3, $4, $5, $6)
        RETURNING id, user_id, cgm_type, username, password, region, is_active, created_at, updated_at
        "#,
    )
    .bind(credential.user_id)
    .bind(credential.cgm_type)
    .bind(credential.username)
    .bind(credential.password)
    .bind(credential.region)
    .bind(credential.is_active)
    .fetch_one(pool)
    .await?;

    Ok(record)
}

/// Find active CGM credentials for all users
#[instrument(skip(pool))]
pub async fn find_all_active(pool: &DbPool) -> Result<Vec<CgmCredential>, AppError> {
    let records = sqlx::query_as::<_, CgmCredential>(
        r#"
        SELECT id, user_id, cgm_type, username, password, region, is_active, created_at, updated_at
        FROM cgm_credentials
        WHERE is_active = TRUE
        "#,
    )
    .fetch_all(pool)
    .await?;

    Ok(records)
}

/// Find all credentials for a specific user
#[instrument(skip(pool))]
pub async fn find_by_user_id(
    pool: &DbPool,
    user_id: i32,
) -> Result<Vec<CgmCredential>, AppError> {
    let records = sqlx::query_as::<_, CgmCredential>(
        r#"
        SELECT id, user_id, cgm_type, username, password, region, is_active, created_at, updated_at
        FROM cgm_credentials
        WHERE user_id = $1
        ORDER BY created_at DESC
        "#,
    )
    .bind(user_id)
    .fetch_all(pool)
    .await?;

    Ok(records)
}

/// Find credential by ID
#[instrument(skip(pool))]
pub async fn find_by_id(pool: &DbPool, id: i32) -> Result<Option<CgmCredential>, AppError> {
    let record = sqlx::query_as::<_, CgmCredential>(
        r#"
        SELECT id, user_id, cgm_type, username, password, region, is_active, created_at, updated_at
        FROM cgm_credentials
        WHERE id = $1
        "#,
    )
    .bind(id)
    .fetch_optional(pool)
    .await?;

    Ok(record)
}

/// Update a CGM credential
#[instrument(skip(pool, username, password))]
pub async fn update(
    pool: &DbPool,
    id: i32,
    cgm_type: Option<String>,
    username: Option<String>,
    password: Option<String>,
    region: Option<String>,
    is_active: Option<bool>,
) -> Result<CgmCredential, AppError> {
    let record = sqlx::query_as::<_, CgmCredential>(
        r#"
        UPDATE cgm_credentials
        SET 
            cgm_type = COALESCE($1, cgm_type),
            username = COALESCE($2, username),
            password = COALESCE($3, password),
            region = COALESCE($4, region),
            is_active = COALESCE($5, is_active),
            updated_at = NOW()
        WHERE id = $6
        RETURNING id, user_id, cgm_type, username, password, region, is_active, created_at, updated_at
        "#,
    )
    .bind(cgm_type)
    .bind(username)
    .bind(password)
    .bind(region)
    .bind(is_active)
    .bind(id)
    .fetch_one(pool)
    .await?;

    Ok(record)
}

/// Make `id` the user's only active credential, in a single statement so concurrent
/// requests can never leave two active rows. Returns `None` if the user doesn't own `id`.
#[instrument(skip(pool))]
pub async fn activate(
    pool: &DbPool,
    user_id: i32,
    id: i32,
) -> Result<Option<CgmCredential>, AppError> {
    let record = sqlx::query_as::<_, CgmCredential>(
        r#"
        WITH target AS (
            SELECT id FROM cgm_credentials WHERE id = $2 AND user_id = $1
        ),
        updated AS (
            -- Touch every row of the user (not just currently-active ones) so a concurrent
            -- activation re-checks each locked row and the last writer wins cleanly.
            UPDATE cgm_credentials c
            SET is_active = (c.id = $2),
                updated_at = CASE WHEN c.is_active = (c.id = $2) THEN c.updated_at ELSE NOW() END
            WHERE c.user_id = $1
              AND EXISTS (SELECT 1 FROM target)
            RETURNING c.id, c.user_id, c.cgm_type, c.username, c.password, c.region, c.is_active, c.created_at, c.updated_at
        )
        SELECT id, user_id, cgm_type, username, password, region, is_active, created_at, updated_at
        FROM updated
        WHERE id = $2
        "#,
    )
    .bind(user_id)
    .bind(id)
    .fetch_optional(pool)
    .await?;

    Ok(record)
}

/// Delete a CGM credential owned by `user_id`
#[instrument(skip(pool))]
pub async fn delete(pool: &DbPool, user_id: i32, id: i32) -> Result<bool, AppError> {
    let result = sqlx::query(
        r#"
        DELETE FROM cgm_credentials
        WHERE id = $1 AND user_id = $2
        "#,
    )
    .bind(id)
    .bind(user_id)
    .execute(pool)
    .await?;

    Ok(result.rows_affected() > 0)
}

use crate::error::AppError;
use crate::models::{CgmCredential, NewCgmCredential};
use crate::DbPool;
use tracing::instrument;

/// Columns mapped by `CgmCredential` (a macro so it can be spliced into `concat!`).
macro_rules! columns {
    () => {
        "id, user_id, cgm_type, username, password, region, is_active, created_at, updated_at, \
         last_sync_at, last_sync_status, last_sync_error"
    };
}

/// Insert a new CGM credential
#[instrument(skip(pool, credential))]
pub async fn insert(
    pool: &DbPool,
    credential: NewCgmCredential,
) -> Result<CgmCredential, AppError> {
    let record = sqlx::query_as::<_, CgmCredential>(concat!(
        "INSERT INTO cgm_credentials (user_id, cgm_type, username, password, region, is_active) \
         VALUES ($1, $2, $3, $4, $5, $6) RETURNING ",
        columns!()
    ))
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
    let records = sqlx::query_as::<_, CgmCredential>(concat!(
        "SELECT ",
        columns!(),
        " FROM cgm_credentials WHERE is_active = TRUE ORDER BY id"
    ))
    .fetch_all(pool)
    .await?;

    Ok(records)
}

/// The user's active credential, if any
#[instrument(skip(pool))]
pub async fn find_active_for_user(
    pool: &DbPool,
    user_id: i32,
) -> Result<Option<CgmCredential>, AppError> {
    let record = sqlx::query_as::<_, CgmCredential>(concat!(
        "SELECT ",
        columns!(),
        " FROM cgm_credentials WHERE user_id = $1 AND is_active = TRUE"
    ))
    .bind(user_id)
    .fetch_optional(pool)
    .await?;

    Ok(record)
}

/// Find all credentials for a specific user
#[instrument(skip(pool))]
pub async fn find_by_user_id(pool: &DbPool, user_id: i32) -> Result<Vec<CgmCredential>, AppError> {
    let records = sqlx::query_as::<_, CgmCredential>(concat!(
        "SELECT ",
        columns!(),
        " FROM cgm_credentials WHERE user_id = $1 ORDER BY created_at DESC"
    ))
    .bind(user_id)
    .fetch_all(pool)
    .await?;

    Ok(records)
}

/// Find credential by ID
#[instrument(skip(pool))]
pub async fn find_by_id(pool: &DbPool, id: i32) -> Result<Option<CgmCredential>, AppError> {
    let record = sqlx::query_as::<_, CgmCredential>(concat!(
        "SELECT ",
        columns!(),
        " FROM cgm_credentials WHERE id = $1"
    ))
    .bind(id)
    .fetch_optional(pool)
    .await?;

    Ok(record)
}

/// Update a CGM credential. Changing what we log in with clears the previous sync
/// outcome (and the cooldown), so a fixed password can be synced immediately.
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
    let record = sqlx::query_as::<_, CgmCredential>(concat!(
        r#"
        WITH login_changed AS (
            SELECT ($1::VARCHAR IS NOT NULL OR $2::VARCHAR IS NOT NULL
                    OR $3::VARCHAR IS NOT NULL OR $4::VARCHAR IS NOT NULL) AS yes
        )
        UPDATE cgm_credentials
        SET
            cgm_type = COALESCE($1, cgm_type),
            username = COALESCE($2, username),
            password = COALESCE($3, password),
            region = COALESCE($4, region),
            is_active = COALESCE($5, is_active),
            last_sync_at = CASE WHEN (SELECT yes FROM login_changed) THEN NULL ELSE last_sync_at END,
            last_sync_status = CASE WHEN (SELECT yes FROM login_changed) THEN NULL ELSE last_sync_status END,
            last_sync_error = CASE WHEN (SELECT yes FROM login_changed) THEN NULL ELSE last_sync_error END,
            updated_at = NOW()
        WHERE id = $6
        RETURNING "#,
        columns!()
    ))
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

/// Make `id` the user's only active credential in a single statement. The
/// `cgm_credentials_one_active_per_user` constraint is checked at the end of the
/// statement, so flipping the old row off and the new one on together is allowed.
/// Returns `None` if the user doesn't own `id`.
#[instrument(skip(pool))]
pub async fn activate(
    pool: &DbPool,
    user_id: i32,
    id: i32,
) -> Result<Option<CgmCredential>, AppError> {
    let record = sqlx::query_as::<_, CgmCredential>(concat!(
        r#"
        WITH updated AS (
            UPDATE cgm_credentials c
            SET is_active = (c.id = $2),
                updated_at = CASE WHEN c.is_active = (c.id = $2) THEN c.updated_at ELSE NOW() END
            WHERE c.user_id = $1
              AND EXISTS (SELECT 1 FROM cgm_credentials t WHERE t.id = $2 AND t.user_id = $1)
            RETURNING c.*
        )
        SELECT "#,
        columns!(),
        " FROM updated WHERE id = $2"
    ))
    .bind(user_id)
    .bind(id)
    .fetch_optional(pool)
    .await?;

    Ok(record)
}

/// Atomically claim a sync slot: stamps `last_sync_at` unless the previous attempt
/// started less than `cooldown_secs` ago. Returns whether the caller may sync.
/// Works across processes and instances, unlike an in-memory limiter.
#[instrument(skip(pool))]
pub async fn claim_sync(pool: &DbPool, id: i32, cooldown_secs: u64) -> Result<bool, AppError> {
    let claimed = sqlx::query_scalar::<_, i32>(
        r#"
        UPDATE cgm_credentials
        SET last_sync_at = NOW()
        WHERE id = $1
          AND (last_sync_at IS NULL OR last_sync_at <= NOW() - make_interval(secs => $2))
        RETURNING id
        "#,
    )
    .bind(id)
    .bind(cooldown_secs as f64)
    .fetch_optional(pool)
    .await?;

    Ok(claimed.is_some())
}

/// Store the outcome of the latest sync attempt
#[instrument(skip(pool, error))]
pub async fn record_sync_result(
    pool: &DbPool,
    id: i32,
    status: &str,
    error: Option<&str>,
) -> Result<(), AppError> {
    sqlx::query(
        "UPDATE cgm_credentials SET last_sync_status = $2, last_sync_error = $3 WHERE id = $1",
    )
    .bind(id)
    .bind(status)
    .bind(error)
    .execute(pool)
    .await?;

    Ok(())
}

/// Delete a CGM credential owned by `user_id`
#[instrument(skip(pool))]
pub async fn delete(pool: &DbPool, user_id: i32, id: i32) -> Result<bool, AppError> {
    let result = sqlx::query("DELETE FROM cgm_credentials WHERE id = $1 AND user_id = $2")
        .bind(id)
        .bind(user_id)
        .execute(pool)
        .await?;

    Ok(result.rows_affected() > 0)
}

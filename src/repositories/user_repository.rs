use crate::error::AppError;
use crate::models::{NewUser, User};
use sqlx::{Pool, Postgres};

/// Initialize the users table if it doesn't exist
pub async fn init_table(pool: &Pool<Postgres>) -> Result<(), AppError> {
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS users (
            id SERIAL PRIMARY KEY,
            username VARCHAR(255) NOT NULL UNIQUE,
            password_hash VARCHAR(255) NOT NULL,
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
        )
        "#,
    )
    .execute(pool)
    .await?;

    Ok(())
}

/// Insert a new user
pub async fn insert(pool: &Pool<Postgres>, user: NewUser) -> Result<User, AppError> {
    let record = sqlx::query_as::<_, User>(
        r#"
        INSERT INTO users (username, password_hash)
        VALUES ($1, $2)
        RETURNING id, username, password_hash, created_at, updated_at
        "#,
    )
    .bind(user.username)
    .bind(user.password_hash)
    .fetch_one(pool)
    .await?;

    Ok(record)
}

/// Find user by username
pub async fn find_by_username(
    pool: &Pool<Postgres>,
    username: &str,
) -> Result<Option<User>, AppError> {
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
pub async fn find_by_id(pool: &Pool<Postgres>, id: i32) -> Result<Option<User>, AppError> {
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

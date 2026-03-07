use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use chrono::{Duration, Utc};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use sqlx::{Pool, Postgres};
use std::env;

use crate::dto::{AuthResponse, LoginRequest, SignupRequest, UserResponse};
use crate::error::AppError;
use crate::models::{NewCgmCredential, NewUser, User};
use crate::repositories::{cgm_repository, user_repository};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    pub sub: i32,
    pub username: String,
    pub exp: i64,
}

pub fn hash_password(password: &str) -> Result<String, AppError> {
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();
    let password_hash = argon2
        .hash_password(password.as_bytes(), &salt)
        .map_err(|e| AppError::InternalError(anyhow::anyhow!("Password hashing failed: {}", e)))?
        .to_string();
    Ok(password_hash)
}

pub fn verify_password(password: &str, hash: &str) -> Result<bool, AppError> {
    let parsed_hash = PasswordHash::new(hash)
        .map_err(|e| AppError::InternalError(anyhow::anyhow!("Invalid password hash: {}", e)))?;
    let argon2 = Argon2::default();
    Ok(argon2
        .verify_password(password.as_bytes(), &parsed_hash)
        .is_ok())
}

pub fn generate_token(user: &User) -> Result<String, AppError> {
    let secret = jwt_secret()?;
    let expiration = Utc::now() + Duration::days(7);

    let claims = Claims {
        sub: user.id,
        username: user.username.clone(),
        exp: expiration.timestamp(),
    };

    let token = encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )
    .map_err(|e| AppError::InternalError(anyhow::anyhow!("Token generation failed: {}", e)))?;

    Ok(token)
}

pub fn validate_token(token: &str) -> Result<Claims, AppError> {
    let secret = jwt_secret()?;

    let token_data = decode::<Claims>(
        token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &Validation::default(),
    )
    .map_err(|_| AppError::AuthError("Invalid or expired token".to_string()))?;

    Ok(token_data.claims)
}

/// Returns JWT_SECRET; in production it must be set and not the default placeholder.
fn jwt_secret() -> Result<String, AppError> {
    let secret = env::var("JWT_SECRET").unwrap_or_else(|_| "secret".to_string());
    let is_production = env::var("APP_ENV")
        .unwrap_or_else(|_| "development".to_string())
        .to_lowercase()
        == "production";
    if is_production && (secret.is_empty() || secret == "secret") {
        return Err(AppError::ConfigError(
            "JWT_SECRET must be set to a strong secret in production".to_string(),
        ));
    }
    Ok(secret)
}

const USERNAME_MIN_LEN: usize = 3;
const USERNAME_MAX_LEN: usize = 64;
const PASSWORD_MIN_LEN: usize = 8;
const PASSWORD_MAX_LEN: usize = 256;
const CGM_CREDENTIAL_MAX_LEN: usize = 512;

fn validate_username(s: &str) -> Result<(), AppError> {
    let len = s.chars().count();
    if len < USERNAME_MIN_LEN {
        return Err(AppError::ValidationError(format!(
            "Username must be at least {} characters",
            USERNAME_MIN_LEN
        )));
    }
    if len > USERNAME_MAX_LEN {
        return Err(AppError::ValidationError(format!(
            "Username must be at most {} characters",
            USERNAME_MAX_LEN
        )));
    }
    Ok(())
}

fn validate_password(s: &str) -> Result<(), AppError> {
    let len = s.chars().count();
    if len < PASSWORD_MIN_LEN {
        return Err(AppError::ValidationError(format!(
            "Password must be at least {} characters",
            PASSWORD_MIN_LEN
        )));
    }
    if len > PASSWORD_MAX_LEN {
        return Err(AppError::ValidationError(format!(
            "Password must be at most {} characters",
            PASSWORD_MAX_LEN
        )));
    }
    Ok(())
}

fn validate_cgm_credentials(username: &str, password: &str) -> Result<(), AppError> {
    if username.chars().count() > CGM_CREDENTIAL_MAX_LEN {
        return Err(AppError::ValidationError(
            "CGM username is too long".to_string(),
        ));
    }
    if password.chars().count() > CGM_CREDENTIAL_MAX_LEN {
        return Err(AppError::ValidationError(
            "CGM password is too long".to_string(),
        ));
    }
    Ok(())
}

pub async fn signup(
    pool: &Pool<Postgres>,
    request: SignupRequest,
) -> Result<AuthResponse, AppError> {
    validate_username(&request.username)?;
    validate_password(&request.password)?;
    validate_cgm_credentials(&request.cgm_username, &request.cgm_password)?;

    // Check if user already exists
    if user_repository::find_by_username(pool, &request.username)
        .await?
        .is_some()
    {
        return Err(AppError::AuthError("Username already exists".to_string()));
    }

    let password_hash = hash_password(&request.password)?;
    let new_user = NewUser {
        username: request.username,
        password_hash,
    };

    let user = user_repository::insert(pool, new_user).await?;

    // Create CGM credential for the user
    let new_cgm = NewCgmCredential {
        user_id: user.id,
        cgm_type: request.cgm_type,
        username: request.cgm_username,
        password: request.cgm_password,
        region: request.cgm_region.map(|r| r.to_lowercase()),
        is_active: true,
    };
    cgm_repository::insert(pool, new_cgm).await?;

    let token = generate_token(&user)?;

    Ok(AuthResponse {
        token,
        user: UserResponse {
            id: user.id,
            username: user.username,
            created_at: user.created_at.to_rfc3339(),
        },
    })
}

pub async fn login(pool: &Pool<Postgres>, request: LoginRequest) -> Result<AuthResponse, AppError> {
    validate_username(&request.username)?;
    validate_password(&request.password)?;

    let user = user_repository::find_by_username(pool, &request.username)
        .await?
        .ok_or_else(|| AppError::AuthError("Invalid username or password".to_string()))?;

    if !verify_password(&request.password, &user.password_hash)? {
        return Err(AppError::AuthError(
            "Invalid username or password".to_string(),
        ));
    }

    let token = generate_token(&user)?;

    Ok(AuthResponse {
        token,
        user: UserResponse {
            id: user.id,
            username: user.username,
            created_at: user.created_at.to_rfc3339(),
        },
    })
}

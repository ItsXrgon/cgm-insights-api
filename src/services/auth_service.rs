use crate::DbPool;
use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use chrono::{Duration, Utc};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};

use crate::dto::{AuthResponse, LoginRequest, SignupRequest, UserResponse};
use crate::error::AppError;
use crate::models::{NewUser, User};
use crate::repositories::user_repository;
use crate::services::cgm_service;

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

pub fn generate_token(user: &User, secret: &str) -> Result<String, AppError> {
    let claims = Claims {
        sub: user.id,
        username: user.username.clone(),
        exp: (Utc::now() + Duration::days(7)).timestamp(),
    };

    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )
    .map_err(|e| AppError::InternalError(anyhow::anyhow!("Token generation failed: {}", e)))
}

pub fn validate_token(token: &str, secret: &str) -> Result<Claims, AppError> {
    decode::<Claims>(
        token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &Validation::default(),
    )
    .map(|data| data.claims)
    .map_err(|_| AppError::AuthError("Invalid or expired token".to_string()))
}

const USERNAME_MIN_LEN: usize = 3;
const USERNAME_MAX_LEN: usize = 64;
const PASSWORD_MIN_LEN: usize = 8;
const PASSWORD_MAX_LEN: usize = 256;

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

pub async fn signup(
    pool: &DbPool,
    jwt_secret: &str,
    request: SignupRequest,
) -> Result<AuthResponse, AppError> {
    validate_username(&request.username)?;
    validate_password(&request.password)?;
    // user_id is assigned by the insert
    let new_cgm = cgm_service::build_new_credential(
        0,
        &request.cgm_type,
        request.cgm_username,
        request.cgm_password,
        request.cgm_region,
        true,
    )?;

    if user_repository::find_by_username(pool, &request.username)
        .await?
        .is_some()
    {
        return Err(AppError::Conflict("Username already exists".to_string()));
    }

    let password_hash = hash_password(&request.password)?;
    let user = user_repository::insert_with_credential(
        pool,
        NewUser {
            username: request.username,
            password_hash,
        },
        new_cgm,
    )
    .await?;

    auth_response(user, jwt_secret)
}

pub async fn login(
    pool: &DbPool,
    jwt_secret: &str,
    request: LoginRequest,
) -> Result<AuthResponse, AppError> {
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

    auth_response(user, jwt_secret)
}

fn auth_response(user: User, jwt_secret: &str) -> Result<AuthResponse, AppError> {
    let token = generate_token(&user, jwt_secret)?;
    Ok(AuthResponse {
        token,
        user: UserResponse {
            id: user.id,
            username: user.username,
            created_at: user.created_at.to_rfc3339(),
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const SECRET: &str = "test-secret-test-secret-test-secret";

    fn user(id: i32) -> User {
        User {
            id,
            username: "alice".into(),
            password_hash: String::new(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    #[test]
    fn password_hash_round_trip() {
        let hash = hash_password("correct horse").unwrap();
        assert_ne!(hash, "correct horse");
        assert!(verify_password("correct horse", &hash).unwrap());
        assert!(!verify_password("wrong horse", &hash).unwrap());
    }

    #[test]
    fn malformed_hash_is_an_error_not_a_match() {
        assert!(verify_password("x", "not-a-phc-string").is_err());
    }

    #[test]
    fn token_round_trip() {
        let token = generate_token(&user(42), SECRET).unwrap();
        let claims = validate_token(&token, SECRET).unwrap();
        assert_eq!(claims.sub, 42);
        assert_eq!(claims.username, "alice");
        assert!(claims.exp > Utc::now().timestamp());
    }

    #[test]
    fn rejects_tampered_and_garbage_tokens() {
        let token = generate_token(&user(1), SECRET).unwrap();
        // Flip a character in the signature segment.
        let (head, sig) = token.rsplit_once('.').unwrap();
        let mut sig: Vec<char> = sig.chars().collect();
        sig[0] = if sig[0] == 'A' { 'B' } else { 'A' };
        let tampered = format!("{}.{}", head, sig.into_iter().collect::<String>());
        assert!(matches!(
            validate_token(&tampered, SECRET),
            Err(AppError::AuthError(_))
        ));
        assert!(validate_token("garbage", SECRET).is_err());
    }

    #[test]
    fn rejects_expired_and_foreign_tokens() {
        let expired = Claims {
            sub: 1,
            username: "alice".into(),
            exp: (Utc::now() - Duration::hours(1)).timestamp(),
        };
        let token = encode(
            &Header::default(),
            &expired,
            &EncodingKey::from_secret(SECRET.as_bytes()),
        )
        .unwrap();
        assert!(validate_token(&token, SECRET).is_err());

        let foreign = encode(
            &Header::default(),
            &Claims {
                exp: (Utc::now() + Duration::hours(1)).timestamp(),
                ..expired
            },
            &EncodingKey::from_secret(b"some-other-secret"),
        )
        .unwrap();
        assert!(validate_token(&foreign, SECRET).is_err());
    }

    #[test]
    fn username_and_password_length_bounds() {
        assert!(validate_username("ab").is_err());
        assert!(validate_username("abc").is_ok());
        assert!(validate_username(&"a".repeat(USERNAME_MAX_LEN)).is_ok());
        assert!(validate_username(&"a".repeat(USERNAME_MAX_LEN + 1)).is_err());
        assert!(validate_password("1234567").is_err());
        assert!(validate_password("12345678").is_ok());
        assert!(validate_password(&"p".repeat(PASSWORD_MAX_LEN + 1)).is_err());
        // counted in chars, not bytes
        assert!(validate_username("äöü").is_ok());
    }
}

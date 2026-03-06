use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;

/// Application-wide error type
#[derive(Debug)]
pub enum AppError {
    /// Authentication errors
    AuthError(String),
    /// API communication errors
    ApiError(String),
    /// Database errors
    DatabaseError(String),
    /// Configuration errors
    ConfigError(String),
    /// Generic internal errors
    InternalError(anyhow::Error),
}

impl From<anyhow::Error> for AppError {
    fn from(err: anyhow::Error) -> Self {
        AppError::InternalError(err)
    }
}

impl From<sqlx::Error> for AppError {
    fn from(err: sqlx::Error) -> Self {
        AppError::DatabaseError(err.to_string())
    }
}

impl From<reqwest::Error> for AppError {
    fn from(err: reqwest::Error) -> Self {
        AppError::ApiError(err.to_string())
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, error_message) = match &self {
            AppError::AuthError(msg) => (StatusCode::UNAUTHORIZED, msg.clone()),
            AppError::ApiError(msg) => {
                sentry::capture_error(&self);
                (StatusCode::BAD_GATEWAY, msg.clone())
            }
            AppError::DatabaseError(msg) => {
                sentry::capture_error(&self);
                tracing::error!(error = ?self, "Database error occurred");
                (StatusCode::INTERNAL_SERVER_ERROR, msg.clone())
            }
            AppError::ConfigError(msg) => {
                sentry::capture_error(&self);
                tracing::error!(error = ?self, "Configuration error occurred");
                (StatusCode::INTERNAL_SERVER_ERROR, msg.clone())
            }
            AppError::InternalError(err) => {
                sentry::capture_error(&self);
                tracing::error!(error = ?err, "Internal error occurred");
                (StatusCode::INTERNAL_SERVER_ERROR, err.to_string())
            }
        };

        let body = Json(json!({
            "error": error_message,
        }));

        (status, body).into_response()
    }
}

impl std::fmt::Display for AppError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AppError::AuthError(msg) => write!(f, "Authentication error: {}", msg),
            AppError::ApiError(msg) => write!(f, "API error: {}", msg),
            AppError::DatabaseError(msg) => write!(f, "Database error: {}", msg),
            AppError::ConfigError(msg) => write!(f, "Configuration error: {}", msg),
            AppError::InternalError(err) => write!(f, "Internal error: {}", err),
        }
    }
}

impl std::error::Error for AppError {}

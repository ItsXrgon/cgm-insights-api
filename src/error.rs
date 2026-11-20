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
    /// Invalid input / validation (e.g. username too short)
    ValidationError(String),
    /// Resource missing, or owned by another user (never reveal which)
    NotFound(String),
    /// Resource already exists (e.g. duplicate username)
    Conflict(String),
    /// The CGM platform rejected the stored credentials or needs user action
    /// (wrong password, unknown region, terms to accept in the vendor app...).
    /// Not our bug: the user must fix the credential.
    CgmCredentialsRejected(String),
    /// The CGM platform temporarily locked/throttled the account. Retry later.
    CgmAccountLocked(String),
    /// Upstream API communication errors
    ApiError(String),
    /// Database errors
    DatabaseError(String),
    /// Configuration errors
    ConfigError(String),
    /// Generic internal errors
    InternalError(anyhow::Error),
}

impl AppError {
    pub fn status_code(&self) -> StatusCode {
        match self {
            AppError::AuthError(_) => StatusCode::UNAUTHORIZED,
            AppError::ValidationError(_) => StatusCode::BAD_REQUEST,
            AppError::NotFound(_) => StatusCode::NOT_FOUND,
            AppError::Conflict(_) => StatusCode::CONFLICT,
            AppError::CgmCredentialsRejected(_) => StatusCode::UNPROCESSABLE_ENTITY,
            AppError::CgmAccountLocked(_) => StatusCode::TOO_MANY_REQUESTS,
            AppError::ApiError(_) => StatusCode::BAD_GATEWAY,
            AppError::DatabaseError(_) | AppError::ConfigError(_) | AppError::InternalError(_) => {
                StatusCode::INTERNAL_SERVER_ERROR
            }
        }
    }

    /// Failures caused by the user's CGM account state rather than by this service.
    /// These are logged as warnings and not reported to Sentry.
    pub fn is_cgm_account_issue(&self) -> bool {
        matches!(
            self,
            AppError::CgmCredentialsRejected(_) | AppError::CgmAccountLocked(_)
        )
    }
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
        let status = self.status_code();
        let error_message = match &self {
            AppError::AuthError(msg)
            | AppError::ValidationError(msg)
            | AppError::NotFound(msg)
            | AppError::Conflict(msg)
            | AppError::CgmCredentialsRejected(msg)
            | AppError::CgmAccountLocked(msg) => msg.clone(),
            AppError::ApiError(msg) => {
                sentry::capture_error(&self);
                msg.clone()
            }
            AppError::DatabaseError(msg) | AppError::ConfigError(msg) => {
                sentry::capture_error(&self);
                tracing::error!(error = ?msg, "{}", self);
                generic_internal_message()
            }
            AppError::InternalError(err) => {
                sentry::capture_error(&self);
                tracing::error!(error = ?err, "Internal error occurred");
                generic_internal_message()
            }
        };

        (status, Json(json!({ "error": error_message }))).into_response()
    }
}

fn generic_internal_message() -> String {
    "Internal server error".to_string()
}

impl std::fmt::Display for AppError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AppError::AuthError(msg) => write!(f, "Authentication error: {}", msg),
            AppError::ValidationError(msg) => write!(f, "Validation error: {}", msg),
            AppError::NotFound(msg) => write!(f, "Not found: {}", msg),
            AppError::Conflict(msg) => write!(f, "Conflict: {}", msg),
            AppError::CgmCredentialsRejected(msg) => {
                write!(f, "CGM credentials rejected: {}", msg)
            }
            AppError::CgmAccountLocked(msg) => write!(f, "CGM account locked: {}", msg),
            AppError::ApiError(msg) => write!(f, "API error: {}", msg),
            AppError::DatabaseError(msg) => write!(f, "Database error: {}", msg),
            AppError::ConfigError(msg) => write!(f, "Configuration error: {}", msg),
            AppError::InternalError(err) => write!(f, "Internal error: {}", err),
        }
    }
}

impl std::error::Error for AppError {}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::to_bytes;

    async fn body_of(err: AppError) -> (StatusCode, serde_json::Value) {
        let res = err.into_response();
        let status = res.status();
        let bytes = to_bytes(res.into_body(), usize::MAX).await.unwrap();
        (status, serde_json::from_slice(&bytes).unwrap())
    }

    #[tokio::test]
    async fn client_errors_expose_message() {
        let cases = [
            (AppError::AuthError("a".into()), StatusCode::UNAUTHORIZED),
            (AppError::ValidationError("v".into()), StatusCode::BAD_REQUEST),
            (AppError::NotFound("n".into()), StatusCode::NOT_FOUND),
            (AppError::Conflict("c".into()), StatusCode::CONFLICT),
            (
                AppError::CgmCredentialsRejected("r".into()),
                StatusCode::UNPROCESSABLE_ENTITY,
            ),
            (
                AppError::CgmAccountLocked("l".into()),
                StatusCode::TOO_MANY_REQUESTS,
            ),
        ];
        for (err, expected) in cases {
            let (status, body) = body_of(err).await;
            assert_eq!(status, expected);
            assert_eq!(body["error"].as_str().unwrap().len(), 1);
        }
    }

    #[tokio::test]
    async fn server_errors_hide_details() {
        for err in [
            AppError::DatabaseError("password=hunter2".into()),
            AppError::ConfigError("secret path".into()),
            AppError::InternalError(anyhow::anyhow!("boom")),
        ] {
            let (status, body) = body_of(err).await;
            assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
            assert_eq!(body["error"], "Internal server error");
        }
    }

    #[test]
    fn only_cgm_account_states_are_account_issues() {
        assert!(AppError::CgmAccountLocked("x".into()).is_cgm_account_issue());
        assert!(AppError::CgmCredentialsRejected("x".into()).is_cgm_account_issue());
        assert!(!AppError::ApiError("x".into()).is_cgm_account_issue());
        assert!(!AppError::DatabaseError("x".into()).is_cgm_account_issue());
    }
}

//! Cloud-specific error types.
//!
//! Maps cloud domain errors into HTTP responses. Follows the same pattern
//! as the core `AppError` but scoped to cloud operations.

use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde::Serialize;

/// Cloud API error.
#[derive(Debug, thiserror::Error)]
pub enum CloudError {
    /// Authentication required or session expired.
    #[error("unauthorized: {0}")]
    Unauthorized(String),

    /// Insufficient permissions for this operation.
    #[error("forbidden: {0}")]
    Forbidden(String),

    /// Requested resource not found.
    #[error("not found: {0}")]
    NotFound(String),

    /// Client sent invalid input.
    #[error("bad request: {0}")]
    BadRequest(String),

    /// Resource already exists (duplicate slug, email, etc.).
    #[error("conflict: {0}")]
    Conflict(String),

    /// Tier limit exceeded (environments, API requests, etc.).
    #[error("limit exceeded: {0}")]
    LimitExceeded(String),

    /// Internal error (database, crypto, etc.).
    #[error("internal error: {0}")]
    Internal(String),
}

/// JSON error response body.
#[derive(Serialize)]
struct ErrorBody {
    error: &'static str,
    message: String,
}

impl IntoResponse for CloudError {
    fn into_response(self) -> Response {
        let (status, error_type, message) = match self {
            Self::Unauthorized(msg) => (StatusCode::UNAUTHORIZED, "unauthorized", msg),
            Self::Forbidden(msg) => (StatusCode::FORBIDDEN, "forbidden", msg),
            Self::NotFound(msg) => (StatusCode::NOT_FOUND, "not_found", msg),
            Self::BadRequest(msg) => (StatusCode::BAD_REQUEST, "bad_request", msg),
            Self::Conflict(msg) => (StatusCode::CONFLICT, "conflict", msg),
            Self::LimitExceeded(msg) => (StatusCode::TOO_MANY_REQUESTS, "limit_exceeded", msg),
            Self::Internal(msg) => {
                tracing::error!(error = %msg, "cloud internal error");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "internal_error",
                    "internal server error".to_owned(),
                )
            }
        };

        let body = ErrorBody {
            error: error_type,
            message,
        };

        (status, axum::Json(body)).into_response()
    }
}

impl From<sqlx::Error> for CloudError {
    fn from(err: sqlx::Error) -> Self {
        match &err {
            sqlx::Error::RowNotFound => Self::NotFound("resource not found".to_owned()),
            sqlx::Error::Database(db_err) => {
                // PostgreSQL unique violation
                if db_err.code().as_deref() == Some("23505") {
                    Self::Conflict("resource already exists".to_owned())
                } else {
                    Self::Internal(format!("database error: {db_err}"))
                }
            }
            _ => Self::Internal(format!("database error: {err}")),
        }
    }
}

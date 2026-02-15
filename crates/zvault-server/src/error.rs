//! HTTP error types for `VaultRS` server.
//!
//! Maps domain errors from `zvault-core` into appropriate HTTP responses.
//! Every error variant produces a JSON body with a machine-readable `error`
//! field and a human-readable `message`.

use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde::Serialize;

use zvault_core::error::{
    AppRoleError, BarrierError, DatabaseError, EngineError, LeaseError, MountError, PkiError,
    PolicyError, SealError, TokenError,
};

/// Application-level error returned from HTTP handlers.
#[derive(Debug)]
pub enum AppError {
    /// The vault is sealed — reject all secret operations.
    Sealed,
    /// Authentication failed or token invalid.
    Unauthorized(String),
    /// Policy denied the operation.
    Forbidden(String),
    /// Requested resource not found.
    NotFound(String),
    /// Client sent invalid input.
    BadRequest(String),
    /// A conflict (e.g., already initialized, already mounted).
    Conflict(String),
    /// Internal server error.
    Internal(String),
}

/// JSON error response body.
#[derive(Serialize)]
struct ErrorBody {
    error: &'static str,
    message: String,
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, error_type, message) = match self {
            Self::Sealed => (
                StatusCode::SERVICE_UNAVAILABLE,
                "sealed",
                "vault is sealed".to_owned(),
            ),
            Self::Unauthorized(msg) => (StatusCode::UNAUTHORIZED, "unauthorized", msg),
            Self::Forbidden(msg) => (StatusCode::FORBIDDEN, "forbidden", msg),
            Self::NotFound(msg) => (StatusCode::NOT_FOUND, "not_found", msg),
            Self::BadRequest(msg) => (StatusCode::BAD_REQUEST, "bad_request", msg),
            Self::Conflict(msg) => (StatusCode::CONFLICT, "conflict", msg),
            Self::Internal(msg) => (StatusCode::INTERNAL_SERVER_ERROR, "internal_error", msg),
        };

        let body = ErrorBody {
            error: error_type,
            message,
        };

        (status, axum::Json(body)).into_response()
    }
}

impl From<SealError> for AppError {
    fn from(err: SealError) -> Self {
        match err {
            SealError::AlreadyInitialized
            | SealError::AlreadyUnsealed
            | SealError::AlreadySealed => Self::Conflict(err.to_string()),

            SealError::NotInitialized
            | SealError::InvalidConfig { .. }
            | SealError::InvalidShare { .. }
            | SealError::RecoveryFailed { .. }
            | SealError::RootKeyDecryption { .. } => Self::BadRequest(err.to_string()),

            SealError::Crypto(_) | SealError::Barrier(_) | SealError::Storage(_) => {
                Self::Internal(err.to_string())
            }
        }
    }
}

impl From<BarrierError> for AppError {
    fn from(err: BarrierError) -> Self {
        match err {
            BarrierError::Sealed => Self::Sealed,
            BarrierError::Crypto(_) | BarrierError::Storage(_) => Self::Internal(err.to_string()),
        }
    }
}

impl From<TokenError> for AppError {
    fn from(err: TokenError) -> Self {
        match err {
            TokenError::NotFound => Self::Unauthorized("invalid token".to_owned()),
            TokenError::Expired { .. } => Self::Unauthorized(err.to_string()),
            TokenError::NotRenewable | TokenError::MaxTtlExceeded { .. } => {
                Self::BadRequest(err.to_string())
            }
            TokenError::Barrier(ref inner) => match inner {
                BarrierError::Sealed => Self::Sealed,
                BarrierError::Crypto(_) | BarrierError::Storage(_) => {
                    Self::Internal(err.to_string())
                }
            },
        }
    }
}

impl From<PolicyError> for AppError {
    fn from(err: PolicyError) -> Self {
        match err {
            PolicyError::NotFound { .. } => Self::NotFound(err.to_string()),
            PolicyError::Invalid { .. } => Self::BadRequest(err.to_string()),
            PolicyError::BuiltIn { .. } | PolicyError::Denied { .. } => {
                Self::Forbidden(err.to_string())
            }
            PolicyError::Barrier(ref inner) => match inner {
                BarrierError::Sealed => Self::Sealed,
                BarrierError::Crypto(_) | BarrierError::Storage(_) => {
                    Self::Internal(err.to_string())
                }
            },
        }
    }
}

impl From<MountError> for AppError {
    fn from(err: MountError) -> Self {
        match err {
            MountError::AlreadyMounted { .. } => Self::Conflict(err.to_string()),
            MountError::NotFound { .. } => Self::NotFound(err.to_string()),
            MountError::InvalidPath { .. } | MountError::UnknownEngineType { .. } => {
                Self::BadRequest(err.to_string())
            }
            MountError::Barrier(ref inner) => match inner {
                BarrierError::Sealed => Self::Sealed,
                BarrierError::Crypto(_) | BarrierError::Storage(_) => {
                    Self::Internal(err.to_string())
                }
            },
        }
    }
}

impl From<EngineError> for AppError {
    fn from(err: EngineError) -> Self {
        match err {
            EngineError::NotFound { .. } => Self::NotFound(err.to_string()),
            EngineError::InvalidRequest { .. } => Self::BadRequest(err.to_string()),
            EngineError::Barrier(ref inner) => match inner {
                BarrierError::Sealed => Self::Sealed,
                BarrierError::Crypto(_) | BarrierError::Storage(_) => {
                    Self::Internal(err.to_string())
                }
            },
            EngineError::Internal { .. } => Self::Internal(err.to_string()),
        }
    }
}

impl From<LeaseError> for AppError {
    fn from(err: LeaseError) -> Self {
        match err {
            LeaseError::NotFound { .. } => Self::NotFound(err.to_string()),
            LeaseError::Expired { .. } | LeaseError::NotRenewable { .. } => {
                Self::BadRequest(err.to_string())
            }
            LeaseError::Barrier(ref inner) => match inner {
                BarrierError::Sealed => Self::Sealed,
                BarrierError::Crypto(_) | BarrierError::Storage(_) => {
                    Self::Internal(err.to_string())
                }
            },
        }
    }
}

impl From<DatabaseError> for AppError {
    fn from(err: DatabaseError) -> Self {
        match err {
            DatabaseError::NotFound { .. } | DatabaseError::RoleNotFound { .. } => {
                Self::NotFound(err.to_string())
            }
            DatabaseError::InvalidConfig { .. } => Self::BadRequest(err.to_string()),
            DatabaseError::Internal { .. } => Self::Internal(err.to_string()),
            DatabaseError::Barrier(ref inner) => match inner {
                BarrierError::Sealed => Self::Sealed,
                BarrierError::Crypto(_) | BarrierError::Storage(_) => {
                    Self::Internal(err.to_string())
                }
            },
        }
    }
}

impl From<PkiError> for AppError {
    fn from(err: PkiError) -> Self {
        match err {
            PkiError::NoRootCa | PkiError::RoleNotFound { .. } => Self::NotFound(err.to_string()),
            PkiError::InvalidRequest { .. } => Self::BadRequest(err.to_string()),
            PkiError::CertGeneration { .. } | PkiError::Internal { .. } => {
                Self::Internal(err.to_string())
            }
            PkiError::Barrier(ref inner) => match inner {
                BarrierError::Sealed => Self::Sealed,
                BarrierError::Crypto(_) | BarrierError::Storage(_) => {
                    Self::Internal(err.to_string())
                }
            },
        }
    }
}

impl From<AppRoleError> for AppError {
    fn from(err: AppRoleError) -> Self {
        match err {
            AppRoleError::RoleNotFound { .. } => Self::NotFound(err.to_string()),
            AppRoleError::InvalidSecretId { .. } => Self::Unauthorized(err.to_string()),
            AppRoleError::InvalidConfig { .. } => Self::BadRequest(err.to_string()),
            AppRoleError::Internal { .. } => Self::Internal(err.to_string()),
            AppRoleError::Barrier(ref inner) => match inner {
                BarrierError::Sealed => Self::Sealed,
                BarrierError::Crypto(_) | BarrierError::Storage(_) => {
                    Self::Internal(err.to_string())
                }
            },
        }
    }
}

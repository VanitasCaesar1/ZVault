//! System routes: `/v1/sys/*`
//!
//! Handles vault initialization, seal/unseal lifecycle, and health checks.
//! These endpoints are the first to come online and the last to go down.

use std::sync::Arc;

use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::{Deserialize, Serialize};

use crate::error::AppError;
use crate::state::AppState;

/// Build the `/v1/sys` router.
pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/init", post(init))
        .route("/unseal", post(unseal))
        .route("/seal", post(seal))
        .route("/seal-status", get(seal_status))
        .route("/health", get(health))
}

// ── Request / Response types ─────────────────────────────────────────

/// Request body for `POST /v1/sys/init`.
#[derive(Debug, Deserialize)]
pub struct InitRequest {
    /// Number of unseal key shares to generate (1-10).
    pub shares: u8,
    /// Minimum shares required to unseal (2..=shares).
    pub threshold: u8,
}

/// Response body for `POST /v1/sys/init`.
#[derive(Debug, Serialize)]
pub struct InitResponse {
    /// Base64-encoded unseal key shares (shown once).
    pub unseal_shares: Vec<String>,
    /// Root token for initial authentication.
    pub root_token: String,
}

/// Request body for `POST /v1/sys/unseal`.
#[derive(Debug, Deserialize)]
pub struct UnsealRequest {
    /// Base64-encoded unseal key share.
    pub share: String,
}

/// Response body for `POST /v1/sys/unseal`.
#[derive(Debug, Serialize)]
pub struct UnsealResponse {
    /// Whether the vault is still sealed.
    pub sealed: bool,
    /// Threshold required.
    pub threshold: u8,
    /// Shares submitted so far.
    pub progress: u8,
}

/// Response body for `GET /v1/sys/seal-status` and `GET /v1/sys/health`.
#[derive(Debug, Serialize)]
pub struct SealStatusResponse {
    /// Whether the vault has been initialized.
    pub initialized: bool,
    /// Whether the vault is currently sealed.
    pub sealed: bool,
    /// Threshold of shares required.
    pub threshold: u8,
    /// Total number of shares.
    pub shares: u8,
    /// Shares submitted in current unseal attempt.
    pub progress: u8,
}

// ── Handlers ─────────────────────────────────────────────────────────

/// Initialize a new vault.
///
/// Generates a root key, splits the unseal key into Shamir shares, and
/// returns the shares + root token. The vault is left sealed.
async fn init(
    State(state): State<Arc<AppState>>,
    Json(body): Json<InitRequest>,
) -> Result<(StatusCode, Json<InitResponse>), AppError> {
    let result = state.seal_manager.init(body.shares, body.threshold).await?;

    Ok((
        StatusCode::OK,
        Json(InitResponse {
            unseal_shares: result.unseal_shares,
            root_token: result.root_token,
        }),
    ))
}

/// Submit an unseal key share.
///
/// Returns progress if more shares are needed, or unseals the vault when
/// the threshold is reached.
async fn unseal(
    State(state): State<Arc<AppState>>,
    Json(body): Json<UnsealRequest>,
) -> Result<Json<UnsealResponse>, AppError> {
    let progress = state.seal_manager.submit_unseal_share(&body.share).await?;

    match progress {
        Some(p) => Ok(Json(UnsealResponse {
            sealed: true,
            threshold: p.threshold,
            progress: p.submitted,
        })),
        None => Ok(Json(UnsealResponse {
            sealed: false,
            threshold: 0,
            progress: 0,
        })),
    }
}

/// Seal the vault, zeroizing all key material from memory.
async fn seal(State(state): State<Arc<AppState>>) -> Result<StatusCode, AppError> {
    state.seal_manager.seal().await?;
    Ok(StatusCode::NO_CONTENT)
}

/// Get the current seal status.
async fn seal_status(
    State(state): State<Arc<AppState>>,
) -> Result<Json<SealStatusResponse>, AppError> {
    let status = state.seal_manager.status().await?;
    Ok(Json(SealStatusResponse {
        initialized: status.initialized,
        sealed: status.sealed,
        threshold: status.threshold,
        shares: status.shares,
        progress: status.progress,
    }))
}

/// Health check endpoint. No auth required.
///
/// Returns 200 if unsealed, 503 if sealed, 501 if not initialized.
async fn health(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let status = state.seal_manager.status().await;

    match status {
        Ok(s) if !s.initialized => {
            let body = SealStatusResponse {
                initialized: false,
                sealed: true,
                threshold: 0,
                shares: 0,
                progress: 0,
            };
            (StatusCode::NOT_IMPLEMENTED, Json(body))
        }
        Ok(s) if s.sealed => {
            let body = SealStatusResponse {
                initialized: s.initialized,
                sealed: true,
                threshold: s.threshold,
                shares: s.shares,
                progress: s.progress,
            };
            (StatusCode::SERVICE_UNAVAILABLE, Json(body))
        }
        Ok(s) => {
            let body = SealStatusResponse {
                initialized: s.initialized,
                sealed: s.sealed,
                threshold: s.threshold,
                shares: s.shares,
                progress: s.progress,
            };
            (StatusCode::OK, Json(body))
        }
        Err(_) => {
            let body = SealStatusResponse {
                initialized: false,
                sealed: true,
                threshold: 0,
                shares: 0,
                progress: 0,
            };
            (StatusCode::INTERNAL_SERVER_ERROR, Json(body))
        }
    }
}

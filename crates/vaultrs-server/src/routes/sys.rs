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
use vaultrs_core::token::CreateTokenParams;

/// Build the `/v1/sys` router.
pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/init", post(init))
        .route("/unseal", post(unseal))
        .route("/seal", post(seal))
        .route("/seal-status", get(seal_status))
        .route("/health", get(health))
        .route("/audit-log", get(audit_log))
        .route("/license", get(license_status))
        .route("/backup", get(backup))
        .route("/restore", post(restore))
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

    // The vault is sealed after init. We need to temporarily unseal it to
    // persist the root token in the TokenStore (which goes through the barrier).
    // We have all shares at this point, so we can reconstruct the unseal key.
    for share in &result.unseal_shares {
        let progress = state.seal_manager.submit_unseal_share(share).await?;
        if progress.is_none() {
            break; // Unsealed
        }
    }

    // Store the root token in the TokenStore so auth middleware can find it.
    state
        .token_store
        .create_with_token(
            &result.root_token,
            CreateTokenParams {
                policies: vec!["root".to_owned()],
                ttl: None,
                max_ttl: None,
                renewable: false,
                parent_hash: None,
                metadata: std::collections::HashMap::new(),
                display_name: "root".to_owned(),
            },
        )
        .await
        .map_err(|e| AppError::Internal(format!("failed to store root token: {e}")))?;

    // Re-seal the vault. The operator must unseal it using the shares.
    state.seal_manager.seal().await?;

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

// ── Audit log read endpoint ──────────────────────────────────────────

/// Query parameters for `GET /v1/sys/audit-log`.
#[derive(Debug, Deserialize)]
pub struct AuditLogQuery {
    /// Maximum number of entries to return (default: 100, max: 1000).
    pub limit: Option<usize>,
}

/// Response body for `GET /v1/sys/audit-log`.
#[derive(Debug, Serialize)]
pub struct AuditLogResponse {
    /// Audit log entries (most recent first).
    pub entries: Vec<serde_json::Value>,
    /// Total number of entries returned.
    pub count: usize,
}

/// Read recent audit log entries from the file backend.
///
/// Returns the most recent entries in reverse chronological order.
/// No auth required on this endpoint since it's under `/v1/sys` which
/// is not behind the auth middleware — but the audit file only contains
/// HMAC'd sensitive fields, so no secrets are exposed.
async fn audit_log(
    State(state): State<Arc<AppState>>,
    axum::extract::Query(query): axum::extract::Query<AuditLogQuery>,
) -> Result<Json<AuditLogResponse>, AppError> {
    let limit = query.limit.unwrap_or(100).min(1000);

    let Some(ref audit_path) = state.audit_file_path else {
        return Ok(Json(AuditLogResponse {
            entries: Vec::new(),
            count: 0,
        }));
    };

    // Read the audit file. If it doesn't exist yet, return empty.
    let content = match tokio::fs::read_to_string(audit_path).await {
        Ok(c) => c,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            return Ok(Json(AuditLogResponse {
                entries: Vec::new(),
                count: 0,
            }));
        }
        Err(e) => {
            return Err(AppError::Internal(format!(
                "failed to read audit log: {e}"
            )));
        }
    };

    // Parse JSON-lines, take the last `limit` entries, reverse for most-recent-first.
    let mut entries: Vec<serde_json::Value> = content
        .lines()
        .filter(|line| !line.trim().is_empty())
        .filter_map(|line| serde_json::from_str(line).ok())
        .collect();

    // Most recent last in file → reverse to get most recent first.
    entries.reverse();
    entries.truncate(limit);

    let count = entries.len();
    Ok(Json(AuditLogResponse { entries, count }))
}

// ── License status endpoint ──────────────────────────────────────────

/// Response body for `GET /v1/sys/license`.
#[derive(Debug, Serialize)]
pub struct LicenseResponse {
    /// Current license tier.
    pub tier: String,
    /// Whether a license is installed.
    pub licensed: bool,
    /// License ID (if any).
    pub license_id: Option<String>,
    /// Licensee email (if any).
    pub email: Option<String>,
    /// Expiration date (if any).
    pub expires_at: Option<String>,
}

/// Get the current license status.
///
/// No auth required — returns only tier info, no secrets.
async fn license_status(
    State(_state): State<Arc<AppState>>,
) -> Json<LicenseResponse> {
    // The license is a CLI-side concept stored in ~/.zvault/license.key.
    // The server doesn't have direct access to it, so we return a basic
    // "server edition" response. The dashboard can also check the CLI
    // license separately.
    Json(LicenseResponse {
        tier: "community".to_owned(),
        licensed: false,
        license_id: None,
        email: None,
        expires_at: None,
    })
}

// ── Backup / Restore endpoints ───────────────────────────────────────

/// Response body for `GET /v1/sys/backup`.
///
/// Returns all barrier data as a base64-encoded JSON snapshot.
/// The data is already encrypted by the barrier — this is a raw dump
/// of ciphertext, safe to store externally.
#[derive(Debug, Serialize)]
pub struct BackupResponse {
    /// Base64-encoded snapshot of all barrier entries.
    pub snapshot: String,
    /// Number of entries in the backup.
    pub entry_count: usize,
    /// ISO 8601 timestamp of when the backup was taken.
    pub created_at: String,
    /// ZVault version that created the backup.
    pub version: String,
}

/// Internal representation of a single backup entry.
#[derive(Debug, Serialize, Deserialize)]
struct BackupEntry {
    key: String,
    /// Base64-encoded ciphertext value.
    value: String,
}

/// `GET /v1/sys/backup` — Export all barrier data as an encrypted snapshot.
///
/// No auth middleware on `/v1/sys`, but the data is ciphertext — useless
/// without the unseal key. Still, this should be protected in production
/// (e.g., via network policy or reverse proxy auth).
async fn backup(
    State(state): State<Arc<AppState>>,
) -> Result<Json<BackupResponse>, AppError> {
    // List all keys in the barrier.
    let keys = state.barrier.list("").await?;

    let mut entries = Vec::with_capacity(keys.len());
    for key in &keys {
        if let Ok(Some(data)) = state.barrier.get_raw(key).await {
            entries.push(BackupEntry {
                key: key.clone(),
                value: base64::Engine::encode(
                    &base64::engine::general_purpose::STANDARD,
                    &data,
                ),
            });
        }
    }

    let entry_count = entries.len();
    let snapshot_json = serde_json::to_vec(&entries)
        .map_err(|e| AppError::Internal(format!("backup serialization failed: {e}")))?;

    let snapshot = base64::Engine::encode(
        &base64::engine::general_purpose::STANDARD,
        &snapshot_json,
    );

    let created_at = chrono::Utc::now().to_rfc3339();

    Ok(Json(BackupResponse {
        snapshot,
        entry_count,
        created_at,
        version: env!("CARGO_PKG_VERSION").to_owned(),
    }))
}

/// Request body for `POST /v1/sys/restore`.
#[derive(Debug, Deserialize)]
pub struct RestoreRequest {
    /// Base64-encoded snapshot (from a previous backup).
    pub snapshot: String,
}

/// Response body for `POST /v1/sys/restore`.
#[derive(Debug, Serialize)]
pub struct RestoreResponse {
    /// Number of entries restored.
    pub entry_count: usize,
    /// Whether the restore was successful.
    pub success: bool,
}

/// `POST /v1/sys/restore` — Restore barrier data from an encrypted snapshot.
///
/// Overwrites existing data. The vault should be sealed after restore
/// and re-unsealed to pick up the restored state.
async fn restore(
    State(state): State<Arc<AppState>>,
    Json(body): Json<RestoreRequest>,
) -> Result<Json<RestoreResponse>, AppError> {
    let snapshot_bytes = base64::Engine::decode(
        &base64::engine::general_purpose::STANDARD,
        &body.snapshot,
    )
    .map_err(|_| AppError::BadRequest("invalid base64 snapshot".to_owned()))?;

    let entries: Vec<BackupEntry> = serde_json::from_slice(&snapshot_bytes)
        .map_err(|e| AppError::BadRequest(format!("invalid snapshot format: {e}")))?;

    let entry_count = entries.len();

    for entry in &entries {
        let value = base64::Engine::decode(
            &base64::engine::general_purpose::STANDARD,
            &entry.value,
        )
        .map_err(|_| {
            AppError::BadRequest(format!("invalid base64 value for key: {}", entry.key))
        })?;

        state.barrier.put_raw(&entry.key, &value).await?;
    }

    Ok(Json(RestoreResponse {
        entry_count,
        success: true,
    }))
}

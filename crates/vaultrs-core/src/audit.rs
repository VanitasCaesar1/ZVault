//! Audit logging system for `VaultRS`.
//!
//! Every API request that touches secrets, auth, or system config generates
//! an audit entry BEFORE the response is sent. If all audit backends fail
//! to write, the request is denied (fail-closed). This is non-negotiable.
//!
//! Sensitive fields (token values, secret data) are HMAC'd with a per-backend
//! key before writing, so audit logs can be used for correlation without
//! exposing actual secret values.

use std::sync::Arc;

use chrono::{DateTime, Utc};
use hmac::{Hmac, Mac};
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use tokio::sync::RwLock;
use tracing::warn;

use crate::error::AuditError;

type HmacSha256 = Hmac<Sha256>;

/// A single audit log entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEntry {
    /// Unique entry ID.
    pub id: String,
    /// When the event occurred.
    pub timestamp: DateTime<Utc>,
    /// Request details.
    pub request: AuditRequest,
    /// Response details.
    pub response: AuditResponse,
    /// Authentication context.
    pub auth: AuditAuth,
}

/// Request portion of an audit entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditRequest {
    /// Operation type (read, write, delete, login, etc.).
    pub operation: String,
    /// Request path.
    pub path: String,
    /// Request data (sensitive fields HMAC'd).
    pub data: Option<serde_json::Value>,
    /// Client IP address.
    pub remote_addr: String,
}

/// Response portion of an audit entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditResponse {
    /// HTTP status code.
    pub status_code: u16,
    /// Error message if any.
    pub error: Option<String>,
}

/// Auth context of an audit entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditAuth {
    /// HMAC'd token identifier.
    pub token_id: String,
    /// Policies attached to the token.
    pub policies: Vec<String>,
    /// Token metadata.
    pub metadata: std::collections::HashMap<String, String>,
}

/// Trait for audit log backends.
///
/// Implementations must be safe to share across async tasks.
#[async_trait::async_trait]
pub trait AuditBackend: Send + Sync {
    /// The backend's name (for error reporting).
    fn name(&self) -> &str;

    /// Write an audit entry. Must not silently drop entries.
    ///
    /// # Errors
    ///
    /// Returns an error if the entry could not be persisted.
    async fn log(&self, entry: &AuditEntry) -> Result<(), AuditError>;
}

/// Manages multiple audit backends with fail-closed semantics.
///
/// If at least one backend succeeds, the request proceeds. If ALL fail,
/// the request is denied.
pub struct AuditManager {
    backends: RwLock<Vec<Arc<dyn AuditBackend>>>,
    /// HMAC key for hashing sensitive fields in audit entries.
    hmac_key: Vec<u8>,
}

impl AuditManager {
    /// Create a new audit manager with the given HMAC key.
    #[must_use]
    pub fn new(hmac_key: Vec<u8>) -> Self {
        Self {
            backends: RwLock::new(Vec::new()),
            hmac_key,
        }
    }

    /// Register an audit backend.
    pub async fn add_backend(&self, backend: Arc<dyn AuditBackend>) {
        self.backends.write().await.push(backend);
    }

    /// Log an audit entry to all backends.
    ///
    /// At least one backend must succeed. If all fail, returns
    /// [`AuditError::AllBackendsFailed`] and the request must be denied.
    ///
    /// # Errors
    ///
    /// Returns [`AuditError::AllBackendsFailed`] if every backend fails.
    pub async fn log(&self, entry: &AuditEntry) -> Result<(), AuditError> {
        let backends = self.backends.read().await;

        if backends.is_empty() {
            // No backends configured — nothing to audit.
            return Ok(());
        }

        let mut any_success = false;
        for backend in backends.iter() {
            match backend.log(entry).await {
                Ok(()) => any_success = true,
                Err(e) => {
                    warn!(
                        backend = backend.name(),
                        error = %e,
                        "audit backend failed"
                    );
                }
            }
        }

        if any_success {
            Ok(())
        } else {
            Err(AuditError::AllBackendsFailed)
        }
    }

    /// HMAC a sensitive field value for safe inclusion in audit logs.
    ///
    /// Returns the hex-encoded HMAC-SHA256 of the input.
    #[must_use]
    #[allow(clippy::missing_panics_doc)]
    pub fn hmac_field(&self, value: &str) -> String {
        // HMAC-SHA256 accepts any key length per RFC 2104, so new_from_slice
        // will never fail here.
        #[allow(clippy::unwrap_used)]
        let mut mac = HmacSha256::new_from_slice(&self.hmac_key)
            // SAFETY: HMAC-SHA256 accepts any key length — this never fails.
            .unwrap();
        mac.update(value.as_bytes());
        hex::encode(mac.finalize().into_bytes())
    }

    /// Check whether any audit backends are configured.
    pub async fn has_backends(&self) -> bool {
        !self.backends.read().await.is_empty()
    }
}

impl std::fmt::Debug for AuditManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AuditManager")
            .field("hmac_key", &"[REDACTED]")
            .finish_non_exhaustive()
    }
}

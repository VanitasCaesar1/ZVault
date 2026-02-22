//! Public types for the `ZVault` SDK.

use serde::{Deserialize, Serialize};

/// A single secret entry returned by the API.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecretEntry {
    /// Secret key name.
    pub key: String,
    /// Decrypted secret value.
    pub value: String,
    /// Version number.
    pub version: i64,
    /// Optional comment.
    pub comment: String,
    /// ISO 8601 creation timestamp.
    pub created_at: String,
    /// ISO 8601 last-updated timestamp.
    pub updated_at: String,
}

/// A secret key (no value) from list operations.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecretKey {
    /// Secret key name.
    pub key: String,
    /// Version number.
    pub version: i64,
    /// Optional comment.
    pub comment: String,
    /// ISO 8601 last-updated timestamp.
    pub updated_at: String,
}

/// Health check result.
#[derive(Debug, Clone)]
pub struct HealthStatus {
    /// Whether the API is reachable and the token is valid.
    pub ok: bool,
    /// Round-trip latency in milliseconds.
    pub latency_ms: u128,
    /// Number of secrets currently cached.
    pub cached_secrets: usize,
}

// --- Internal API response types ---

#[derive(Deserialize)]
pub(crate) struct SecretResponse {
    pub secret: SecretEntry,
}

#[derive(Deserialize)]
pub(crate) struct SecretKeysResponse {
    pub keys: Vec<SecretKey>,
}

#[derive(Deserialize)]
pub(crate) struct ApiErrorBody {
    pub error: Option<ApiErrorDetail>,
}

#[derive(Deserialize)]
pub(crate) struct ApiErrorDetail {
    pub message: Option<String>,
}

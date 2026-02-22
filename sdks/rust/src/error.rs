//! Error types for the `ZVault` SDK.

/// All errors that can occur when using the `ZVault` SDK.
#[derive(Debug, thiserror::Error)]
pub enum ZVaultError {
    /// Missing required configuration.
    #[error("zvault config error: {0}")]
    Config(String),

    /// API returned an HTTP error.
    #[error("zvault API error {status_code}: {message}")]
    Api {
        /// HTTP status code.
        status_code: u16,
        /// Error message from the API.
        message: String,
    },

    /// Authentication failed (401/403).
    #[error("zvault auth error: {0}")]
    Auth(String),

    /// Secret not found (404).
    #[error("secret \"{key}\" not found in environment \"{env}\"")]
    NotFound {
        /// The secret key that was not found.
        key: String,
        /// The environment that was searched.
        env: String,
    },

    /// Request timed out.
    #[error("zvault request timed out")]
    Timeout,

    /// Network or HTTP client error.
    #[error("zvault network error: {0}")]
    Network(#[from] reqwest::Error),

    /// JSON serialization/deserialization error.
    #[error("zvault json error: {0}")]
    Json(#[from] serde_json::Error),
}

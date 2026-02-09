//! Storage error types.
//!
//! Every error variant carries enough context to diagnose the problem
//! without a debugger, following `VaultRS` error handling standards.

/// Errors that can occur during storage operations.
#[derive(Debug, thiserror::Error)]
pub enum StorageError {
    /// Failed to open the storage backend at the given path.
    #[error("failed to open storage at '{path}': {reason}")]
    Open { path: String, reason: String },

    /// Failed to read a value from storage.
    #[error("failed to read key '{key}': {reason}")]
    Read { key: String, reason: String },

    /// Failed to write a value to storage.
    #[error("failed to write key '{key}': {reason}")]
    Write { key: String, reason: String },

    /// Failed to delete a key from storage.
    #[error("failed to delete key '{key}': {reason}")]
    Delete { key: String, reason: String },

    /// Failed to list keys with the given prefix.
    #[error("failed to list keys with prefix '{prefix}': {reason}")]
    List { prefix: String, reason: String },

    /// A required column family or table was not found.
    #[error("missing column family or table '{name}'")]
    MissingTable { name: String },

    /// Failed to begin or commit a transaction.
    #[error("transaction failed: {reason}")]
    Transaction { reason: String },

    /// A storage key contained invalid UTF-8.
    #[error("invalid key encoding: {reason}")]
    InvalidKey { reason: String },
}

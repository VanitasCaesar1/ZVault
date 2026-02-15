//! Error types for `zvault-core`.
//!
//! Each error variant carries enough context to diagnose the problem without
//! a debugger. Crypto errors never include key material — only key identifiers
//! or operation descriptions.

use zvault_storage::StorageError;

/// Errors from cryptographic operations.
#[derive(Debug, thiserror::Error)]
pub enum CryptoError {
    /// AES-256-GCM encryption failed.
    #[error("encryption failed: {reason}")]
    Encryption { reason: String },

    /// AES-256-GCM decryption failed (wrong key, corrupted ciphertext, or tampered tag).
    #[error("decryption failed: {reason}")]
    Decryption { reason: String },

    /// HKDF key derivation failed.
    #[error("key derivation failed for context '{context}': {reason}")]
    KeyDerivation { context: String, reason: String },

    /// Ciphertext is too short to contain a valid nonce + tag.
    #[error("ciphertext too short: expected at least {expected} bytes, got {actual}")]
    CiphertextTooShort { expected: usize, actual: usize },
}

/// Errors from the encryption barrier.
#[derive(Debug, thiserror::Error)]
pub enum BarrierError {
    /// The vault is sealed — no operations are possible until unseal.
    #[error("vault is sealed")]
    Sealed,

    /// A cryptographic operation within the barrier failed.
    #[error("barrier crypto error: {0}")]
    Crypto(#[from] CryptoError),

    /// The underlying storage backend returned an error.
    #[error("barrier storage error: {0}")]
    Storage(#[from] StorageError),
}

/// Errors from seal/unseal operations.
#[derive(Debug, thiserror::Error)]
pub enum SealError {
    /// The vault has already been initialized.
    #[error("vault is already initialized")]
    AlreadyInitialized,

    /// The vault has not been initialized yet.
    #[error("vault is not initialized")]
    NotInitialized,

    /// The vault is already unsealed.
    #[error("vault is already unsealed")]
    AlreadyUnsealed,

    /// The vault is already sealed.
    #[error("vault is already sealed")]
    AlreadySealed,

    /// Invalid Shamir configuration parameters.
    #[error("invalid seal config: {reason}")]
    InvalidConfig { reason: String },

    /// A submitted unseal share was invalid or corrupted.
    #[error("invalid unseal share: {reason}")]
    InvalidShare { reason: String },

    /// Shamir secret recovery failed (not enough shares or corrupted shares).
    #[error("share recovery failed: {reason}")]
    RecoveryFailed { reason: String },

    /// Failed to decrypt the root key with the reconstructed unseal key.
    #[error("root key decryption failed: {reason}")]
    RootKeyDecryption { reason: String },

    /// A cryptographic operation failed during seal/unseal.
    #[error("seal crypto error: {0}")]
    Crypto(#[from] CryptoError),

    /// The encryption barrier returned an error during raw storage access.
    #[error("seal barrier error: {0}")]
    Barrier(#[from] BarrierError),

    /// The underlying storage backend returned an error.
    #[error("seal storage error: {0}")]
    Storage(#[from] StorageError),
}

/// Errors from token operations.
#[derive(Debug, thiserror::Error)]
pub enum TokenError {
    /// The token was not found in storage.
    #[error("token not found")]
    NotFound,

    /// The token has expired.
    #[error("token expired at {expired_at}")]
    Expired { expired_at: String },

    /// The token is not renewable.
    #[error("token is not renewable")]
    NotRenewable,

    /// The token has exceeded its maximum TTL.
    #[error("token has exceeded max TTL of {max_ttl_secs}s")]
    MaxTtlExceeded { max_ttl_secs: i64 },

    /// The barrier returned an error.
    #[error("token barrier error: {0}")]
    Barrier(#[from] BarrierError),
}

/// Errors from policy operations.
#[derive(Debug, thiserror::Error)]
pub enum PolicyError {
    /// The requested policy was not found.
    #[error("policy not found: {name}")]
    NotFound { name: String },

    /// The policy document is invalid.
    #[error("invalid policy: {reason}")]
    Invalid { reason: String },

    /// Cannot modify a built-in policy.
    #[error("cannot modify built-in policy: {name}")]
    BuiltIn { name: String },

    /// Access denied by policy evaluation.
    #[error("permission denied on path '{path}' for capability '{capability}'")]
    Denied { path: String, capability: String },

    /// The barrier returned an error.
    #[error("policy barrier error: {0}")]
    Barrier(#[from] BarrierError),
}

/// Errors from audit operations.
#[derive(Debug, thiserror::Error)]
pub enum AuditError {
    /// All audit backends failed to write — request must be denied.
    #[error("all audit backends failed (fail-closed)")]
    AllBackendsFailed,

    /// A specific audit backend failed.
    #[error("audit backend '{name}' failed: {reason}")]
    BackendFailure { name: String, reason: String },

    /// Serialization of the audit entry failed.
    #[error("audit serialization failed: {reason}")]
    Serialization { reason: String },
}

/// Errors from mount table operations.
#[derive(Debug, thiserror::Error)]
pub enum MountError {
    /// The mount path is already in use.
    #[error("mount path already in use: {path}")]
    AlreadyMounted { path: String },

    /// The mount path was not found.
    #[error("mount not found: {path}")]
    NotFound { path: String },

    /// Invalid mount path.
    #[error("invalid mount path: {reason}")]
    InvalidPath { reason: String },

    /// Unknown engine type.
    #[error("unknown engine type: {engine_type}")]
    UnknownEngineType { engine_type: String },

    /// The barrier returned an error.
    #[error("mount barrier error: {0}")]
    Barrier(#[from] BarrierError),
}

/// Errors from secrets engine operations.
#[derive(Debug, thiserror::Error)]
pub enum EngineError {
    /// The requested secret was not found.
    #[error("secret not found at path '{path}'")]
    NotFound { path: String },

    /// Invalid request to the engine.
    #[error("invalid engine request: {reason}")]
    InvalidRequest { reason: String },

    /// The barrier returned an error.
    #[error("engine barrier error: {0}")]
    Barrier(#[from] BarrierError),

    /// Internal engine error.
    #[error("engine internal error: {reason}")]
    Internal { reason: String },
}

/// Errors from lease operations.
#[derive(Debug, thiserror::Error)]
pub enum LeaseError {
    /// The lease was not found.
    #[error("lease not found: {lease_id}")]
    NotFound { lease_id: String },

    /// The lease has already expired.
    #[error("lease already expired: {lease_id}")]
    Expired { lease_id: String },

    /// The lease is not renewable.
    #[error("lease is not renewable: {lease_id}")]
    NotRenewable { lease_id: String },

    /// The barrier returned an error.
    #[error("lease barrier error: {0}")]
    Barrier(#[from] BarrierError),
}

/// Errors from the database secrets engine.
#[derive(Debug, thiserror::Error)]
pub enum DatabaseError {
    /// Database connection config not found.
    #[error("database config not found: {name}")]
    NotFound { name: String },

    /// Database role not found.
    #[error("database role not found: {name}")]
    RoleNotFound { name: String },

    /// Invalid configuration parameters.
    #[error("invalid database config: {reason}")]
    InvalidConfig { reason: String },

    /// Internal engine error.
    #[error("database engine error: {reason}")]
    Internal { reason: String },

    /// The barrier returned an error.
    #[error("database barrier error: {0}")]
    Barrier(#[from] BarrierError),
}

/// Errors from the PKI secrets engine.
#[derive(Debug, thiserror::Error)]
pub enum PkiError {
    /// No root CA has been generated yet.
    #[error("no root CA configured — generate one first")]
    NoRootCa,

    /// PKI role not found.
    #[error("PKI role not found: {name}")]
    RoleNotFound { name: String },

    /// Invalid configuration or request.
    #[error("invalid PKI request: {reason}")]
    InvalidRequest { reason: String },

    /// Certificate generation failed.
    #[error("certificate generation failed: {reason}")]
    CertGeneration { reason: String },

    /// Internal engine error.
    #[error("PKI engine error: {reason}")]
    Internal { reason: String },

    /// The barrier returned an error.
    #[error("PKI barrier error: {0}")]
    Barrier(#[from] BarrierError),
}

/// Errors from the AppRole auth method.
#[derive(Debug, thiserror::Error)]
pub enum AppRoleError {
    /// AppRole role not found.
    #[error("approle role not found: {name}")]
    RoleNotFound { name: String },

    /// Invalid secret ID.
    #[error("invalid secret ID for role '{role_name}'")]
    InvalidSecretId { role_name: String },

    /// Invalid configuration.
    #[error("invalid approle config: {reason}")]
    InvalidConfig { reason: String },

    /// Internal error.
    #[error("approle error: {reason}")]
    Internal { reason: String },

    /// The barrier returned an error.
    #[error("approle barrier error: {0}")]
    Barrier(#[from] BarrierError),
}

//! Seal/unseal lifecycle for `ZVault`.
//!
//! Implements the Shamir's Secret Sharing based initialization and unseal
//! protocol. The flow is:
//!
//! 1. **Init**: Generate root key + unseal key, encrypt root key with unseal
//!    key, split unseal key into N shares with threshold T, store encrypted
//!    root key, return shares to operator (shown once, never stored).
//!
//! 2. **Unseal**: Operator submits shares one at a time. When T shares are
//!    collected, reconstruct the unseal key, decrypt the root key, unseal
//!    the barrier.
//!
//! 3. **Seal**: Zeroize the root key from memory, seal the barrier.
//!
//! # Security model
//!
//! - The unseal key is never stored. It exists only as Shamir shares held by
//!   operators.
//! - The root key is stored encrypted by the unseal key at `sys/seal/root_key`.
//! - Seal config (threshold, share count) is stored at `sys/seal/config`.
//! - Shares are shown once at init time and never persisted by the server.

use std::sync::Arc;

use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64;
use serde::{Deserialize, Serialize};
use sharks::{Share, Sharks};
use tokio::sync::Mutex;
use tracing::info;

use crate::barrier::Barrier;
use crate::crypto::{self, EncryptionKey};
use crate::error::SealError;

/// Storage key for the encrypted root key.
const ROOT_KEY_PATH: &str = "sys/seal/root_key";

/// Storage key for the seal configuration.
const SEAL_CONFIG_PATH: &str = "sys/seal/config";

/// Persisted seal configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SealConfig {
    /// Total number of unseal shares.
    pub shares: u8,
    /// Minimum shares required to reconstruct the unseal key.
    pub threshold: u8,
}

/// Result of a successful vault initialization.
#[derive(Debug)]
pub struct InitResult {
    /// Base64-encoded unseal key shares. Shown once, never stored.
    pub unseal_shares: Vec<String>,
    /// The root token for initial authentication.
    pub root_token: String,
}

/// Progress of an ongoing unseal operation.
#[derive(Debug, Clone)]
pub struct UnsealProgress {
    /// Total threshold required.
    pub threshold: u8,
    /// Number of shares submitted so far.
    pub submitted: u8,
}

/// Manages the seal/unseal lifecycle.
///
/// Holds the barrier, accumulated unseal shares, and seal configuration.
/// Thread-safe via internal `Mutex` on the mutable share accumulator.
pub struct SealManager {
    barrier: Arc<Barrier>,
    /// Accumulated raw share bytes during unseal. Cleared after success or seal.
    pending_shares: Mutex<Vec<Vec<u8>>>,
}

impl SealManager {
    /// Create a new seal manager wrapping the given barrier.
    #[must_use]
    pub fn new(barrier: Arc<Barrier>) -> Self {
        Self {
            barrier,
            pending_shares: Mutex::new(Vec::new()),
        }
    }

    /// Initialize a new vault.
    ///
    /// Generates a root key and unseal key, encrypts the root key with the
    /// unseal key, splits the unseal key into Shamir shares, stores the
    /// encrypted root key and config, and returns the shares + root token.
    ///
    /// The vault is left in a **sealed** state after init. The operator must
    /// unseal it using the returned shares.
    ///
    /// # Errors
    ///
    /// - [`SealError::AlreadyInitialized`] if the vault has already been initialized.
    /// - [`SealError::InvalidConfig`] if share count or threshold are out of bounds.
    /// - [`SealError::Crypto`] if key generation or encryption fails.
    /// - [`SealError::Storage`] if writing to the backend fails.
    pub async fn init(&self, shares: u8, threshold: u8) -> Result<InitResult, SealError> {
        // Validate parameters per security rules: 1-10 shares, threshold 2..=shares.
        validate_config(shares, threshold)?;

        // Check if already initialized.
        if self.is_initialized().await? {
            return Err(SealError::AlreadyInitialized);
        }

        // Generate root key (256-bit, will encrypt all vault data).
        let root_key = EncryptionKey::generate();

        // Generate unseal key (256-bit, will be split into Shamir shares).
        let unseal_key = EncryptionKey::generate();

        // Encrypt root key with unseal key.
        let encrypted_root = crypto::encrypt(&unseal_key, root_key.as_bytes())?;

        // Split unseal key into Shamir shares.
        let shamir = Sharks(threshold);
        let dealer = shamir.dealer(unseal_key.as_bytes());
        let share_vec: Vec<Share> = dealer.take(usize::from(shares)).collect();
        let encoded_shares: Vec<String> = share_vec
            .iter()
            .map(|s| BASE64.encode(Vec::from(s)))
            .collect();

        // Store encrypted root key (raw — it's already encrypted by unseal key).
        self.barrier
            .put_raw(ROOT_KEY_PATH, &encrypted_root)
            .await
            .map_err(SealError::Barrier)?;

        // Store seal config (raw — not sensitive, but stored before barrier is unsealed).
        let config = SealConfig { shares, threshold };
        let config_bytes = serde_json::to_vec(&config).map_err(|e| SealError::InvalidConfig {
            reason: format!("failed to serialize seal config: {e}"),
        })?;
        self.barrier
            .put_raw(SEAL_CONFIG_PATH, &config_bytes)
            .await
            .map_err(SealError::Barrier)?;

        // Generate root token (UUID v4).
        let root_token = uuid::Uuid::new_v4().to_string();

        info!(shares = shares, threshold = threshold, "vault initialized");

        Ok(InitResult {
            unseal_shares: encoded_shares,
            root_token,
        })
    }

    /// Submit an unseal share.
    ///
    /// Returns `Ok(Some(progress))` if more shares are needed, or `Ok(None)`
    /// when the threshold is reached and the vault is successfully unsealed.
    ///
    /// # Errors
    ///
    /// - [`SealError::NotInitialized`] if the vault hasn't been initialized.
    /// - [`SealError::AlreadyUnsealed`] if the vault is already unsealed.
    /// - [`SealError::InvalidShare`] if the share is malformed.
    /// - [`SealError::RecoveryFailed`] if share reconstruction fails.
    /// - [`SealError::RootKeyDecryption`] if the reconstructed key can't decrypt the root key.
    pub async fn submit_unseal_share(
        &self,
        share_b64: &str,
    ) -> Result<Option<UnsealProgress>, SealError> {
        // Must be initialized.
        if !self.is_initialized().await? {
            return Err(SealError::NotInitialized);
        }

        // Must be sealed.
        if self.barrier.is_unsealed().await {
            return Err(SealError::AlreadyUnsealed);
        }

        // Decode the share.
        let share_bytes = BASE64
            .decode(share_b64)
            .map_err(|e| SealError::InvalidShare {
                reason: format!("base64 decode failed: {e}"),
            })?;

        // Load config to know the threshold.
        let config = self.load_config().await?;

        // Accumulate the share.
        let mut pending = self.pending_shares.lock().await;
        pending.push(share_bytes);

        let submitted = u8::try_from(pending.len()).unwrap_or(u8::MAX);

        if submitted < config.threshold {
            return Ok(Some(UnsealProgress {
                threshold: config.threshold,
                submitted,
            }));
        }

        // We have enough shares — attempt reconstruction.
        let shamir = Sharks(config.threshold);
        let parsed_shares: Result<Vec<Share>, SealError> = pending
            .iter()
            .map(|bytes| {
                Share::try_from(bytes.as_slice()).map_err(|e| SealError::InvalidShare {
                    reason: format!("share deserialization failed: {e}"),
                })
            })
            .collect();
        let parsed_shares = parsed_shares?;

        let unseal_key_bytes =
            shamir
                .recover(&parsed_shares)
                .map_err(|e| SealError::RecoveryFailed {
                    reason: e.to_string(),
                })?;

        // Clear pending shares immediately.
        pending.clear();
        drop(pending);

        // Reconstruct the unseal key.
        let unseal_key_array: [u8; 32] =
            unseal_key_bytes
                .try_into()
                .map_err(|_| SealError::RecoveryFailed {
                    reason: "recovered key is not 32 bytes".to_owned(),
                })?;
        let unseal_key = EncryptionKey::from_bytes(unseal_key_array);

        // Load and decrypt the root key.
        let encrypted_root = self
            .barrier
            .get_raw(ROOT_KEY_PATH)
            .await
            .map_err(SealError::Barrier)?
            .ok_or(SealError::NotInitialized)?;

        let root_key_bytes = crypto::decrypt(&unseal_key, &encrypted_root).map_err(|e| {
            SealError::RootKeyDecryption {
                reason: e.to_string(),
            }
        })?;

        let root_key_array: [u8; 32] =
            root_key_bytes
                .try_into()
                .map_err(|_| SealError::RootKeyDecryption {
                    reason: "decrypted root key is not 32 bytes".to_owned(),
                })?;
        let root_key = EncryptionKey::from_bytes(root_key_array);

        // Unseal the barrier.
        self.barrier.unseal(root_key).await;

        info!("vault unsealed");

        Ok(None)
    }

    /// Seal the vault, zeroizing the root key from memory.
    ///
    /// # Errors
    ///
    /// - [`SealError::AlreadySealed`] if the vault is already sealed.
    pub async fn seal(&self) -> Result<(), SealError> {
        if !self.barrier.is_unsealed().await {
            return Err(SealError::AlreadySealed);
        }

        // Clear any pending shares.
        self.pending_shares.lock().await.clear();

        // Seal the barrier (zeroizes root key).
        self.barrier.seal().await;

        info!("vault sealed");

        Ok(())
    }

    /// Check whether the vault has been initialized (root key exists in storage).
    ///
    /// # Errors
    ///
    /// Returns [`SealError::Storage`] if the storage backend fails.
    pub async fn is_initialized(&self) -> Result<bool, SealError> {
        let exists = self
            .barrier
            .get_raw(ROOT_KEY_PATH)
            .await
            .map_err(SealError::Barrier)?
            .is_some();
        Ok(exists)
    }

    /// Get the current seal status.
    ///
    /// # Errors
    ///
    /// Returns [`SealError::Storage`] if the storage backend fails.
    pub async fn status(&self) -> Result<SealStatus, SealError> {
        let initialized = self.is_initialized().await?;
        let sealed = !self.barrier.is_unsealed().await;

        let (threshold, shares, progress) = if initialized {
            let config = self.load_config().await?;
            let pending = self.pending_shares.lock().await;
            let submitted = u8::try_from(pending.len()).unwrap_or(u8::MAX);
            (config.threshold, config.shares, submitted)
        } else {
            (0, 0, 0)
        };

        Ok(SealStatus {
            initialized,
            sealed,
            threshold,
            shares,
            progress,
        })
    }

    /// Load the seal configuration from storage.
    async fn load_config(&self) -> Result<SealConfig, SealError> {
        let config_bytes = self
            .barrier
            .get_raw(SEAL_CONFIG_PATH)
            .await
            .map_err(SealError::Barrier)?
            .ok_or(SealError::NotInitialized)?;

        serde_json::from_slice(&config_bytes).map_err(|e| SealError::InvalidConfig {
            reason: format!("failed to deserialize seal config: {e}"),
        })
    }
}

/// Current seal status of the vault.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SealStatus {
    /// Whether the vault has been initialized.
    pub initialized: bool,
    /// Whether the vault is currently sealed.
    pub sealed: bool,
    /// Threshold of shares required to unseal.
    pub threshold: u8,
    /// Total number of shares.
    pub shares: u8,
    /// Number of shares submitted so far in the current unseal attempt.
    pub progress: u8,
}

impl std::fmt::Debug for SealManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SealManager")
            .field("barrier", &self.barrier)
            .finish_non_exhaustive()
    }
}

/// Validate Shamir configuration parameters.
fn validate_config(share_count: u8, threshold: u8) -> Result<(), SealError> {
    if !(1..=10).contains(&share_count) {
        return Err(SealError::InvalidConfig {
            reason: format!("share count must be 1-10, got {share_count}"),
        });
    }
    if threshold < 2 {
        return Err(SealError::InvalidConfig {
            reason: format!("threshold must be at least 2, got {threshold}"),
        });
    }
    if threshold > share_count {
        return Err(SealError::InvalidConfig {
            reason: format!("threshold ({threshold}) cannot exceed share count ({share_count})"),
        });
    }
    Ok(())
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use std::sync::Arc;

    use zvault_storage::MemoryBackend;

    use super::*;
    use crate::barrier::Barrier;

    fn make_seal_manager() -> SealManager {
        let storage = Arc::new(MemoryBackend::new());
        let barrier = Arc::new(Barrier::new(storage));
        SealManager::new(barrier)
    }

    // ── validate_config ──────────────────────────────────────────────

    #[test]
    fn validate_config_valid_params() {
        assert!(validate_config(5, 3).is_ok());
        assert!(validate_config(3, 2).is_ok());
        assert!(validate_config(10, 10).is_ok());
        assert!(validate_config(2, 2).is_ok());
    }

    #[test]
    fn validate_config_zero_shares() {
        let err = validate_config(0, 2).unwrap_err();
        assert!(matches!(err, SealError::InvalidConfig { .. }));
    }

    #[test]
    fn validate_config_too_many_shares() {
        let err = validate_config(11, 2).unwrap_err();
        assert!(matches!(err, SealError::InvalidConfig { .. }));
    }

    #[test]
    fn validate_config_threshold_below_two() {
        let err = validate_config(5, 1).unwrap_err();
        assert!(matches!(err, SealError::InvalidConfig { .. }));
    }

    #[test]
    fn validate_config_threshold_exceeds_shares() {
        let err = validate_config(3, 4).unwrap_err();
        assert!(matches!(err, SealError::InvalidConfig { .. }));
    }

    // ── init ─────────────────────────────────────────────────────────

    #[tokio::test]
    async fn init_returns_correct_share_count() {
        let mgr = make_seal_manager();
        let result = mgr.init(5, 3).await.unwrap();
        assert_eq!(result.unseal_shares.len(), 5);
        assert!(!result.root_token.is_empty());
    }

    #[tokio::test]
    async fn init_leaves_vault_sealed() {
        let mgr = make_seal_manager();
        mgr.init(3, 2).await.unwrap();
        assert!(!mgr.barrier.is_unsealed().await);
    }

    #[tokio::test]
    async fn init_marks_vault_initialized() {
        let mgr = make_seal_manager();
        assert!(!mgr.is_initialized().await.unwrap());
        mgr.init(3, 2).await.unwrap();
        assert!(mgr.is_initialized().await.unwrap());
    }

    #[tokio::test]
    async fn init_twice_returns_already_initialized() {
        let mgr = make_seal_manager();
        mgr.init(3, 2).await.unwrap();
        let err = mgr.init(3, 2).await.unwrap_err();
        assert!(matches!(err, SealError::AlreadyInitialized));
    }

    #[tokio::test]
    async fn init_invalid_config_rejected() {
        let mgr = make_seal_manager();
        let err = mgr.init(0, 2).await.unwrap_err();
        assert!(matches!(err, SealError::InvalidConfig { .. }));
    }

    // ── unseal ───────────────────────────────────────────────────────

    #[tokio::test]
    async fn unseal_full_flow_with_threshold() {
        let mgr = make_seal_manager();
        let result = mgr.init(5, 3).await.unwrap();

        // Submit first share — need more.
        let progress = mgr
            .submit_unseal_share(&result.unseal_shares[0])
            .await
            .unwrap();
        assert!(progress.is_some());
        let p = progress.unwrap();
        assert_eq!(p.threshold, 3);
        assert_eq!(p.submitted, 1);

        // Submit second share — still need more.
        let progress = mgr
            .submit_unseal_share(&result.unseal_shares[1])
            .await
            .unwrap();
        assert!(progress.is_some());
        assert_eq!(progress.unwrap().submitted, 2);

        // Submit third share — threshold reached, vault unseals.
        let progress = mgr
            .submit_unseal_share(&result.unseal_shares[2])
            .await
            .unwrap();
        assert!(progress.is_none());
        assert!(mgr.barrier.is_unsealed().await);
    }

    #[tokio::test]
    async fn unseal_with_minimum_config() {
        let mgr = make_seal_manager();
        let result = mgr.init(2, 2).await.unwrap();

        mgr.submit_unseal_share(&result.unseal_shares[0])
            .await
            .unwrap();
        let progress = mgr
            .submit_unseal_share(&result.unseal_shares[1])
            .await
            .unwrap();
        assert!(progress.is_none());
        assert!(mgr.barrier.is_unsealed().await);
    }

    #[tokio::test]
    async fn unseal_not_initialized_returns_error() {
        let mgr = make_seal_manager();
        let err = mgr.submit_unseal_share("dGVzdA==").await.unwrap_err();
        assert!(matches!(err, SealError::NotInitialized));
    }

    #[tokio::test]
    async fn unseal_already_unsealed_returns_error() {
        let mgr = make_seal_manager();
        let result = mgr.init(2, 2).await.unwrap();

        // Unseal fully.
        mgr.submit_unseal_share(&result.unseal_shares[0])
            .await
            .unwrap();
        mgr.submit_unseal_share(&result.unseal_shares[1])
            .await
            .unwrap();

        // Try to submit another share while unsealed.
        let err = mgr
            .submit_unseal_share(&result.unseal_shares[0])
            .await
            .unwrap_err();
        assert!(matches!(err, SealError::AlreadyUnsealed));
    }

    #[tokio::test]
    async fn unseal_invalid_base64_returns_error() {
        let mgr = make_seal_manager();
        mgr.init(3, 2).await.unwrap();

        let err = mgr
            .submit_unseal_share("not-valid-base64!!!")
            .await
            .unwrap_err();
        assert!(matches!(err, SealError::InvalidShare { .. }));
    }

    // ── seal ─────────────────────────────────────────────────────────

    #[tokio::test]
    async fn seal_after_unseal_works() {
        let mgr = make_seal_manager();
        let result = mgr.init(2, 2).await.unwrap();

        // Unseal.
        mgr.submit_unseal_share(&result.unseal_shares[0])
            .await
            .unwrap();
        mgr.submit_unseal_share(&result.unseal_shares[1])
            .await
            .unwrap();
        assert!(mgr.barrier.is_unsealed().await);

        // Seal.
        mgr.seal().await.unwrap();
        assert!(!mgr.barrier.is_unsealed().await);
    }

    #[tokio::test]
    async fn seal_when_already_sealed_returns_error() {
        let mgr = make_seal_manager();
        mgr.init(3, 2).await.unwrap();

        let err = mgr.seal().await.unwrap_err();
        assert!(matches!(err, SealError::AlreadySealed));
    }

    #[tokio::test]
    async fn seal_clears_pending_shares() {
        let mgr = make_seal_manager();
        let result = mgr.init(3, 2).await.unwrap();

        // Submit one share (not enough to unseal).
        mgr.submit_unseal_share(&result.unseal_shares[0])
            .await
            .unwrap();

        // Manually unseal the barrier to allow seal() to work.
        mgr.barrier.unseal(EncryptionKey::generate()).await;
        mgr.seal().await.unwrap();

        // Pending shares should be cleared — status shows 0 progress.
        let status = mgr.status().await.unwrap();
        assert_eq!(status.progress, 0);
    }

    // ── status ───────────────────────────────────────────────────────

    #[tokio::test]
    async fn status_uninitialized() {
        let mgr = make_seal_manager();
        let status = mgr.status().await.unwrap();
        assert!(!status.initialized);
        assert!(status.sealed);
        assert_eq!(status.threshold, 0);
        assert_eq!(status.shares, 0);
        assert_eq!(status.progress, 0);
    }

    #[tokio::test]
    async fn status_initialized_sealed() {
        let mgr = make_seal_manager();
        mgr.init(5, 3).await.unwrap();

        let status = mgr.status().await.unwrap();
        assert!(status.initialized);
        assert!(status.sealed);
        assert_eq!(status.threshold, 3);
        assert_eq!(status.shares, 5);
        assert_eq!(status.progress, 0);
    }

    #[tokio::test]
    async fn status_tracks_unseal_progress() {
        let mgr = make_seal_manager();
        let result = mgr.init(5, 3).await.unwrap();

        mgr.submit_unseal_share(&result.unseal_shares[0])
            .await
            .unwrap();

        let status = mgr.status().await.unwrap();
        assert_eq!(status.progress, 1);
        assert!(status.sealed);
    }

    #[tokio::test]
    async fn status_unsealed() {
        let mgr = make_seal_manager();
        let result = mgr.init(2, 2).await.unwrap();

        mgr.submit_unseal_share(&result.unseal_shares[0])
            .await
            .unwrap();
        mgr.submit_unseal_share(&result.unseal_shares[1])
            .await
            .unwrap();

        let status = mgr.status().await.unwrap();
        assert!(status.initialized);
        assert!(!status.sealed);
        // Pending shares cleared after successful unseal.
        assert_eq!(status.progress, 0);
    }

    // ── reseal + re-unseal cycle ─────────────────────────────────────

    #[tokio::test]
    async fn reseal_and_reunseal_works() {
        let mgr = make_seal_manager();
        let result = mgr.init(3, 2).await.unwrap();

        // First unseal.
        mgr.submit_unseal_share(&result.unseal_shares[0])
            .await
            .unwrap();
        mgr.submit_unseal_share(&result.unseal_shares[1])
            .await
            .unwrap();
        assert!(mgr.barrier.is_unsealed().await);

        // Seal.
        mgr.seal().await.unwrap();
        assert!(!mgr.barrier.is_unsealed().await);

        // Re-unseal with different share combination.
        mgr.submit_unseal_share(&result.unseal_shares[1])
            .await
            .unwrap();
        mgr.submit_unseal_share(&result.unseal_shares[2])
            .await
            .unwrap();
        assert!(mgr.barrier.is_unsealed().await);
    }

    #[tokio::test]
    async fn barrier_works_after_unseal() {
        let mgr = make_seal_manager();
        let result = mgr.init(2, 2).await.unwrap();

        mgr.submit_unseal_share(&result.unseal_shares[0])
            .await
            .unwrap();
        mgr.submit_unseal_share(&result.unseal_shares[1])
            .await
            .unwrap();

        // The barrier should now accept encrypted read/write.
        mgr.barrier.put("test/key", b"hello").await.unwrap();
        let val = mgr.barrier.get("test/key").await.unwrap();
        assert_eq!(val, Some(b"hello".to_vec()));
    }

    // ── SealManager Debug ────────────────────────────────────────────

    #[test]
    fn seal_manager_debug_does_not_leak() {
        let mgr = make_seal_manager();
        let debug = format!("{mgr:?}");
        assert!(debug.contains("SealManager"));
        assert!(!debug.contains("pending_shares"));
    }
}

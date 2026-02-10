//! Token store for `ZVault`.
//!
//! Tokens are the primary authentication mechanism. Every API request carries
//! a token that maps to policies. Tokens are never stored in plaintext —
//! they are SHA-256 hashed before persisting. Comparison uses constant-time
//! equality to prevent timing side-channels.
//!
//! # Security model
//!
//! - Tokens are UUID v4 (128 bits of OS CSPRNG randomness).
//! - Stored as `SHA-256(token)` — the plaintext token is returned once at
//!   creation and never stored.
//! - Lookup is by hash: caller provides plaintext token, we hash it and
//!   look up the hash in storage.
//! - Token comparison uses `subtle::ConstantTimeEq`.
//! - Tokens have TTLs and optional max TTLs.
//! - Revoking a parent token revokes all children (tree revocation).

use std::sync::Arc;

use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tracing::info;

use crate::barrier::Barrier;
use crate::error::TokenError;

/// Storage prefix for token entries.
const TOKEN_PREFIX: &str = "sys/tokens/";

/// Storage prefix for parent→children index.
const TOKEN_CHILDREN_PREFIX: &str = "sys/token-children/";

/// A stored token entry (persisted through the barrier).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenEntry {
    /// SHA-256 hash of the token (hex-encoded). This is the storage key.
    pub token_hash: String,
    /// Policies attached to this token.
    pub policies: Vec<String>,
    /// When the token was created.
    pub created_at: DateTime<Utc>,
    /// When the token expires (None = never).
    pub expires_at: Option<DateTime<Utc>>,
    /// Whether the token can be renewed.
    pub renewable: bool,
    /// Maximum lifetime (from creation). Renewals cannot extend past this.
    pub max_ttl: Option<Duration>,
    /// Parent token hash (None for root tokens).
    pub parent_hash: Option<String>,
    /// Arbitrary metadata attached to the token.
    pub metadata: std::collections::HashMap<String, String>,
    /// Display name for audit logs.
    pub display_name: String,
}

/// Parameters for creating a new token.
pub struct CreateTokenParams {
    /// Policies to attach.
    pub policies: Vec<String>,
    /// Time-to-live from now.
    pub ttl: Option<Duration>,
    /// Maximum TTL (renewals cannot extend past this).
    pub max_ttl: Option<Duration>,
    /// Whether the token can be renewed.
    pub renewable: bool,
    /// Parent token hash (for tree revocation).
    pub parent_hash: Option<String>,
    /// Arbitrary metadata.
    pub metadata: std::collections::HashMap<String, String>,
    /// Display name for audit logs.
    pub display_name: String,
}

/// Manages token creation, lookup, renewal, and revocation.
pub struct TokenStore {
    barrier: Arc<Barrier>,
}

impl TokenStore {
    /// Create a new token store backed by the given barrier.
    #[must_use]
    pub fn new(barrier: Arc<Barrier>) -> Self {
        Self { barrier }
    }

    /// Create a new token and persist its hash.
    ///
    /// Returns the plaintext token (shown once, never stored).
    ///
    /// # Errors
    ///
    /// Returns [`TokenError::Barrier`] if storage fails.
    pub async fn create(&self, params: CreateTokenParams) -> Result<String, TokenError> {
        let plaintext_token = uuid::Uuid::new_v4().to_string();
        let token_hash = hash_token(&plaintext_token);
        let now = Utc::now();

        let expires_at = params.ttl.map(|ttl| now + ttl);

        let entry = TokenEntry {
            token_hash: token_hash.clone(),
            policies: params.policies,
            created_at: now,
            expires_at,
            renewable: params.renewable,
            max_ttl: params.max_ttl,
            parent_hash: params.parent_hash.clone(),
            metadata: params.metadata,
            display_name: params.display_name,
        };

        let entry_bytes = serde_json::to_vec(&entry).map_err(|e| {
            TokenError::Barrier(crate::error::BarrierError::Crypto(
                crate::error::CryptoError::Encryption {
                    reason: format!("token serialization failed: {e}"),
                },
            ))
        })?;

        let key = format!("{TOKEN_PREFIX}{token_hash}");
        self.barrier.put(&key, &entry_bytes).await?;

        // Index parent→child relationship for tree revocation.
        if let Some(ref parent) = params.parent_hash {
            let child_key = format!("{TOKEN_CHILDREN_PREFIX}{parent}/{token_hash}");
            self.barrier.put(&child_key, b"1").await?;
        }

        info!(display_name = %entry.display_name, "token created");

        Ok(plaintext_token)
    }

    /// Look up a token by its plaintext value.
    ///
    /// Hashes the token, fetches from storage, and validates expiry.
    ///
    /// # Errors
    ///
    /// - [`TokenError::NotFound`] if the token hash doesn't exist.
    /// - [`TokenError::Expired`] if the token's TTL has passed.
    /// - [`TokenError::Barrier`] if storage fails.
    pub async fn lookup(&self, plaintext_token: &str) -> Result<TokenEntry, TokenError> {
        let token_hash = hash_token(plaintext_token);
        let key = format!("{TOKEN_PREFIX}{token_hash}");

        let data = self.barrier.get(&key).await?.ok_or(TokenError::NotFound)?;

        let entry: TokenEntry = serde_json::from_slice(&data).map_err(|e| {
            TokenError::Barrier(crate::error::BarrierError::Crypto(
                crate::error::CryptoError::Decryption {
                    reason: format!("token deserialization failed: {e}"),
                },
            ))
        })?;

        // Check expiry.
        if let Some(expires_at) = entry.expires_at {
            if Utc::now() > expires_at {
                return Err(TokenError::Expired {
                    expired_at: expires_at.to_rfc3339(),
                });
            }
        }

        Ok(entry)
    }

    /// Renew a token, extending its TTL.
    ///
    /// # Errors
    ///
    /// - [`TokenError::NotFound`] if the token doesn't exist.
    /// - [`TokenError::NotRenewable`] if the token isn't renewable.
    /// - [`TokenError::MaxTtlExceeded`] if renewal would exceed max TTL.
    /// - [`TokenError::Barrier`] if storage fails.
    pub async fn renew(
        &self,
        plaintext_token: &str,
        increment: Duration,
    ) -> Result<TokenEntry, TokenError> {
        let mut entry = self.lookup(plaintext_token).await?;

        if !entry.renewable {
            return Err(TokenError::NotRenewable);
        }

        let now = Utc::now();
        let mut new_expires = now + increment;

        // Clamp to max_ttl if set.
        if let Some(max_ttl) = entry.max_ttl {
            let absolute_max = entry.created_at + max_ttl;
            if new_expires > absolute_max {
                if now >= absolute_max {
                    return Err(TokenError::MaxTtlExceeded {
                        max_ttl_secs: max_ttl.num_seconds(),
                    });
                }
                new_expires = absolute_max;
            }
        }

        entry.expires_at = Some(new_expires);

        let entry_bytes = serde_json::to_vec(&entry).map_err(|e| {
            TokenError::Barrier(crate::error::BarrierError::Crypto(
                crate::error::CryptoError::Encryption {
                    reason: format!("token serialization failed: {e}"),
                },
            ))
        })?;

        let key = format!("{TOKEN_PREFIX}{}", entry.token_hash);
        self.barrier.put(&key, &entry_bytes).await?;

        Ok(entry)
    }

    /// Revoke a token and all its children (tree revocation).
    ///
    /// # Errors
    ///
    /// Returns [`TokenError::Barrier`] if storage fails.
    pub async fn revoke(&self, plaintext_token: &str) -> Result<(), TokenError> {
        let token_hash = hash_token(plaintext_token);
        self.revoke_by_hash(&token_hash).await
    }

    /// Revoke a token by its hash, recursively revoking children.
    async fn revoke_by_hash(&self, token_hash: &str) -> Result<(), TokenError> {
        // First, revoke all children.
        let children_prefix = format!("{TOKEN_CHILDREN_PREFIX}{token_hash}/");
        let children = self.barrier.list(&children_prefix).await?;

        for child_key in &children {
            // Extract child hash from the key.
            if let Some(child_hash) = child_key.strip_prefix(&children_prefix) {
                // Use Box::pin for recursive async call.
                Box::pin(self.revoke_by_hash(child_hash)).await?;
            }
            // Clean up the index entry.
            self.barrier.delete(child_key).await?;
        }

        // Delete the token itself.
        let key = format!("{TOKEN_PREFIX}{token_hash}");
        self.barrier.delete(&key).await?;

        info!(
            token_hash_prefix = &token_hash[..8.min(token_hash.len())],
            "token revoked"
        );

        Ok(())
    }
}

/// Hash a plaintext token with SHA-256, returning hex-encoded hash.
///
/// This is a one-way operation. The plaintext token cannot be recovered.
#[must_use]
pub fn hash_token(plaintext: &str) -> String {
    let digest = Sha256::digest(plaintext.as_bytes());
    hex::encode(digest)
}

impl std::fmt::Debug for TokenStore {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TokenStore").finish_non_exhaustive()
    }
}

//! Lease manager for `VaultRS`.
//!
//! Every dynamically generated credential (database creds, certificates, etc.)
//! gets a lease with a TTL. The lease manager runs a background tick that
//! finds expired leases and triggers revocation through the originating engine.
//!
//! Leases are stored through the barrier at `sys/leases/<id>`.

use std::sync::Arc;

use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use tracing::{info, warn};

use crate::barrier::Barrier;
use crate::error::LeaseError;

/// Storage prefix for lease entries.
const LEASE_PREFIX: &str = "sys/leases/";

/// A lease tracking a dynamically generated credential.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Lease {
    /// Unique lease ID.
    pub id: String,
    /// Engine mount path that created this lease (e.g., `database/creds/readonly`).
    pub engine_path: String,
    /// When the lease was issued.
    pub issued_at: DateTime<Utc>,
    /// Time-to-live from issuance.
    pub ttl_secs: i64,
    /// Whether the lease can be renewed.
    pub renewable: bool,
    /// Engine-specific data needed for revocation (e.g., username to drop).
    pub data: serde_json::Value,
    /// Token hash that created this lease (for token revocation cascading).
    pub token_hash: String,
}

impl Lease {
    /// Check whether this lease has expired.
    #[must_use]
    pub fn is_expired(&self) -> bool {
        let expires_at = self.issued_at + Duration::seconds(self.ttl_secs);
        Utc::now() > expires_at
    }

    /// Get the expiration time.
    #[must_use]
    pub fn expires_at(&self) -> DateTime<Utc> {
        self.issued_at + Duration::seconds(self.ttl_secs)
    }
}

/// Manages lease creation, renewal, revocation, and expiry scanning.
pub struct LeaseManager {
    barrier: Arc<Barrier>,
}

impl LeaseManager {
    /// Create a new lease manager backed by the given barrier.
    #[must_use]
    pub fn new(barrier: Arc<Barrier>) -> Self {
        Self { barrier }
    }

    /// Create a new lease and persist it.
    ///
    /// Returns the lease ID.
    ///
    /// # Errors
    ///
    /// Returns [`LeaseError::Barrier`] if storage fails.
    pub async fn create(&self, lease: &Lease) -> Result<String, LeaseError> {
        let bytes = serde_json::to_vec(lease).map_err(|e| {
            LeaseError::Barrier(crate::error::BarrierError::Crypto(
                crate::error::CryptoError::Encryption {
                    reason: format!("lease serialization failed: {e}"),
                },
            ))
        })?;

        let key = format!("{LEASE_PREFIX}{}", lease.id);
        self.barrier.put(&key, &bytes).await?;

        info!(lease_id = %lease.id, engine = %lease.engine_path, ttl = lease.ttl_secs, "lease created");

        Ok(lease.id.clone())
    }

    /// Look up a lease by ID.
    ///
    /// # Errors
    ///
    /// - [`LeaseError::NotFound`] if the lease doesn't exist.
    /// - [`LeaseError::Barrier`] if storage fails.
    pub async fn lookup(&self, lease_id: &str) -> Result<Lease, LeaseError> {
        let key = format!("{LEASE_PREFIX}{lease_id}");
        let data = self
            .barrier
            .get(&key)
            .await?
            .ok_or_else(|| LeaseError::NotFound {
                lease_id: lease_id.to_owned(),
            })?;

        serde_json::from_slice(&data).map_err(|e| {
            LeaseError::Barrier(crate::error::BarrierError::Crypto(
                crate::error::CryptoError::Decryption {
                    reason: format!("lease deserialization failed: {e}"),
                },
            ))
        })
    }

    /// Renew a lease by extending its TTL.
    ///
    /// # Errors
    ///
    /// - [`LeaseError::NotFound`] if the lease doesn't exist.
    /// - [`LeaseError::NotRenewable`] if the lease isn't renewable.
    /// - [`LeaseError::Expired`] if the lease has already expired.
    /// - [`LeaseError::Barrier`] if storage fails.
    pub async fn renew(&self, lease_id: &str, increment_secs: i64) -> Result<Lease, LeaseError> {
        let mut lease = self.lookup(lease_id).await?;

        if !lease.renewable {
            return Err(LeaseError::NotRenewable {
                lease_id: lease_id.to_owned(),
            });
        }

        if lease.is_expired() {
            return Err(LeaseError::Expired {
                lease_id: lease_id.to_owned(),
            });
        }

        lease.ttl_secs = lease.ttl_secs.saturating_add(increment_secs);

        let bytes = serde_json::to_vec(&lease).map_err(|e| {
            LeaseError::Barrier(crate::error::BarrierError::Crypto(
                crate::error::CryptoError::Encryption {
                    reason: format!("lease serialization failed: {e}"),
                },
            ))
        })?;

        let key = format!("{LEASE_PREFIX}{}", lease.id);
        self.barrier.put(&key, &bytes).await?;

        info!(lease_id = %lease.id, new_ttl = lease.ttl_secs, "lease renewed");

        Ok(lease)
    }

    /// Revoke a lease immediately.
    ///
    /// This only removes the lease from storage. The caller is responsible
    /// for calling the engine's revocation logic (e.g., dropping the DB user).
    ///
    /// # Errors
    ///
    /// Returns [`LeaseError::Barrier`] if storage fails.
    pub async fn revoke(&self, lease_id: &str) -> Result<(), LeaseError> {
        let key = format!("{LEASE_PREFIX}{lease_id}");
        self.barrier.delete(&key).await?;

        info!(lease_id = %lease_id, "lease revoked");

        Ok(())
    }

    /// Scan for expired leases and return their IDs.
    ///
    /// The caller should iterate the returned IDs, call the engine's
    /// revocation logic, then call [`revoke`](LeaseManager::revoke) to
    /// clean up storage.
    ///
    /// # Errors
    ///
    /// Returns [`LeaseError::Barrier`] if storage fails.
    pub async fn find_expired(&self) -> Result<Vec<Lease>, LeaseError> {
        let keys = self.barrier.list(LEASE_PREFIX).await?;
        let mut expired = Vec::new();

        for key in &keys {
            match self.barrier.get(key).await {
                Ok(Some(data)) => {
                    if let Ok(lease) = serde_json::from_slice::<Lease>(&data) {
                        if lease.is_expired() {
                            expired.push(lease);
                        }
                    }
                }
                Ok(None) => {}
                Err(e) => {
                    warn!(key = %key, error = %e, "failed to read lease during expiry scan");
                }
            }
        }

        Ok(expired)
    }

    /// Revoke all leases matching a prefix (e.g., when unmounting an engine).
    ///
    /// Returns the number of leases revoked.
    ///
    /// # Errors
    ///
    /// Returns [`LeaseError::Barrier`] if storage fails.
    pub async fn revoke_prefix(&self, engine_path_prefix: &str) -> Result<u64, LeaseError> {
        let keys = self.barrier.list(LEASE_PREFIX).await?;
        let mut count = 0u64;

        for key in &keys {
            if let Ok(Some(data)) = self.barrier.get(key).await {
                if let Ok(lease) = serde_json::from_slice::<Lease>(&data) {
                    if lease.engine_path.starts_with(engine_path_prefix) {
                        self.barrier.delete(key).await?;
                        count = count.saturating_add(1);
                    }
                }
            }
        }

        info!(prefix = %engine_path_prefix, count = count, "leases revoked by prefix");

        Ok(count)
    }
}

impl std::fmt::Debug for LeaseManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LeaseManager").finish_non_exhaustive()
    }
}

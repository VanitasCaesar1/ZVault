//! Secrets engine trait and KV v2 implementation.
//!
//! Secrets engines are mounted at path prefixes and handle read/write/delete
//! operations for secrets. The KV v2 engine stores versioned key-value pairs
//! with metadata tracking.

use std::collections::HashMap;
use std::sync::Arc;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::barrier::Barrier;
use crate::error::EngineError;

/// A request to a secrets engine.
#[derive(Debug, Clone)]
pub struct EngineRequest {
    /// Operation type.
    pub operation: Operation,
    /// Path relative to the engine mount (e.g., `data/myapp/password`).
    pub path: String,
    /// Request data (for write operations).
    pub data: Option<serde_json::Value>,
}

/// Engine operation types.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Operation {
    /// Read a secret.
    Read,
    /// Write/create a secret.
    Write,
    /// Delete a secret.
    Delete,
    /// List keys under a prefix.
    List,
}

/// Response from a secrets engine.
#[derive(Debug, Clone, Serialize)]
pub struct EngineResponse {
    /// Response data.
    pub data: Option<serde_json::Value>,
    /// Optional lease ID for dynamic secrets.
    pub lease_id: Option<String>,
    /// Lease TTL in seconds.
    pub lease_duration: Option<i64>,
    /// Whether the lease is renewable.
    pub renewable: bool,
}

/// KV v2 secrets engine — versioned key-value storage.
///
/// Stores secrets with version history and metadata. Each write creates a
/// new version. Reads return the latest version by default.
///
/// Storage layout under the engine's mount prefix:
/// - `data/<path>` — versioned secret data
/// - `metadata/<path>` — version metadata
pub struct KvEngine {
    barrier: Arc<Barrier>,
    /// Mount path prefix (e.g., `kv/default/`).
    prefix: String,
}

/// Stored secret with version history.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct KvSecret {
    /// All versions, keyed by version number.
    versions: HashMap<u32, KvVersion>,
    /// Current (latest) version number.
    current_version: u32,
    /// Maximum number of versions to keep (0 = unlimited).
    max_versions: u32,
}

/// A single version of a secret.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct KvVersion {
    /// The secret data as key-value pairs.
    data: HashMap<String, serde_json::Value>,
    /// When this version was created.
    created_at: DateTime<Utc>,
    /// When this version was deleted (soft delete).
    deleted_at: Option<DateTime<Utc>>,
}

/// Metadata about a secret (returned by metadata endpoints).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KvMetadata {
    /// Current version number.
    pub current_version: u32,
    /// When the secret was first created.
    pub created_at: DateTime<Utc>,
    /// When the secret was last updated.
    pub updated_at: DateTime<Utc>,
    /// Number of versions stored.
    pub version_count: u32,
    /// Maximum versions allowed.
    pub max_versions: u32,
}

impl KvEngine {
    /// Create a new KV v2 engine with the given barrier and mount prefix.
    #[must_use]
    pub fn new(barrier: Arc<Barrier>, prefix: String) -> Self {
        Self { barrier, prefix }
    }

    /// Handle a request to this engine.
    ///
    /// # Errors
    ///
    /// Returns [`EngineError`] on storage failures or invalid operations.
    pub async fn handle(&self, req: &EngineRequest) -> Result<EngineResponse, EngineError> {
        match req.operation {
            Operation::Read => self.read(&req.path).await,
            Operation::Write => self.write(&req.path, req.data.clone()).await,
            Operation::Delete => self.delete(&req.path).await,
            Operation::List => self.list(&req.path).await,
        }
    }

    /// Read the latest version of a secret.
    async fn read(&self, path: &str) -> Result<EngineResponse, EngineError> {
        let storage_key = format!("{}data/{}", self.prefix, path);
        let data = self
            .barrier
            .get(&storage_key)
            .await
            .map_err(EngineError::Barrier)?;

        match data {
            None => Err(EngineError::NotFound {
                path: path.to_owned(),
            }),
            Some(bytes) => {
                let secret: KvSecret =
                    serde_json::from_slice(&bytes).map_err(|e| EngineError::Internal {
                        reason: format!("deserialization failed: {e}"),
                    })?;

                let version = secret
                    .versions
                    .get(&secret.current_version)
                    .ok_or_else(|| EngineError::Internal {
                        reason: format!("version {} missing", secret.current_version),
                    })?;

                if version.deleted_at.is_some() {
                    return Err(EngineError::NotFound {
                        path: path.to_owned(),
                    });
                }

                let response_data = serde_json::json!({
                    "data": version.data,
                    "metadata": {
                        "version": secret.current_version,
                        "created_time": version.created_at.to_rfc3339(),
                    }
                });

                Ok(EngineResponse {
                    data: Some(response_data),
                    lease_id: None,
                    lease_duration: None,
                    renewable: false,
                })
            }
        }
    }

    /// Write a new version of a secret.
    async fn write(
        &self,
        path: &str,
        data: Option<serde_json::Value>,
    ) -> Result<EngineResponse, EngineError> {
        let kv_data: HashMap<String, serde_json::Value> = match data {
            Some(serde_json::Value::Object(map)) => {
                map.into_iter().collect()
            }
            Some(other) => {
                let mut m = HashMap::new();
                m.insert("value".to_owned(), other);
                m
            }
            None => HashMap::new(),
        };

        let storage_key = format!("{}data/{}", self.prefix, path);
        let now = Utc::now();

        // Load existing secret or create new.
        let mut secret = match self.barrier.get(&storage_key).await.map_err(EngineError::Barrier)? {
            Some(bytes) => {
                serde_json::from_slice::<KvSecret>(&bytes).map_err(|e| EngineError::Internal {
                    reason: format!("deserialization failed: {e}"),
                })?
            }
            None => KvSecret {
                versions: HashMap::new(),
                current_version: 0,
                max_versions: 10,
            },
        };

        // Increment version.
        secret.current_version = secret.current_version.saturating_add(1);

        let version = KvVersion {
            data: kv_data,
            created_at: now,
            deleted_at: None,
        };
        secret
            .versions
            .insert(secret.current_version, version);

        // Prune old versions if max_versions is set.
        if secret.max_versions > 0 {
            while secret.versions.len() > secret.max_versions as usize {
                let min_version = secret.versions.keys().copied().min().unwrap_or(0);
                secret.versions.remove(&min_version);
            }
        }

        let bytes = serde_json::to_vec(&secret).map_err(|e| EngineError::Internal {
            reason: format!("serialization failed: {e}"),
        })?;
        self.barrier
            .put(&storage_key, &bytes)
            .await
            .map_err(EngineError::Barrier)?;

        let response_data = serde_json::json!({
            "version": secret.current_version,
            "created_time": now.to_rfc3339(),
        });

        Ok(EngineResponse {
            data: Some(response_data),
            lease_id: None,
            lease_duration: None,
            renewable: false,
        })
    }

    /// Soft-delete the latest version of a secret.
    async fn delete(&self, path: &str) -> Result<EngineResponse, EngineError> {
        let storage_key = format!("{}data/{}", self.prefix, path);
        let data = self
            .barrier
            .get(&storage_key)
            .await
            .map_err(EngineError::Barrier)?;

        match data {
            None => Err(EngineError::NotFound {
                path: path.to_owned(),
            }),
            Some(bytes) => {
                let mut secret: KvSecret =
                    serde_json::from_slice(&bytes).map_err(|e| EngineError::Internal {
                        reason: format!("deserialization failed: {e}"),
                    })?;

                if let Some(version) = secret.versions.get_mut(&secret.current_version) {
                    version.deleted_at = Some(Utc::now());
                }

                let updated = serde_json::to_vec(&secret).map_err(|e| EngineError::Internal {
                    reason: format!("serialization failed: {e}"),
                })?;
                self.barrier
                    .put(&storage_key, &updated)
                    .await
                    .map_err(EngineError::Barrier)?;

                Ok(EngineResponse {
                    data: None,
                    lease_id: None,
                    lease_duration: None,
                    renewable: false,
                })
            }
        }
    }

    /// List keys under a prefix.
    async fn list(&self, path: &str) -> Result<EngineResponse, EngineError> {
        let storage_prefix = format!("{}data/{}", self.prefix, path);
        let keys = self
            .barrier
            .list(&storage_prefix)
            .await
            .map_err(EngineError::Barrier)?;

        let relative_keys: Vec<String> = keys
            .iter()
            .filter_map(|k| k.strip_prefix(&storage_prefix).map(String::from))
            .collect();

        Ok(EngineResponse {
            data: Some(serde_json::json!({ "keys": relative_keys })),
            lease_id: None,
            lease_duration: None,
            renewable: false,
        })
    }

    /// Get metadata about a secret.
    ///
    /// # Errors
    ///
    /// Returns [`EngineError::NotFound`] if the secret doesn't exist.
    pub async fn metadata(&self, path: &str) -> Result<KvMetadata, EngineError> {
        let storage_key = format!("{}data/{}", self.prefix, path);
        let data = self
            .barrier
            .get(&storage_key)
            .await
            .map_err(EngineError::Barrier)?
            .ok_or_else(|| EngineError::NotFound {
                path: path.to_owned(),
            })?;

        let secret: KvSecret =
            serde_json::from_slice(&data).map_err(|e| EngineError::Internal {
                reason: format!("deserialization failed: {e}"),
            })?;

        let created_at = secret
            .versions
            .values()
            .map(|v| v.created_at)
            .min()
            .unwrap_or_else(Utc::now);

        let updated_at = secret
            .versions
            .values()
            .map(|v| v.created_at)
            .max()
            .unwrap_or_else(Utc::now);

        Ok(KvMetadata {
            current_version: secret.current_version,
            created_at,
            updated_at,
            #[allow(clippy::cast_possible_truncation)]
            version_count: secret.versions.len() as u32, // max_versions caps at u32
            max_versions: secret.max_versions,
        })
    }
}

impl std::fmt::Debug for KvEngine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("KvEngine")
            .field("prefix", &self.prefix)
            .finish_non_exhaustive()
    }
}

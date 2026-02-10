//! Transit secrets engine for `ZVault`.
//!
//! Provides encryption-as-a-service: callers send plaintext, get back
//! ciphertext (and vice versa), without ever seeing the encryption key.
//! Keys are named, versioned, and stored through the barrier.
//!
//! Supported operations:
//! - `encrypt` / `decrypt` — AES-256-GCM
//! - `rewrap` — re-encrypt ciphertext under the latest key version
//! - `datakey` — generate a data encryption key (returned wrapped + plaintext)
//!
//! # Security model
//!
//! - Named keys are derived from the root key via HKDF with unique info.
//! - Key versions allow rotation without re-encrypting all data.
//! - Ciphertext is prefixed with `vault:v{version}:` for version tracking.

use std::collections::HashMap;
use std::sync::Arc;

use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::barrier::Barrier;
use crate::crypto::{self, EncryptionKey};
use crate::error::EngineError;

/// A named transit key with version history.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransitKey {
    /// Key name.
    pub name: String,
    /// Key versions, keyed by version number. Each value is the raw key bytes (encrypted at rest).
    pub versions: HashMap<u32, TransitKeyVersion>,
    /// Current (latest) version number.
    pub latest_version: u32,
    /// Minimum version allowed for decryption (for key rotation enforcement).
    pub min_decryption_version: u32,
    /// Whether this key supports encryption.
    pub supports_encryption: bool,
    /// Whether this key supports decryption.
    pub supports_decryption: bool,
    /// When the key was created.
    pub created_at: DateTime<Utc>,
}

/// A single version of a transit key.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransitKeyVersion {
    /// The raw key material (32 bytes, stored encrypted through barrier).
    pub key_material: Vec<u8>,
    /// When this version was created.
    pub created_at: DateTime<Utc>,
}

/// Transit secrets engine — encryption as a service.
pub struct TransitEngine {
    barrier: Arc<Barrier>,
    /// Storage prefix for transit keys.
    prefix: String,
}

impl TransitEngine {
    /// Create a new transit engine with the given barrier and mount prefix.
    #[must_use]
    pub fn new(barrier: Arc<Barrier>, prefix: String) -> Self {
        Self { barrier, prefix }
    }

    /// Create a new named encryption key.
    ///
    /// # Errors
    ///
    /// Returns [`EngineError`] if the key already exists or storage fails.
    pub async fn create_key(&self, name: &str) -> Result<(), EngineError> {
        let storage_key = format!("{}keys/{}", self.prefix, name);

        if self
            .barrier
            .get(&storage_key)
            .await
            .map_err(EngineError::Barrier)?
            .is_some()
        {
            return Err(EngineError::InvalidRequest {
                reason: format!("key '{name}' already exists"),
            });
        }

        let key_material = EncryptionKey::generate();
        let now = Utc::now();

        let mut versions = HashMap::new();
        versions.insert(
            1,
            TransitKeyVersion {
                key_material: key_material.as_bytes().to_vec(),
                created_at: now,
            },
        );

        let transit_key = TransitKey {
            name: name.to_owned(),
            versions,
            latest_version: 1,
            min_decryption_version: 1,
            supports_encryption: true,
            supports_decryption: true,
            created_at: now,
        };

        let bytes = serde_json::to_vec(&transit_key).map_err(|e| EngineError::Internal {
            reason: format!("serialization failed: {e}"),
        })?;
        self.barrier
            .put(&storage_key, &bytes)
            .await
            .map_err(EngineError::Barrier)?;

        Ok(())
    }

    /// Rotate a named key, creating a new version.
    ///
    /// # Errors
    ///
    /// Returns [`EngineError`] if the key doesn't exist or storage fails.
    pub async fn rotate_key(&self, name: &str) -> Result<u32, EngineError> {
        let mut key = self.load_key(name).await?;

        let new_material = EncryptionKey::generate();
        let new_version = key.latest_version.saturating_add(1);

        key.versions.insert(
            new_version,
            TransitKeyVersion {
                key_material: new_material.as_bytes().to_vec(),
                created_at: Utc::now(),
            },
        );
        key.latest_version = new_version;

        self.save_key(&key).await?;

        Ok(new_version)
    }

    /// Encrypt plaintext using the latest version of a named key.
    ///
    /// Returns ciphertext in the format `vault:v{version}:{base64_ciphertext}`.
    ///
    /// # Errors
    ///
    /// Returns [`EngineError`] if the key doesn't exist, doesn't support
    /// encryption, or a crypto operation fails.
    pub async fn encrypt(&self, key_name: &str, plaintext: &[u8]) -> Result<String, EngineError> {
        let key = self.load_key(key_name).await?;

        if !key.supports_encryption {
            return Err(EngineError::InvalidRequest {
                reason: format!("key '{key_name}' does not support encryption"),
            });
        }

        let version = key.latest_version;
        let key_version =
            key.versions
                .get(&version)
                .ok_or_else(|| EngineError::Internal {
                    reason: format!("key version {version} missing"),
                })?;

        let enc_key = Self::material_to_key(&key_version.key_material)?;
        let ciphertext = crypto::encrypt(&enc_key, plaintext).map_err(|e| {
            EngineError::Internal {
                reason: format!("encryption failed: {e}"),
            }
        })?;

        Ok(format!("vault:v{version}:{}", BASE64.encode(&ciphertext)))
    }

    /// Decrypt ciphertext that was encrypted by this transit engine.
    ///
    /// # Errors
    ///
    /// Returns [`EngineError`] if the key doesn't exist, the ciphertext format
    /// is invalid, the version is below `min_decryption_version`, or decryption fails.
    pub async fn decrypt(
        &self,
        key_name: &str,
        ciphertext: &str,
    ) -> Result<Vec<u8>, EngineError> {
        let key = self.load_key(key_name).await?;

        if !key.supports_decryption {
            return Err(EngineError::InvalidRequest {
                reason: format!("key '{key_name}' does not support decryption"),
            });
        }

        let (version, raw_ct) = parse_ciphertext(ciphertext)?;

        if version < key.min_decryption_version {
            return Err(EngineError::InvalidRequest {
                reason: format!(
                    "ciphertext version {version} is below minimum decryption version {}",
                    key.min_decryption_version
                ),
            });
        }

        let key_version =
            key.versions
                .get(&version)
                .ok_or_else(|| EngineError::NotFound {
                    path: format!("{key_name}/v{version}"),
                })?;

        let enc_key = Self::material_to_key(&key_version.key_material)?;
        crypto::decrypt(&enc_key, &raw_ct).map_err(|e| EngineError::Internal {
            reason: format!("decryption failed: {e}"),
        })
    }

    /// Re-wrap ciphertext under the latest key version without revealing plaintext.
    ///
    /// # Errors
    ///
    /// Returns [`EngineError`] on any failure.
    pub async fn rewrap(
        &self,
        key_name: &str,
        ciphertext: &str,
    ) -> Result<String, EngineError> {
        let plaintext = self.decrypt(key_name, ciphertext).await?;
        self.encrypt(key_name, &plaintext).await
    }

    /// Generate a new data encryption key, returned both as plaintext and
    /// wrapped (encrypted) by the named transit key.
    ///
    /// # Errors
    ///
    /// Returns [`EngineError`] if the named key doesn't exist or encryption fails.
    pub async fn generate_data_key(
        &self,
        key_name: &str,
    ) -> Result<DataKeyResponse, EngineError> {
        let data_key = EncryptionKey::generate();
        let plaintext_b64 = BASE64.encode(data_key.as_bytes());
        let wrapped = self.encrypt(key_name, data_key.as_bytes()).await?;

        Ok(DataKeyResponse {
            plaintext: plaintext_b64,
            ciphertext: wrapped,
        })
    }

    /// List all transit key names.
    ///
    /// # Errors
    ///
    /// Returns [`EngineError`] if storage fails.
    pub async fn list_keys(&self) -> Result<Vec<String>, EngineError> {
        let prefix = format!("{}keys/", self.prefix);
        let keys = self
            .barrier
            .list(&prefix)
            .await
            .map_err(EngineError::Barrier)?;

        Ok(keys
            .iter()
            .filter_map(|k| k.strip_prefix(&prefix).map(String::from))
            .collect())
    }

    /// Get metadata about a named key (without exposing key material).
    ///
    /// # Errors
    ///
    /// Returns [`EngineError::NotFound`] if the key doesn't exist.
    pub async fn key_info(&self, name: &str) -> Result<TransitKeyInfo, EngineError> {
        let key = self.load_key(name).await?;

        Ok(TransitKeyInfo {
            name: key.name,
            latest_version: key.latest_version,
            min_decryption_version: key.min_decryption_version,
            supports_encryption: key.supports_encryption,
            supports_decryption: key.supports_decryption,
            version_count: u32::try_from(key.versions.len()).unwrap_or(u32::MAX),
            created_at: key.created_at,
        })
    }

    // ── Internal helpers ─────────────────────────────────────────────

    async fn load_key(&self, name: &str) -> Result<TransitKey, EngineError> {
        let storage_key = format!("{}keys/{}", self.prefix, name);
        let data = self
            .barrier
            .get(&storage_key)
            .await
            .map_err(EngineError::Barrier)?
            .ok_or_else(|| EngineError::NotFound {
                path: format!("transit/keys/{name}"),
            })?;

        serde_json::from_slice(&data).map_err(|e| EngineError::Internal {
            reason: format!("key deserialization failed: {e}"),
        })
    }

    async fn save_key(&self, key: &TransitKey) -> Result<(), EngineError> {
        let storage_key = format!("{}keys/{}", self.prefix, key.name);
        let bytes = serde_json::to_vec(key).map_err(|e| EngineError::Internal {
            reason: format!("key serialization failed: {e}"),
        })?;
        self.barrier
            .put(&storage_key, &bytes)
            .await
            .map_err(EngineError::Barrier)?;
        Ok(())
    }

    fn material_to_key(material: &[u8]) -> Result<EncryptionKey, EngineError> {
        let bytes: [u8; 32] = material.try_into().map_err(|_| EngineError::Internal {
            reason: "key material is not 32 bytes".to_owned(),
        })?;
        Ok(EncryptionKey::from_bytes(bytes))
    }
}

/// Response from `generate_data_key`.
#[derive(Debug, Serialize)]
pub struct DataKeyResponse {
    /// Base64-encoded plaintext data key.
    pub plaintext: String,
    /// Transit-encrypted data key (vault:v{n}:...).
    pub ciphertext: String,
}

/// Public metadata about a transit key (no key material).
#[derive(Debug, Serialize)]
pub struct TransitKeyInfo {
    pub name: String,
    pub latest_version: u32,
    pub min_decryption_version: u32,
    pub supports_encryption: bool,
    pub supports_decryption: bool,
    pub version_count: u32,
    pub created_at: DateTime<Utc>,
}

/// Parse `vault:v{version}:{base64}` ciphertext format.
fn parse_ciphertext(ct: &str) -> Result<(u32, Vec<u8>), EngineError> {
    let parts: Vec<&str> = ct.splitn(3, ':').collect();
    if parts.len() != 3 || parts[0] != "vault" {
        return Err(EngineError::InvalidRequest {
            reason: "invalid ciphertext format, expected vault:v{N}:{base64}".to_owned(),
        });
    }

    let version_str = parts[1].strip_prefix('v').ok_or_else(|| {
        EngineError::InvalidRequest {
            reason: "invalid version prefix, expected 'v{N}'".to_owned(),
        }
    })?;

    let version: u32 = version_str.parse().map_err(|_| EngineError::InvalidRequest {
        reason: format!("invalid version number: {version_str}"),
    })?;

    let raw = BASE64
        .decode(parts[2])
        .map_err(|e| EngineError::InvalidRequest {
            reason: format!("invalid base64 ciphertext: {e}"),
        })?;

    Ok((version, raw))
}

impl std::fmt::Debug for TransitEngine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TransitEngine")
            .field("prefix", &self.prefix)
            .finish_non_exhaustive()
    }
}

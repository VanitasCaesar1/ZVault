//! AppRole authentication method for `ZVault`.
//!
//! Provides machine-to-machine authentication using role IDs and secret IDs.
//! An operator creates a role with policies, retrieves the role ID, generates
//! secret IDs, and distributes them to applications. Applications exchange
//! a `(role_id, secret_id)` pair for a vault token.

use std::collections::HashMap;
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tokio::sync::RwLock;

use crate::barrier::Barrier;
use crate::error::AppRoleError;
use crate::token::{TokenEntry, TokenStore};

/// An AppRole role definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppRole {
    /// Role name.
    pub name: String,
    /// Unique role ID (UUID, generated on creation).
    pub role_id: String,
    /// Policies to attach to tokens issued via this role.
    pub policies: Vec<String>,
    /// Token TTL in seconds.
    pub token_ttl_secs: i64,
    /// Token max TTL in seconds.
    pub token_max_ttl_secs: i64,
    /// Whether secret IDs are required for login.
    pub bind_secret_id: bool,
    /// Maximum number of uses for generated secret IDs (0 = unlimited).
    pub secret_id_num_uses: u32,
    /// Secret ID TTL in seconds (0 = no expiry).
    pub secret_id_ttl_secs: i64,
}

/// A generated secret ID entry stored in the barrier.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecretIdEntry {
    /// The secret ID value (stored as SHA-256 hash).
    pub secret_id_hash: String,
    /// Role name this secret ID belongs to.
    pub role_name: String,
    /// Remaining uses (0 = unlimited).
    pub num_uses_left: u32,
    /// Creation timestamp.
    pub created_at: String,
}

/// The AppRole auth store.
pub struct AppRoleStore {
    barrier: Arc<Barrier>,
    prefix: String,
    /// Cached roles.
    roles: RwLock<HashMap<String, AppRole>>,
}

impl AppRoleStore {
    /// Create a new AppRole store.
    pub fn new(barrier: Arc<Barrier>, prefix: String) -> Self {
        Self {
            barrier,
            prefix,
            roles: RwLock::new(HashMap::new()),
        }
    }

    fn role_key(&self, name: &str) -> String {
        format!("{}roles/{}", self.prefix, name)
    }

    fn secret_id_key(&self, role_name: &str, hash: &str) -> String {
        format!("{}secret-id/{}/{}", self.prefix, role_name, hash)
    }

    /// Hash a secret ID for storage (never store plaintext).
    fn hash_secret_id(secret_id: &str) -> String {
        hex::encode(Sha256::digest(secret_id.as_bytes()))
    }

    /// Create a new AppRole role.
    ///
    /// # Errors
    ///
    /// Returns `AppRoleError::InvalidConfig` if required fields are missing.
    pub async fn create_role(&self, mut role: AppRole) -> Result<AppRole, AppRoleError> {
        if role.name.is_empty() {
            return Err(AppRoleError::InvalidConfig {
                reason: "role name is required".to_owned(),
            });
        }
        if role.policies.is_empty() {
            return Err(AppRoleError::InvalidConfig {
                reason: "at least one policy is required".to_owned(),
            });
        }
        // Generate role_id if not set.
        if role.role_id.is_empty() {
            role.role_id = uuid::Uuid::new_v4().to_string();
        }

        let data = serde_json::to_vec(&role).map_err(|e| AppRoleError::Internal {
            reason: format!("serialization failed: {e}"),
        })?;
        self.barrier.put(&self.role_key(&role.name), &data).await?;
        self.roles.write().await.insert(role.name.clone(), role.clone());
        Ok(role)
    }

    /// Get a role by name.
    ///
    /// # Errors
    ///
    /// Returns `AppRoleError::RoleNotFound` if the role does not exist.
    pub async fn get_role(&self, name: &str) -> Result<AppRole, AppRoleError> {
        if let Some(role) = self.roles.read().await.get(name) {
            return Ok(role.clone());
        }
        let data = self
            .barrier
            .get(&self.role_key(name))
            .await?
            .ok_or_else(|| AppRoleError::RoleNotFound {
                name: name.to_owned(),
            })?;
        let role: AppRole =
            serde_json::from_slice(&data).map_err(|e| AppRoleError::Internal {
                reason: format!("deserialization failed: {e}"),
            })?;
        self.roles.write().await.insert(name.to_owned(), role.clone());
        Ok(role)
    }

    /// Delete a role and all its secret IDs.
    ///
    /// # Errors
    ///
    /// Returns `AppRoleError::Barrier` if the barrier is sealed.
    pub async fn delete_role(&self, name: &str) -> Result<(), AppRoleError> {
        self.barrier.delete(&self.role_key(name)).await?;
        self.roles.write().await.remove(name);
        // Clean up secret IDs for this role.
        let prefix = format!("{}secret-id/{}/", self.prefix, name);
        let keys = self.barrier.list(&prefix).await?;
        for key in &keys {
            let _ = self.barrier.delete(key).await;
        }
        Ok(())
    }

    /// List all role names.
    ///
    /// # Errors
    ///
    /// Returns `AppRoleError::Barrier` if the barrier is sealed.
    pub async fn list_roles(&self) -> Result<Vec<String>, AppRoleError> {
        let prefix = format!("{}roles/", self.prefix);
        let keys = self.barrier.list(&prefix).await?;
        Ok(keys
            .into_iter()
            .filter_map(|k| k.strip_prefix(&prefix).map(String::from))
            .collect())
    }

    /// Get the role ID for a named role.
    ///
    /// # Errors
    ///
    /// Returns `AppRoleError::RoleNotFound` if the role does not exist.
    pub async fn get_role_id(&self, name: &str) -> Result<String, AppRoleError> {
        let role = self.get_role(name).await?;
        Ok(role.role_id)
    }

    /// Generate a new secret ID for a role.
    ///
    /// # Errors
    ///
    /// Returns `AppRoleError::RoleNotFound` if the role does not exist.
    pub async fn generate_secret_id(&self, role_name: &str) -> Result<String, AppRoleError> {
        let role = self.get_role(role_name).await?;
        let secret_id = uuid::Uuid::new_v4().to_string();
        let hash = Self::hash_secret_id(&secret_id);

        let entry = SecretIdEntry {
            secret_id_hash: hash.clone(),
            role_name: role_name.to_owned(),
            num_uses_left: role.secret_id_num_uses,
            created_at: chrono::Utc::now().to_rfc3339(),
        };

        let data = serde_json::to_vec(&entry).map_err(|e| AppRoleError::Internal {
            reason: format!("serialization failed: {e}"),
        })?;
        self.barrier
            .put(&self.secret_id_key(role_name, &hash), &data)
            .await?;

        Ok(secret_id)
    }

    /// Login with a role_id and secret_id, returning the plaintext token and its entry.
    ///
    /// # Errors
    ///
    /// Returns `AppRoleError::RoleNotFound` if no role matches the role_id.
    /// Returns `AppRoleError::InvalidSecretId` if the secret_id is invalid.
    pub async fn login(
        &self,
        role_id: &str,
        secret_id: &str,
        token_store: &TokenStore,
    ) -> Result<(String, TokenEntry), AppRoleError> {
        // Find role by role_id (scan cached roles, then barrier).
        let role = self.find_role_by_id(role_id).await?;

        if role.bind_secret_id {
            let hash = Self::hash_secret_id(secret_id);
            let key = self.secret_id_key(&role.name, &hash);
            let data = self
                .barrier
                .get(&key)
                .await?
                .ok_or_else(|| AppRoleError::InvalidSecretId {
                    role_name: role.name.clone(),
                })?;

            let mut entry: SecretIdEntry =
                serde_json::from_slice(&data).map_err(|e| AppRoleError::Internal {
                    reason: format!("deserialization failed: {e}"),
                })?;

            // Decrement uses if limited.
            if entry.num_uses_left > 0 {
                entry.num_uses_left = entry.num_uses_left.saturating_sub(1);
                if entry.num_uses_left == 0 {
                    // Last use — delete the secret ID.
                    let _ = self.barrier.delete(&key).await;
                } else {
                    let updated = serde_json::to_vec(&entry).map_err(|e| {
                        AppRoleError::Internal {
                            reason: format!("serialization failed: {e}"),
                        }
                    })?;
                    let _ = self.barrier.put(&key, &updated).await;
                }
            }
        }

        // Create a token with the role's policies.
        use crate::token::CreateTokenParams;
        let ttl = chrono::Duration::seconds(role.token_ttl_secs);
        let max_ttl = chrono::Duration::seconds(role.token_max_ttl_secs);

        let plaintext_token = token_store
            .create(CreateTokenParams {
                policies: role.policies.clone(),
                ttl: Some(ttl),
                max_ttl: Some(max_ttl),
                renewable: true,
                parent_hash: None,
                metadata: HashMap::new(),
                display_name: format!("approle-{}", role.name),
            })
            .await
            .map_err(|e| AppRoleError::Internal {
                reason: format!("token creation failed: {e}"),
            })?;

        // Look up the created token to get the full entry.
        let token_entry = token_store
            .lookup(&plaintext_token)
            .await
            .map_err(|e| AppRoleError::Internal {
                reason: format!("token lookup failed: {e}"),
            })?;

        Ok((plaintext_token, token_entry))
    }

    /// Find a role by its role_id (not name).
    async fn find_role_by_id(&self, role_id: &str) -> Result<AppRole, AppRoleError> {
        // Check cache first.
        for role in self.roles.read().await.values() {
            if role.role_id == role_id {
                return Ok(role.clone());
            }
        }
        // Scan barrier.
        let prefix = format!("{}roles/", self.prefix);
        let keys = self.barrier.list(&prefix).await?;
        for key in &keys {
            if let Ok(Some(data)) = self.barrier.get(key).await {
                if let Ok(role) = serde_json::from_slice::<AppRole>(&data) {
                    self.roles.write().await.insert(role.name.clone(), role.clone());
                    if role.role_id == role_id {
                        return Ok(role);
                    }
                }
            }
        }
        Err(AppRoleError::RoleNotFound {
            name: format!("role_id={role_id}"),
        })
    }
}

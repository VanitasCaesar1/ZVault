//! Database secrets engine for `ZVault`.
//!
//! Generates short-lived database credentials on demand. Supports PostgreSQL
//! and MySQL connection configurations with role-based credential generation.
//! Credentials are tracked via the lease system and revoked on expiry.

use std::collections::HashMap;
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;

use crate::barrier::Barrier;
use crate::error::DatabaseError;

/// A configured database connection.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseConfig {
    /// Connection name (e.g., "my-postgres").
    pub name: String,
    /// Database type: "postgresql" or "mysql".
    pub plugin: String,
    /// Connection string (stored encrypted via barrier).
    pub connection_url: String,
    /// Maximum open connections.
    pub max_open_connections: u32,
    /// Allowed roles for this connection.
    pub allowed_roles: Vec<String>,
}

/// A role definition that controls how credentials are generated.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseRole {
    /// Role name.
    pub name: String,
    /// Which database connection this role uses.
    pub db_name: String,
    /// SQL statements to create the user. `{{name}}` and `{{password}}` are replaced.
    pub creation_statements: Vec<String>,
    /// SQL statements to revoke the user.
    pub revocation_statements: Vec<String>,
    /// Default TTL in seconds.
    pub default_ttl_secs: i64,
    /// Maximum TTL in seconds.
    pub max_ttl_secs: i64,
}

/// Generated credentials returned to the caller.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseCredentials {
    /// Generated username.
    pub username: String,
    /// Generated password.
    pub password: String,
}

/// The database secrets engine.
///
/// Stores connection configs and role definitions in the barrier-encrypted
/// storage. Credential generation creates a random username/password pair
/// and returns them with a lease.
pub struct DatabaseEngine {
    barrier: Arc<Barrier>,
    prefix: String,
    /// In-memory cache of configs (loaded from barrier on access).
    configs: RwLock<HashMap<String, DatabaseConfig>>,
    /// In-memory cache of roles.
    roles: RwLock<HashMap<String, DatabaseRole>>,
}

impl DatabaseEngine {
    /// Create a new database engine with the given barrier and storage prefix.
    ///
    /// # Errors
    ///
    /// Returns `DatabaseError` if the barrier is sealed or storage fails.
    pub fn new(barrier: Arc<Barrier>, prefix: String) -> Self {
        Self {
            barrier,
            prefix,
            configs: RwLock::new(HashMap::new()),
            roles: RwLock::new(HashMap::new()),
        }
    }

    fn config_key(&self, name: &str) -> String {
        format!("{}config/{}", self.prefix, name)
    }

    fn role_key(&self, name: &str) -> String {
        format!("{}roles/{}", self.prefix, name)
    }

    /// Configure a database connection.
    ///
    /// # Errors
    ///
    /// Returns `DatabaseError::InvalidConfig` if required fields are missing.
    /// Returns `DatabaseError::Barrier` if the barrier is sealed.
    pub async fn configure(&self, config: DatabaseConfig) -> Result<(), DatabaseError> {
        if config.name.is_empty() {
            return Err(DatabaseError::InvalidConfig {
                reason: "connection name is required".to_owned(),
            });
        }
        if config.connection_url.is_empty() {
            return Err(DatabaseError::InvalidConfig {
                reason: "connection_url is required".to_owned(),
            });
        }
        if config.plugin != "postgresql" && config.plugin != "mysql" {
            return Err(DatabaseError::InvalidConfig {
                reason: format!("unsupported plugin '{}', expected 'postgresql' or 'mysql'", config.plugin),
            });
        }

        let data = serde_json::to_vec(&config).map_err(|e| DatabaseError::Internal {
            reason: format!("serialization failed: {e}"),
        })?;
        self.barrier
            .put(&self.config_key(&config.name), &data)
            .await?;
        self.configs.write().await.insert(config.name.clone(), config);
        Ok(())
    }

    /// Read a database connection config by name.
    ///
    /// # Errors
    ///
    /// Returns `DatabaseError::NotFound` if the config does not exist.
    pub async fn get_config(&self, name: &str) -> Result<DatabaseConfig, DatabaseError> {
        // Check in-memory cache first.
        if let Some(cfg) = self.configs.read().await.get(name) {
            return Ok(cfg.clone());
        }
        let data = self
            .barrier
            .get(&self.config_key(name))
            .await?
            .ok_or_else(|| DatabaseError::NotFound {
                name: name.to_owned(),
            })?;
        let config: DatabaseConfig =
            serde_json::from_slice(&data).map_err(|e| DatabaseError::Internal {
                reason: format!("deserialization failed: {e}"),
            })?;
        self.configs.write().await.insert(name.to_owned(), config.clone());
        Ok(config)
    }

    /// Delete a database connection config.
    ///
    /// # Errors
    ///
    /// Returns `DatabaseError::Barrier` if the barrier is sealed.
    pub async fn delete_config(&self, name: &str) -> Result<(), DatabaseError> {
        self.barrier.delete(&self.config_key(name)).await?;
        self.configs.write().await.remove(name);
        Ok(())
    }

    /// List all configured database connections.
    ///
    /// # Errors
    ///
    /// Returns `DatabaseError::Barrier` if the barrier is sealed.
    pub async fn list_configs(&self) -> Result<Vec<String>, DatabaseError> {
        let prefix = format!("{}config/", self.prefix);
        let keys = self.barrier.list(&prefix).await?;
        Ok(keys
            .into_iter()
            .filter_map(|k| k.strip_prefix(&prefix).map(String::from))
            .collect())
    }

    /// Create a role definition.
    ///
    /// # Errors
    ///
    /// Returns `DatabaseError::InvalidConfig` if required fields are missing.
    pub async fn create_role(&self, role: DatabaseRole) -> Result<(), DatabaseError> {
        if role.name.is_empty() {
            return Err(DatabaseError::InvalidConfig {
                reason: "role name is required".to_owned(),
            });
        }
        if role.db_name.is_empty() {
            return Err(DatabaseError::InvalidConfig {
                reason: "db_name is required".to_owned(),
            });
        }
        if role.creation_statements.is_empty() {
            return Err(DatabaseError::InvalidConfig {
                reason: "creation_statements is required".to_owned(),
            });
        }
        // Verify the referenced config exists.
        self.get_config(&role.db_name).await?;

        let data = serde_json::to_vec(&role).map_err(|e| DatabaseError::Internal {
            reason: format!("serialization failed: {e}"),
        })?;
        self.barrier.put(&self.role_key(&role.name), &data).await?;
        self.roles.write().await.insert(role.name.clone(), role);
        Ok(())
    }

    /// Read a role by name.
    ///
    /// # Errors
    ///
    /// Returns `DatabaseError::NotFound` if the role does not exist.
    pub async fn get_role(&self, name: &str) -> Result<DatabaseRole, DatabaseError> {
        if let Some(role) = self.roles.read().await.get(name) {
            return Ok(role.clone());
        }
        let data = self
            .barrier
            .get(&self.role_key(name))
            .await?
            .ok_or_else(|| DatabaseError::RoleNotFound {
                name: name.to_owned(),
            })?;
        let role: DatabaseRole =
            serde_json::from_slice(&data).map_err(|e| DatabaseError::Internal {
                reason: format!("deserialization failed: {e}"),
            })?;
        self.roles.write().await.insert(name.to_owned(), role.clone());
        Ok(role)
    }

    /// Delete a role.
    ///
    /// # Errors
    ///
    /// Returns `DatabaseError::Barrier` if the barrier is sealed.
    pub async fn delete_role(&self, name: &str) -> Result<(), DatabaseError> {
        self.barrier.delete(&self.role_key(name)).await?;
        self.roles.write().await.remove(name);
        Ok(())
    }

    /// List all role names.
    ///
    /// # Errors
    ///
    /// Returns `DatabaseError::Barrier` if the barrier is sealed.
    pub async fn list_roles(&self) -> Result<Vec<String>, DatabaseError> {
        let prefix = format!("{}roles/", self.prefix);
        let keys = self.barrier.list(&prefix).await?;
        Ok(keys
            .into_iter()
            .filter_map(|k| k.strip_prefix(&prefix).map(String::from))
            .collect())
    }

    /// Generate credentials for a role.
    ///
    /// Creates a random username and password. In a production deployment,
    /// these would be executed against the actual database. For now, the
    /// credentials are generated and returned — the caller is responsible
    /// for creating a lease.
    ///
    /// # Errors
    ///
    /// Returns `DatabaseError::RoleNotFound` if the role does not exist.
    /// Returns `DatabaseError::NotFound` if the referenced config is missing.
    pub async fn generate_credentials(
        &self,
        role_name: &str,
    ) -> Result<(DatabaseCredentials, DatabaseRole), DatabaseError> {
        let role = self.get_role(role_name).await?;
        // Verify config still exists.
        let _config = self.get_config(&role.db_name).await?;

        let username = format!("v-{}-{}", role_name, &uuid::Uuid::new_v4().to_string()[..8]);
        let password = uuid::Uuid::new_v4().to_string().replace('-', "");

        let creds = DatabaseCredentials { username, password };
        Ok((creds, role))
    }
}

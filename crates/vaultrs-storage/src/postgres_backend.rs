//! PostgreSQL storage backend.
//!
//! Stores all key-value data in a single `kv_store` table. Keys are UTF-8
//! strings, values are opaque encrypted bytes. The barrier encrypts all data
//! before it reaches this layer.
//!
//! Feature-gated behind `postgres-backend`. Uses `sqlx` with the Tokio
//! runtime for fully async operations — no `spawn_blocking` needed.

use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;

use crate::{StorageBackend, StorageError};

/// A storage backend backed by PostgreSQL.
///
/// Thread-safe via `PgPool` (connection pool). All operations are fully async.
///
/// # Examples
///
/// ```no_run
/// # use vaultrs_storage::PostgresBackend;
/// # #[tokio::main]
/// # async fn main() {
/// let backend = PostgresBackend::connect("postgres://localhost/zvault").await.unwrap();
/// # }
/// ```
#[derive(Clone)]
pub struct PostgresBackend {
    pool: PgPool,
}

impl std::fmt::Debug for PostgresBackend {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PostgresBackend")
            .field("pool", &"[PgPool]")
            .finish_non_exhaustive()
    }
}

impl PostgresBackend {
    /// Connect to PostgreSQL and run the initial migration.
    ///
    /// Creates the `kv_store` table if it does not exist.
    ///
    /// # Errors
    ///
    /// Returns [`StorageError::Open`] if the connection or migration fails.
    pub async fn connect(database_url: &str) -> Result<Self, StorageError> {
        let pool = PgPoolOptions::new()
            .max_connections(10)
            .connect(database_url)
            .await
            .map_err(|e| StorageError::Open {
                path: database_url.to_owned(),
                reason: e.to_string(),
            })?;

        // Auto-create the table if it doesn't exist.
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS kv_store (\
                key   TEXT  PRIMARY KEY, \
                value BYTEA NOT NULL\
            )"
        )
        .execute(&pool)
        .await
        .map_err(|e| StorageError::Open {
            path: database_url.to_owned(),
            reason: format!("migration failed: {e}"),
        })?;

        // Create prefix index for efficient list operations.
        sqlx::query(
            "CREATE INDEX IF NOT EXISTS idx_kv_store_key_prefix \
             ON kv_store (key text_pattern_ops)"
        )
        .execute(&pool)
        .await
        .map_err(|e| StorageError::Open {
            path: database_url.to_owned(),
            reason: format!("index creation failed: {e}"),
        })?;

        Ok(Self { pool })
    }

    /// Return a reference to the underlying connection pool.
    #[must_use]
    pub fn pool(&self) -> &PgPool {
        &self.pool
    }
}

#[async_trait::async_trait]
impl StorageBackend for PostgresBackend {
    async fn get(&self, key: &str) -> Result<Option<Vec<u8>>, StorageError> {
        let row: Option<(Vec<u8>,)> = sqlx::query_as(
            "SELECT value FROM kv_store WHERE key = $1"
        )
        .bind(key)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| StorageError::Read {
            key: key.to_owned(),
            reason: e.to_string(),
        })?;

        Ok(row.map(|(v,)| v))
    }

    async fn put(&self, key: &str, value: &[u8]) -> Result<(), StorageError> {
        sqlx::query(
            "INSERT INTO kv_store (key, value) VALUES ($1, $2) \
             ON CONFLICT (key) DO UPDATE SET value = EXCLUDED.value"
        )
        .bind(key)
        .bind(value)
        .execute(&self.pool)
        .await
        .map_err(|e| StorageError::Write {
            key: key.to_owned(),
            reason: e.to_string(),
        })?;

        Ok(())
    }

    async fn delete(&self, key: &str) -> Result<(), StorageError> {
        sqlx::query("DELETE FROM kv_store WHERE key = $1")
            .bind(key)
            .execute(&self.pool)
            .await
            .map_err(|e| StorageError::Delete {
                key: key.to_owned(),
                reason: e.to_string(),
            })?;

        Ok(())
    }

    async fn list(&self, prefix: &str) -> Result<Vec<String>, StorageError> {
        let rows: Vec<(String,)> = sqlx::query_as(
            "SELECT key FROM kv_store WHERE key LIKE $1 ORDER BY key"
        )
        .bind(format!("{prefix}%"))
        .fetch_all(&self.pool)
        .await
        .map_err(|e| StorageError::List {
            prefix: prefix.to_owned(),
            reason: e.to_string(),
        })?;

        Ok(rows.into_iter().map(|(k,)| k).collect())
    }

    async fn exists(&self, key: &str) -> Result<bool, StorageError> {
        let row: Option<(bool,)> = sqlx::query_as(
            "SELECT EXISTS(SELECT 1 FROM kv_store WHERE key = $1)"
        )
        .bind(key)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| StorageError::Read {
            key: key.to_owned(),
            reason: e.to_string(),
        })?;

        Ok(row.map(|(e,)| e).unwrap_or(false))
    }
}

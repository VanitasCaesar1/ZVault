//! Storage backend abstraction for `ZVault`.
//!
//! This crate defines the [`StorageBackend`] trait — a pure key-value storage
//! interface that knows nothing about secrets, encryption, or engines. The
//! encryption barrier in `zvault-core` wraps a storage backend to ensure all
//! data is encrypted before it reaches this layer.
//!
//! Three implementations are provided:
//!
//! - [`RocksDbBackend`] — production default, backed by `RocksDB` (feature `rocksdb-backend`)
//! - [`RedbBackend`] — pure-Rust alternative, backed by redb (feature `redb-backend`)
//! - [`MemoryBackend`] — in-memory, for testing only

mod error;
mod memory;
#[cfg(feature = "postgres-backend")]
mod postgres_backend;
#[cfg(feature = "redb-backend")]
mod redb_backend;
#[cfg(feature = "rocksdb-backend")]
mod rocksdb_backend;

pub use error::StorageError;
pub use memory::MemoryBackend;
#[cfg(feature = "postgres-backend")]
pub use postgres_backend::PostgresBackend;
#[cfg(feature = "redb-backend")]
pub use redb_backend::RedbBackend;
#[cfg(feature = "rocksdb-backend")]
pub use rocksdb_backend::RocksDbBackend;

/// A pluggable key-value storage backend.
///
/// Keys are UTF-8 strings using `/` as a separator (e.g. `sys/config`,
/// `kv/default/data/myapp/password`). Values are opaque byte arrays —
/// always encrypted by the barrier before reaching storage.
///
/// Implementations must be safe to share across async tasks (`Send + Sync`).
#[async_trait::async_trait]
pub trait StorageBackend: Send + Sync + 'static {
    /// Retrieve a value by key.
    ///
    /// Returns `Ok(None)` if the key does not exist.
    ///
    /// # Errors
    ///
    /// Returns [`StorageError::Read`] if the underlying backend fails.
    async fn get(&self, key: &str) -> Result<Option<Vec<u8>>, StorageError>;

    /// Store a key-value pair, overwriting any existing value.
    ///
    /// # Errors
    ///
    /// Returns [`StorageError::Write`] if the underlying backend fails.
    async fn put(&self, key: &str, value: &[u8]) -> Result<(), StorageError>;

    /// Delete a key. This is idempotent — deleting a non-existent key is not
    /// an error.
    ///
    /// # Errors
    ///
    /// Returns [`StorageError::Delete`] if the underlying backend fails.
    async fn delete(&self, key: &str) -> Result<(), StorageError>;

    /// List all keys that start with the given prefix.
    ///
    /// Returns keys only, not values. This is a metadata operation used for
    /// directory-style listing.
    ///
    /// # Errors
    ///
    /// Returns [`StorageError::List`] if the underlying backend fails.
    async fn list(&self, prefix: &str) -> Result<Vec<String>, StorageError>;

    /// Check whether a key exists in storage.
    ///
    /// The default implementation calls [`get`](StorageBackend::get) and checks
    /// for `Some`. Backends may override this with a more efficient check.
    ///
    /// # Errors
    ///
    /// Returns [`StorageError::Read`] if the underlying backend fails.
    async fn exists(&self, key: &str) -> Result<bool, StorageError> {
        Ok(self.get(key).await?.is_some())
    }
}

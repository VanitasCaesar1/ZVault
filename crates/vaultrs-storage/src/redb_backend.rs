//! Pure-Rust redb storage backend.
//!
//! An alternative to `RocksDB` for environments where a pure-Rust build is
//! required (no C++ FFI). Feature-gated behind `redb-backend`.
//!
//! redb uses a B-tree internally, giving consistent read/write performance
//! without LSM compaction pauses. All operations are transactional.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use redb::{Database, TableDefinition};

use crate::{StorageBackend, StorageError};

/// The single table used for all key-value data.
/// Key namespacing is handled at the barrier/engine level.
const DATA_TABLE: TableDefinition<&str, &[u8]> = TableDefinition::new("data");

/// A storage backend backed by redb (pure Rust, B-tree based).
///
/// Thread-safe via `Arc<Database>`. Blocking redb calls are offloaded to the
/// Tokio blocking thread pool.
///
/// # Examples
///
/// ```no_run
/// # use vaultrs_storage::RedbBackend;
/// let backend = RedbBackend::open("/var/lib/vaultrs/data.redb").unwrap();
/// ```
#[derive(Clone)]
pub struct RedbBackend {
    db: Arc<Database>,
    path: PathBuf,
}

impl std::fmt::Debug for RedbBackend {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RedbBackend")
            .field("path", &self.path)
            .finish_non_exhaustive()
    }
}

impl RedbBackend {
    /// Open or create a redb database at the given path.
    ///
    /// # Errors
    ///
    /// Returns [`StorageError::Open`] if redb fails to open or create the
    /// database file.
    pub fn open(path: impl AsRef<Path>) -> Result<Self, StorageError> {
        let path = path.as_ref();
        let db = Database::create(path).map_err(|e| StorageError::Open {
            path: path.display().to_string(),
            reason: e.to_string(),
        })?;

        // Ensure the data table exists by opening a write transaction.
        let txn = db.begin_write().map_err(|e| StorageError::Transaction {
            reason: e.to_string(),
        })?;
        {
            // Opening the table in a write txn creates it if missing.
            let _table = txn
                .open_table(DATA_TABLE)
                .map_err(|e| StorageError::MissingTable {
                    name: format!("data: {e}"),
                })?;
        }
        txn.commit().map_err(|e| StorageError::Transaction {
            reason: e.to_string(),
        })?;

        Ok(Self {
            db: Arc::new(db),
            path: path.to_path_buf(),
        })
    }

    /// Return the filesystem path of this database.
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }
}

#[async_trait::async_trait]
impl StorageBackend for RedbBackend {
    async fn get(&self, key: &str) -> Result<Option<Vec<u8>>, StorageError> {
        let db = Arc::clone(&self.db);
        let key = key.to_owned();
        tokio::task::spawn_blocking(move || {
            let txn = db.begin_read().map_err(|e| StorageError::Transaction {
                reason: e.to_string(),
            })?;
            let table = txn
                .open_table(DATA_TABLE)
                .map_err(|e| StorageError::MissingTable {
                    name: format!("data: {e}"),
                })?;
            let result = table
                .get(key.as_str())
                .map_err(|e| StorageError::Read {
                    key: key.clone(),
                    reason: e.to_string(),
                })?
                .map(|v| v.value().to_vec());
            Ok(result)
        })
        .await
        .map_err(|e| StorageError::Read {
            key: String::new(),
            reason: format!("blocking task panicked: {e}"),
        })?
    }

    async fn put(&self, key: &str, value: &[u8]) -> Result<(), StorageError> {
        let db = Arc::clone(&self.db);
        let key = key.to_owned();
        let value = value.to_vec();
        tokio::task::spawn_blocking(move || {
            let txn = db.begin_write().map_err(|e| StorageError::Transaction {
                reason: e.to_string(),
            })?;
            {
                let mut table =
                    txn.open_table(DATA_TABLE)
                        .map_err(|e| StorageError::MissingTable {
                            name: format!("data: {e}"),
                        })?;
                table
                    .insert(key.as_str(), value.as_slice())
                    .map_err(|e| StorageError::Write {
                        key: key.clone(),
                        reason: e.to_string(),
                    })?;
            }
            txn.commit().map_err(|e| StorageError::Transaction {
                reason: e.to_string(),
            })?;
            Ok(())
        })
        .await
        .map_err(|e| StorageError::Write {
            key: String::new(),
            reason: format!("blocking task panicked: {e}"),
        })?
    }

    async fn delete(&self, key: &str) -> Result<(), StorageError> {
        let db = Arc::clone(&self.db);
        let key = key.to_owned();
        tokio::task::spawn_blocking(move || {
            let txn = db.begin_write().map_err(|e| StorageError::Transaction {
                reason: e.to_string(),
            })?;
            {
                let mut table =
                    txn.open_table(DATA_TABLE)
                        .map_err(|e| StorageError::MissingTable {
                            name: format!("data: {e}"),
                        })?;
                // remove() is idempotent — returns Ok(None) if key doesn't exist.
                table
                    .remove(key.as_str())
                    .map_err(|e| StorageError::Delete {
                        key: key.clone(),
                        reason: e.to_string(),
                    })?;
            }
            txn.commit().map_err(|e| StorageError::Transaction {
                reason: e.to_string(),
            })?;
            Ok(())
        })
        .await
        .map_err(|e| StorageError::Delete {
            key: String::new(),
            reason: format!("blocking task panicked: {e}"),
        })?
    }

    async fn list(&self, prefix: &str) -> Result<Vec<String>, StorageError> {
        let db = Arc::clone(&self.db);
        let prefix = prefix.to_owned();
        tokio::task::spawn_blocking(move || {
            let txn = db.begin_read().map_err(|e| StorageError::Transaction {
                reason: e.to_string(),
            })?;
            let table = txn
                .open_table(DATA_TABLE)
                .map_err(|e| StorageError::MissingTable {
                    name: format!("data: {e}"),
                })?;

            let mut keys = Vec::new();
            let range = table
                .range(prefix.as_str()..)
                .map_err(|e| StorageError::List {
                    prefix: prefix.clone(),
                    reason: e.to_string(),
                })?;
            for item in range {
                let (k, _) = item.map_err(|e| StorageError::List {
                    prefix: prefix.clone(),
                    reason: e.to_string(),
                })?;
                let key_str = k.value();
                if !key_str.starts_with(&prefix) {
                    break;
                }
                keys.push(key_str.to_owned());
            }
            Ok(keys)
        })
        .await
        .map_err(|e| StorageError::List {
            prefix: String::new(),
            reason: format!("blocking task panicked: {e}"),
        })?
    }
}

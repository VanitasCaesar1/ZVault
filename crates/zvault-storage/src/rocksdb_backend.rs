//! `RocksDB` storage backend — the production default.
//!
//! Wraps the `rocksdb` crate behind the [`StorageBackend`] trait. All
//! operations are dispatched to a blocking thread via
//! [`tokio::task::spawn_blocking`] since `RocksDB` is a synchronous C++ library.
//!
//! Key namespacing and encryption happen above this layer (in the barrier).
//! This backend treats keys as opaque UTF-8 strings and values as opaque bytes.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use rocksdb::{DBWithThreadMode, MultiThreaded, Options};

use crate::{StorageBackend, StorageError};

type Db = DBWithThreadMode<MultiThreaded>;

/// A storage backend backed by `RocksDB`.
///
/// Thread-safe (`Arc<DB>` internally) and safe to share across async tasks.
/// All blocking `RocksDB` calls are offloaded to the Tokio blocking thread pool.
///
/// # Examples
///
/// ```no_run
/// # use zvault_storage::RocksDbBackend;
/// let backend = RocksDbBackend::open("/var/lib/zvault/data").unwrap();
/// ```
#[derive(Clone)]
pub struct RocksDbBackend {
    db: Arc<Db>,
    path: PathBuf,
}

impl std::fmt::Debug for RocksDbBackend {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RocksDbBackend")
            .field("path", &self.path)
            .finish_non_exhaustive()
    }
}

impl RocksDbBackend {
    /// Open a `RocksDB` database at the given path.
    ///
    /// Creates the database directory if it does not exist.
    ///
    /// # Errors
    ///
    /// Returns [`StorageError::Open`] if `RocksDB` fails to open or create the
    /// database at the specified path.
    pub fn open(path: impl AsRef<Path>) -> Result<Self, StorageError> {
        let path = path.as_ref();
        let mut opts = Options::default();
        opts.create_if_missing(true);
        opts.set_prefix_extractor(rocksdb::SliceTransform::create_fixed_prefix(0));

        let db = Db::open(&opts, path).map_err(|e| StorageError::Open {
            path: path.display().to_string(),
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
impl StorageBackend for RocksDbBackend {
    async fn get(&self, key: &str) -> Result<Option<Vec<u8>>, StorageError> {
        let db = Arc::clone(&self.db);
        let key = key.to_owned();
        tokio::task::spawn_blocking(move || {
            db.get(key.as_bytes()).map_err(|e| StorageError::Read {
                key,
                reason: e.to_string(),
            })
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
            db.put(key.as_bytes(), &value)
                .map_err(|e| StorageError::Write {
                    key,
                    reason: e.to_string(),
                })
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
            db.delete(key.as_bytes()).map_err(|e| StorageError::Delete {
                key,
                reason: e.to_string(),
            })
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
            let iter = db.iterator(rocksdb::IteratorMode::From(
                prefix.as_bytes(),
                rocksdb::Direction::Forward,
            ));

            let mut keys = Vec::new();
            for item in iter {
                let (k, _) = item.map_err(|e| StorageError::List {
                    prefix: prefix.clone(),
                    reason: e.to_string(),
                })?;
                let key_str =
                    String::from_utf8(k.to_vec()).map_err(|e| StorageError::InvalidKey {
                        reason: e.to_string(),
                    })?;
                if !key_str.starts_with(&prefix) {
                    break;
                }
                keys.push(key_str);
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

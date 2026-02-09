//! In-memory storage backend for testing.
//!
//! This backend stores all data in a `BTreeMap` behind a `RwLock`. It is not
//! persistent — all data is lost when the process exits. Use this for unit
//! tests and integration tests where you need a real storage backend without
//! touching disk.

use std::collections::BTreeMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::{StorageBackend, StorageError};

/// An in-memory storage backend backed by a `BTreeMap`.
///
/// Thread-safe and async-compatible. Data is sorted by key, which makes
/// prefix listing efficient via `BTreeMap::range`.
///
/// # Examples
///
/// ```
/// # use vaultrs_storage::{MemoryBackend, StorageBackend};
/// # #[tokio::main]
/// # async fn main() {
/// let backend = MemoryBackend::new();
/// backend.put("sys/config", b"data").await.unwrap();
/// let val = backend.get("sys/config").await.unwrap();
/// assert_eq!(val, Some(b"data".to_vec()));
/// # }
/// ```
#[derive(Debug, Clone)]
pub struct MemoryBackend {
    data: Arc<RwLock<BTreeMap<String, Vec<u8>>>>,
}

impl MemoryBackend {
    /// Create a new empty in-memory backend.
    #[must_use]
    pub fn new() -> Self {
        Self {
            data: Arc::new(RwLock::new(BTreeMap::new())),
        }
    }
}

impl Default for MemoryBackend {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl StorageBackend for MemoryBackend {
    async fn get(&self, key: &str) -> Result<Option<Vec<u8>>, StorageError> {
        let data = self.data.read().await;
        Ok(data.get(key).cloned())
    }

    async fn put(&self, key: &str, value: &[u8]) -> Result<(), StorageError> {
        let mut data = self.data.write().await;
        data.insert(key.to_owned(), value.to_vec());
        Ok(())
    }

    async fn delete(&self, key: &str) -> Result<(), StorageError> {
        let mut data = self.data.write().await;
        data.remove(key);
        Ok(())
    }

    async fn list(&self, prefix: &str) -> Result<Vec<String>, StorageError> {
        let data = self.data.read().await;
        let keys = data
            .range(prefix.to_owned()..)
            .take_while(|(k, _)| k.starts_with(prefix))
            .map(|(k, _)| k.clone())
            .collect();
        Ok(keys)
    }

    async fn exists(&self, key: &str) -> Result<bool, StorageError> {
        let data = self.data.read().await;
        Ok(data.contains_key(key))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn get_nonexistent_returns_none() {
        let backend = MemoryBackend::new();
        let result = backend.get("does/not/exist").await.unwrap();
        assert_eq!(result, None);
    }

    #[tokio::test]
    async fn put_and_get_roundtrip() {
        let backend = MemoryBackend::new();
        backend.put("sys/config", b"hello").await.unwrap();
        let val = backend.get("sys/config").await.unwrap();
        assert_eq!(val, Some(b"hello".to_vec()));
    }

    #[tokio::test]
    async fn put_overwrites_existing() {
        let backend = MemoryBackend::new();
        backend.put("key", b"v1").await.unwrap();
        backend.put("key", b"v2").await.unwrap();
        let val = backend.get("key").await.unwrap();
        assert_eq!(val, Some(b"v2".to_vec()));
    }

    #[tokio::test]
    async fn delete_existing_key() {
        let backend = MemoryBackend::new();
        backend.put("key", b"val").await.unwrap();
        backend.delete("key").await.unwrap();
        let val = backend.get("key").await.unwrap();
        assert_eq!(val, None);
    }

    #[tokio::test]
    async fn delete_nonexistent_is_noop() {
        let backend = MemoryBackend::new();
        // Should not error.
        backend.delete("nope").await.unwrap();
    }

    #[tokio::test]
    async fn list_with_prefix() {
        let backend = MemoryBackend::new();
        backend.put("kv/data/a", b"1").await.unwrap();
        backend.put("kv/data/b", b"2").await.unwrap();
        backend.put("kv/metadata/a", b"3").await.unwrap();
        backend.put("sys/config", b"4").await.unwrap();

        let keys = backend.list("kv/data/").await.unwrap();
        assert_eq!(keys, vec!["kv/data/a", "kv/data/b"]);
    }

    #[tokio::test]
    async fn list_empty_prefix_returns_all() {
        let backend = MemoryBackend::new();
        backend.put("a", b"1").await.unwrap();
        backend.put("b", b"2").await.unwrap();
        let keys = backend.list("").await.unwrap();
        assert_eq!(keys, vec!["a", "b"]);
    }

    #[tokio::test]
    async fn list_no_matches_returns_empty() {
        let backend = MemoryBackend::new();
        backend.put("sys/config", b"1").await.unwrap();
        let keys = backend.list("kv/").await.unwrap();
        assert!(keys.is_empty());
    }

    #[tokio::test]
    async fn exists_returns_true_for_existing() {
        let backend = MemoryBackend::new();
        backend.put("key", b"val").await.unwrap();
        assert!(backend.exists("key").await.unwrap());
    }

    #[tokio::test]
    async fn exists_returns_false_for_missing() {
        let backend = MemoryBackend::new();
        assert!(!backend.exists("nope").await.unwrap());
    }

    #[tokio::test]
    async fn clone_shares_state() {
        let backend = MemoryBackend::new();
        let clone = backend.clone();
        backend.put("key", b"val").await.unwrap();
        let val = clone.get("key").await.unwrap();
        assert_eq!(val, Some(b"val".to_vec()));
    }
}

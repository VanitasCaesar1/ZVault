//! Encryption barrier for `ZVault`.
//!
//! The barrier is the most critical architectural invariant: every byte that
//! touches the storage backend passes through the barrier's encrypt/decrypt.
//! The storage layer only ever sees ciphertext.
//!
//! When the vault is sealed, the barrier rejects all operations with
//! [`BarrierError::Sealed`].
//!
//! # Security model
//!
//! - The root key lives only in process memory, never on disk in plaintext.
//! - All values are encrypted with AES-256-GCM (fresh nonce per write).
//! - Keys (storage paths) are stored in plaintext to support prefix listing.
//! - Sealing zeroizes the root key from memory immediately.

use std::sync::Arc;

use tokio::sync::RwLock;
use zvault_storage::StorageBackend;

use crate::crypto::{self, EncryptionKey};
use crate::error::BarrierError;

/// The encryption barrier wrapping a storage backend.
///
/// All reads decrypt, all writes encrypt. When sealed, all operations return
/// [`BarrierError::Sealed`].
pub struct Barrier {
    storage: Arc<dyn StorageBackend>,
    key: RwLock<Option<EncryptionKey>>,
}

impl Barrier {
    /// Create a new sealed barrier wrapping the given storage backend.
    #[must_use]
    pub fn new(storage: Arc<dyn StorageBackend>) -> Self {
        Self {
            storage,
            key: RwLock::new(None),
        }
    }

    /// Unseal the barrier by providing the root encryption key.
    ///
    /// After this call, all read/write operations will succeed (assuming the
    /// underlying storage is healthy).
    pub async fn unseal(&self, key: EncryptionKey) {
        let mut guard = self.key.write().await;
        *guard = Some(key);
    }

    /// Seal the barrier, zeroizing the root key from memory.
    ///
    /// After this call, all operations return [`BarrierError::Sealed`].
    /// The key is zeroized via its `ZeroizeOnDrop` implementation when the
    /// old `Option<EncryptionKey>` is replaced with `None`.
    pub async fn seal(&self) {
        let mut guard = self.key.write().await;
        *guard = None;
    }

    /// Check whether the barrier is currently unsealed.
    pub async fn is_unsealed(&self) -> bool {
        self.key.read().await.is_some()
    }

    /// Read a value from storage, decrypting it through the barrier.
    ///
    /// Returns `Ok(None)` if the key does not exist in storage.
    ///
    /// # Errors
    ///
    /// - [`BarrierError::Sealed`] if the vault is sealed.
    /// - [`BarrierError::Crypto`] if decryption fails.
    /// - [`BarrierError::Storage`] if the storage backend fails.
    pub async fn get(&self, key: &str) -> Result<Option<Vec<u8>>, BarrierError> {
        let root_key = self.root_key().await?;

        let encrypted = self.storage.get(key).await?;
        match encrypted {
            None => Ok(None),
            Some(ciphertext) => {
                let plaintext = crypto::decrypt(&root_key, &ciphertext)?;
                Ok(Some(plaintext))
            }
        }
    }

    /// Write a value to storage, encrypting it through the barrier.
    ///
    /// # Errors
    ///
    /// - [`BarrierError::Sealed`] if the vault is sealed.
    /// - [`BarrierError::Crypto`] if encryption fails.
    /// - [`BarrierError::Storage`] if the storage backend fails.
    pub async fn put(&self, key: &str, value: &[u8]) -> Result<(), BarrierError> {
        let root_key = self.root_key().await?;

        let ciphertext = crypto::encrypt(&root_key, value)?;
        self.storage.put(key, &ciphertext).await?;
        Ok(())
    }

    /// Delete a key from storage.
    ///
    /// # Errors
    ///
    /// - [`BarrierError::Sealed`] if the vault is sealed.
    /// - [`BarrierError::Storage`] if the storage backend fails.
    pub async fn delete(&self, key: &str) -> Result<(), BarrierError> {
        let _root_key = self.root_key().await?;
        self.storage.delete(key).await?;
        Ok(())
    }

    /// List keys with the given prefix.
    ///
    /// Keys (paths) are not encrypted — only values are. This allows prefix
    /// listing to work without a separate index.
    ///
    /// # Errors
    ///
    /// - [`BarrierError::Sealed`] if the vault is sealed.
    /// - [`BarrierError::Storage`] if the storage backend fails.
    pub async fn list(&self, prefix: &str) -> Result<Vec<String>, BarrierError> {
        let _root_key = self.root_key().await?;
        let keys = self.storage.list(prefix).await?;
        Ok(keys)
    }

    /// Check whether a key exists in storage.
    ///
    /// # Errors
    ///
    /// - [`BarrierError::Sealed`] if the vault is sealed.
    /// - [`BarrierError::Storage`] if the storage backend fails.
    pub async fn exists(&self, key: &str) -> Result<bool, BarrierError> {
        let _root_key = self.root_key().await?;
        let exists = self.storage.exists(key).await?;
        Ok(exists)
    }

    /// Write raw bytes to storage WITHOUT encryption.
    ///
    /// Used for storing the encrypted root key during initialization and
    /// for backup/restore operations (which transfer ciphertext as-is).
    ///
    /// # Security
    ///
    /// Do NOT use this for normal secret storage. All application data must
    /// go through [`put`](Barrier::put) which encrypts before writing.
    ///
    /// # Errors
    ///
    /// Returns [`BarrierError::Storage`] if the storage backend fails.
    pub async fn put_raw(&self, key: &str, value: &[u8]) -> Result<(), BarrierError> {
        self.storage.put(key, value).await?;
        Ok(())
    }

    /// Read raw bytes from storage WITHOUT decryption.
    ///
    /// Used for reading the encrypted root key during unseal and for
    /// backup/restore operations (which transfer ciphertext as-is).
    ///
    /// # Security
    ///
    /// Do NOT use this for normal secret reads. All application data must
    /// go through [`get`](Barrier::get) which decrypts after reading.
    ///
    /// # Errors
    ///
    /// Returns [`BarrierError::Storage`] if the storage backend fails.
    pub async fn get_raw(&self, key: &str) -> Result<Option<Vec<u8>>, BarrierError> {
        let val = self.storage.get(key).await?;
        Ok(val)
    }

    /// Clone the current root key (if unsealed).
    ///
    /// # Errors
    ///
    /// Returns [`BarrierError::Sealed`] if the vault is sealed.
    async fn root_key(&self) -> Result<EncryptionKey, BarrierError> {
        let guard = self.key.read().await;
        guard.clone().ok_or(BarrierError::Sealed)
    }
}

impl fmt::Debug for Barrier {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Barrier")
            .field("sealed", &"<check with is_unsealed()>")
            .finish_non_exhaustive()
    }
}

use std::fmt;

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use zvault_storage::MemoryBackend;

    fn make_barrier() -> Barrier {
        let storage = Arc::new(MemoryBackend::new());
        Barrier::new(storage)
    }

    #[tokio::test]
    async fn sealed_barrier_rejects_get() {
        let barrier = make_barrier();
        let result = barrier.get("key").await;
        assert!(matches!(result, Err(BarrierError::Sealed)));
    }

    #[tokio::test]
    async fn sealed_barrier_rejects_put() {
        let barrier = make_barrier();
        let result = barrier.put("key", b"value").await;
        assert!(matches!(result, Err(BarrierError::Sealed)));
    }

    #[tokio::test]
    async fn sealed_barrier_rejects_delete() {
        let barrier = make_barrier();
        let result = barrier.delete("key").await;
        assert!(matches!(result, Err(BarrierError::Sealed)));
    }

    #[tokio::test]
    async fn sealed_barrier_rejects_list() {
        let barrier = make_barrier();
        let result = barrier.list("prefix/").await;
        assert!(matches!(result, Err(BarrierError::Sealed)));
    }

    #[tokio::test]
    async fn sealed_barrier_rejects_exists() {
        let barrier = make_barrier();
        let result = barrier.exists("key").await;
        assert!(matches!(result, Err(BarrierError::Sealed)));
    }

    #[tokio::test]
    async fn unseal_then_put_get_roundtrip() {
        let barrier = make_barrier();
        let key = EncryptionKey::generate();
        barrier.unseal(key).await;

        barrier.put("sys/test", b"hello world").await.unwrap();
        let val = barrier.get("sys/test").await.unwrap();
        assert_eq!(val, Some(b"hello world".to_vec()));
    }

    #[tokio::test]
    async fn get_nonexistent_returns_none() {
        let barrier = make_barrier();
        barrier.unseal(EncryptionKey::generate()).await;

        let val = barrier.get("does/not/exist").await.unwrap();
        assert_eq!(val, None);
    }

    #[tokio::test]
    async fn delete_removes_key() {
        let barrier = make_barrier();
        barrier.unseal(EncryptionKey::generate()).await;

        barrier.put("key", b"val").await.unwrap();
        barrier.delete("key").await.unwrap();
        let val = barrier.get("key").await.unwrap();
        assert_eq!(val, None);
    }

    #[tokio::test]
    async fn list_returns_matching_keys() {
        let barrier = make_barrier();
        barrier.unseal(EncryptionKey::generate()).await;

        barrier.put("kv/data/a", b"1").await.unwrap();
        barrier.put("kv/data/b", b"2").await.unwrap();
        barrier.put("sys/config", b"3").await.unwrap();

        let keys = barrier.list("kv/data/").await.unwrap();
        assert_eq!(keys, vec!["kv/data/a", "kv/data/b"]);
    }

    #[tokio::test]
    async fn exists_works() {
        let barrier = make_barrier();
        barrier.unseal(EncryptionKey::generate()).await;

        assert!(!barrier.exists("key").await.unwrap());
        barrier.put("key", b"val").await.unwrap();
        assert!(barrier.exists("key").await.unwrap());
    }

    #[tokio::test]
    async fn seal_zeroizes_and_rejects() {
        let barrier = make_barrier();
        barrier.unseal(EncryptionKey::generate()).await;

        barrier.put("key", b"val").await.unwrap();
        barrier.seal().await;

        let result = barrier.get("key").await;
        assert!(matches!(result, Err(BarrierError::Sealed)));
    }

    #[tokio::test]
    async fn reseal_and_unseal_with_same_key_reads_data() {
        let storage = Arc::new(MemoryBackend::new());
        let barrier = Barrier::new(Arc::clone(&storage) as Arc<dyn StorageBackend>);
        let key = EncryptionKey::generate();

        barrier.unseal(key.clone()).await;
        barrier.put("key", b"persistent").await.unwrap();
        barrier.seal().await;

        // Re-unseal with the same key — data should still be readable.
        barrier.unseal(key).await;
        let val = barrier.get("key").await.unwrap();
        assert_eq!(val, Some(b"persistent".to_vec()));
    }

    #[tokio::test]
    async fn different_key_cannot_decrypt() {
        let storage = Arc::new(MemoryBackend::new());
        let barrier = Barrier::new(Arc::clone(&storage) as Arc<dyn StorageBackend>);

        let key1 = EncryptionKey::generate();
        barrier.unseal(key1).await;
        barrier.put("key", b"secret").await.unwrap();
        barrier.seal().await;

        // Unseal with a different key — decryption should fail.
        let key2 = EncryptionKey::generate();
        barrier.unseal(key2).await;
        let result = barrier.get("key").await;
        assert!(matches!(result, Err(BarrierError::Crypto(_))));
    }

    #[tokio::test]
    async fn put_raw_and_get_raw_bypass_encryption() {
        let barrier = make_barrier();
        let raw_data = b"already-encrypted-root-key";

        barrier.put_raw("sys/root_key", raw_data).await.unwrap();
        let val = barrier.get_raw("sys/root_key").await.unwrap();
        assert_eq!(val, Some(raw_data.to_vec()));
    }

    #[tokio::test]
    async fn is_unsealed_reflects_state() {
        let barrier = make_barrier();
        assert!(!barrier.is_unsealed().await);

        barrier.unseal(EncryptionKey::generate()).await;
        assert!(barrier.is_unsealed().await);

        barrier.seal().await;
        assert!(!barrier.is_unsealed().await);
    }
}

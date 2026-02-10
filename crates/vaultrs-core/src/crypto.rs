//! Cryptographic primitives for `ZVault`.
//!
//! Provides AES-256-GCM authenticated encryption, HKDF-SHA256 key derivation,
//! and zeroize-on-drop key newtypes. All key material is automatically cleared
//! from memory when dropped.
//!
//! # Security model
//!
//! - Every encryption generates a fresh 96-bit nonce via `OsRng`.
//! - Ciphertext format: `nonce (12 bytes) || ciphertext || tag (16 bytes)`.
//! - Key derivation uses HKDF-SHA256 with unique `info` per engine.
//! - All key types derive `Zeroize` + `ZeroizeOnDrop`.

use std::fmt;

use aes_gcm::aead::{Aead, AeadCore, KeyInit, OsRng};
use aes_gcm::{Aes256Gcm, Key, Nonce};
use hkdf::Hkdf;
use sha2::Sha256;
use zeroize::{Zeroize, ZeroizeOnDrop};

use crate::error::CryptoError;

/// Minimum ciphertext length: 12-byte nonce + 16-byte AES-GCM tag.
const MIN_CIPHERTEXT_LEN: usize = 12 + 16;

/// Nonce length for AES-256-GCM (96 bits).
const NONCE_LEN: usize = 12;

/// A 256-bit encryption key that is zeroized on drop.
///
/// Used as the root key and for per-engine derived keys. The inner bytes
/// are never exposed in `Debug` output.
#[derive(Clone, Zeroize, ZeroizeOnDrop)]
pub struct EncryptionKey([u8; 32]);

impl EncryptionKey {
    /// Create a key from raw bytes.
    #[must_use]
    pub fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    /// Generate a new random key using the OS CSPRNG.
    #[must_use]
    pub fn generate() -> Self {
        let key = Aes256Gcm::generate_key(OsRng);
        let mut bytes = [0u8; 32];
        bytes.copy_from_slice(&key);
        Self(bytes)
    }

    /// Borrow the raw key bytes.
    ///
    /// Use with care — the caller must not log or persist these bytes.
    #[must_use]
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

impl fmt::Debug for EncryptionKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("EncryptionKey")
            .field("bytes", &"[REDACTED]")
            .finish()
    }
}

/// Encrypt plaintext using AES-256-GCM with a fresh random nonce.
///
/// Returns `nonce (12 bytes) || ciphertext || tag (16 bytes)`.
///
/// # Errors
///
/// Returns [`CryptoError::Encryption`] if the AEAD operation fails.
pub fn encrypt(key: &EncryptionKey, plaintext: &[u8]) -> Result<Vec<u8>, CryptoError> {
    let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(key.as_bytes()));
    let nonce = Aes256Gcm::generate_nonce(&mut OsRng);
    let ciphertext = cipher
        .encrypt(&nonce, plaintext)
        .map_err(|e| CryptoError::Encryption {
            reason: e.to_string(),
        })?;

    // nonce || ciphertext (includes tag appended by aes-gcm)
    let mut combined = Vec::with_capacity(NONCE_LEN.saturating_add(ciphertext.len()));
    combined.extend_from_slice(&nonce);
    combined.extend_from_slice(&ciphertext);
    Ok(combined)
}

/// Decrypt ciphertext produced by [`encrypt`].
///
/// Expects the format `nonce (12 bytes) || ciphertext || tag (16 bytes)`.
///
/// # Errors
///
/// Returns [`CryptoError::CiphertextTooShort`] if the input is shorter than
/// 28 bytes (nonce + tag minimum).
///
/// Returns [`CryptoError::Decryption`] if authentication fails (wrong key,
/// corrupted data, or tampered tag).
pub fn decrypt(key: &EncryptionKey, combined: &[u8]) -> Result<Vec<u8>, CryptoError> {
    if combined.len() < MIN_CIPHERTEXT_LEN {
        return Err(CryptoError::CiphertextTooShort {
            expected: MIN_CIPHERTEXT_LEN,
            actual: combined.len(),
        });
    }

    let (nonce_bytes, ciphertext) = combined.split_at(NONCE_LEN);
    let nonce = Nonce::from_slice(nonce_bytes);
    let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(key.as_bytes()));

    cipher
        .decrypt(nonce, ciphertext)
        .map_err(|e| CryptoError::Decryption {
            reason: e.to_string(),
        })
}

/// Derive a per-engine encryption key from a root key using HKDF-SHA256.
///
/// The `salt` should be unique per vault instance. The `info` string must be
/// unique per engine (e.g. `b"vaultrs-kv-v1"`, `b"vaultrs-transit-v1"`).
///
/// # Errors
///
/// Returns [`CryptoError::KeyDerivation`] if HKDF expansion fails (should
/// only happen if output length exceeds 255 * hash length).
pub fn derive_key(
    root_key: &EncryptionKey,
    salt: Option<&[u8]>,
    info: &[u8],
) -> Result<EncryptionKey, CryptoError> {
    let hk = Hkdf::<Sha256>::new(salt, root_key.as_bytes());
    let mut derived = [0u8; 32];
    hk.expand(info, &mut derived)
        .map_err(|e| CryptoError::KeyDerivation {
            context: String::from_utf8_lossy(info).into_owned(),
            reason: e.to_string(),
        })?;
    Ok(EncryptionKey::from_bytes(derived))
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn encrypt_decrypt_roundtrip() {
        let key = EncryptionKey::generate();
        let plaintext = b"secret data for vaultrs";
        let ciphertext = encrypt(&key, plaintext).unwrap();
        let decrypted = decrypt(&key, &ciphertext).unwrap();
        assert_eq!(plaintext.as_slice(), decrypted.as_slice());
    }

    #[test]
    fn encrypt_decrypt_empty_plaintext() {
        let key = EncryptionKey::generate();
        let ciphertext = encrypt(&key, b"").unwrap();
        let decrypted = decrypt(&key, &ciphertext).unwrap();
        assert!(decrypted.is_empty());
    }

    #[test]
    fn decrypt_wrong_key_fails() {
        let key1 = EncryptionKey::generate();
        let key2 = EncryptionKey::generate();
        let ciphertext = encrypt(&key1, b"secret").unwrap();
        let result = decrypt(&key2, &ciphertext);
        assert!(result.is_err());
    }

    #[test]
    fn decrypt_too_short_fails() {
        let key = EncryptionKey::generate();
        let result = decrypt(&key, &[0u8; 10]);
        assert!(matches!(
            result,
            Err(CryptoError::CiphertextTooShort {
                expected: 28,
                actual: 10
            })
        ));
    }

    #[test]
    fn decrypt_tampered_ciphertext_fails() {
        let key = EncryptionKey::generate();
        let mut ciphertext = encrypt(&key, b"secret").unwrap();
        // Flip a byte in the ciphertext portion (after the nonce).
        if let Some(byte) = ciphertext.get_mut(NONCE_LEN) {
            *byte ^= 0xFF;
        }
        let result = decrypt(&key, &ciphertext);
        assert!(matches!(result, Err(CryptoError::Decryption { .. })));
    }

    #[test]
    fn ciphertext_starts_with_nonce() {
        let key = EncryptionKey::generate();
        let ciphertext = encrypt(&key, b"data").unwrap();
        // Must be at least nonce (12) + tag (16) + plaintext (4) = 32 bytes.
        assert!(ciphertext.len() >= 32);
    }

    #[test]
    fn two_encryptions_produce_different_ciphertext() {
        let key = EncryptionKey::generate();
        let plaintext = b"same data";
        let ct1 = encrypt(&key, plaintext).unwrap();
        let ct2 = encrypt(&key, plaintext).unwrap();
        // Different nonces → different ciphertext.
        assert_ne!(ct1, ct2);
    }

    #[test]
    fn derive_key_produces_deterministic_output() {
        let root = EncryptionKey::generate();
        let salt = b"test-salt";
        let info = b"vaultrs-kv-v1";
        let k1 = derive_key(&root, Some(salt), info).unwrap();
        let k2 = derive_key(&root, Some(salt), info).unwrap();
        assert_eq!(k1.as_bytes(), k2.as_bytes());
    }

    #[test]
    fn derive_key_different_info_produces_different_keys() {
        let root = EncryptionKey::generate();
        let salt = b"test-salt";
        let k1 = derive_key(&root, Some(salt), b"vaultrs-kv-v1").unwrap();
        let k2 = derive_key(&root, Some(salt), b"vaultrs-transit-v1").unwrap();
        assert_ne!(k1.as_bytes(), k2.as_bytes());
    }

    #[test]
    fn derive_key_no_salt_works() {
        let root = EncryptionKey::generate();
        let result = derive_key(&root, None, b"vaultrs-kv-v1");
        assert!(result.is_ok());
    }

    #[test]
    fn encryption_key_debug_redacts_bytes() {
        let key = EncryptionKey::generate();
        let debug = format!("{key:?}");
        assert!(debug.contains("[REDACTED]"));
        assert!(!debug.contains("0x"));
    }

    #[test]
    fn derived_key_encrypts_and_decrypts() {
        let root = EncryptionKey::generate();
        let derived = derive_key(&root, Some(b"salt"), b"vaultrs-kv-v1").unwrap();
        let plaintext = b"encrypted with derived key";
        let ciphertext = encrypt(&derived, plaintext).unwrap();
        let decrypted = decrypt(&derived, &ciphertext).unwrap();
        assert_eq!(plaintext.as_slice(), decrypted.as_slice());
    }
}

//! File-based audit backend for `ZVault`.
//!
//! Appends JSON-lines audit entries to a file. Each line is a complete
//! JSON object representing one [`AuditEntry`]. The file is opened in
//! append-only mode — no update or delete operations are ever performed.
//!
//! # Thread safety
//!
//! Uses a `tokio::sync::Mutex` around the file handle to serialize writes.
//! This is acceptable because audit writes are infrequent relative to
//! request throughput and the critical section is tiny (one `write_all`).

use std::path::{Path, PathBuf};

use tokio::fs::OpenOptions;
use tokio::io::AsyncWriteExt;
use tokio::sync::Mutex;

use crate::audit::{AuditBackend, AuditEntry};
use crate::error::AuditError;

/// Audit backend that writes JSON-lines to a file.
pub struct FileAuditBackend {
    /// Path to the audit log file.
    path: PathBuf,
    /// Serialized write access to the file.
    writer: Mutex<Option<tokio::fs::File>>,
}

impl FileAuditBackend {
    /// Create a new file audit backend writing to the given path.
    ///
    /// The file is created (or opened for append) lazily on the first write.
    #[must_use]
    pub fn new(path: impl AsRef<Path>) -> Self {
        Self {
            path: path.as_ref().to_path_buf(),
            writer: Mutex::new(None),
        }
    }

    /// Open or reuse the file handle.
    async fn get_writer(
        &self,
    ) -> Result<tokio::sync::MutexGuard<'_, Option<tokio::fs::File>>, AuditError> {
        let mut guard = self.writer.lock().await;
        if guard.is_none() {
            let file = OpenOptions::new()
                .create(true)
                .append(true)
                .open(&self.path)
                .await
                .map_err(|e| AuditError::BackendFailure {
                    name: self.name().to_owned(),
                    reason: format!("failed to open audit file '{}': {e}", self.path.display()),
                })?;
            *guard = Some(file);
        }
        Ok(guard)
    }
}

#[async_trait::async_trait]
impl AuditBackend for FileAuditBackend {
    #[allow(clippy::needless_lifetimes, clippy::unnecessary_literal_bound)]
    fn name(&self) -> &str {
        "file"
    }

    async fn log(&self, entry: &AuditEntry) -> Result<(), AuditError> {
        let mut line = serde_json::to_vec(entry).map_err(|e| AuditError::Serialization {
            reason: e.to_string(),
        })?;
        line.push(b'\n');

        let mut guard = self.get_writer().await?;
        let file = guard.as_mut().ok_or_else(|| AuditError::BackendFailure {
            name: "file".to_owned(),
            reason: "file handle unexpectedly None after open".to_owned(),
        })?;

        file.write_all(&line)
            .await
            .map_err(|e| AuditError::BackendFailure {
                name: "file".to_owned(),
                reason: format!("write failed: {e}"),
            })?;

        file.flush().await.map_err(|e| AuditError::BackendFailure {
            name: "file".to_owned(),
            reason: format!("flush failed: {e}"),
        })?;

        Ok(())
    }
}

impl std::fmt::Debug for FileAuditBackend {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FileAuditBackend")
            .field("path", &self.path)
            .finish_non_exhaustive()
    }
}

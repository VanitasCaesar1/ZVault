//! Engine mount table for `ZVault`.
//!
//! The mount table maps path prefixes to secrets engine types. When a request
//! arrives at `/v1/secret/data/foo`, the router strips `/v1/`, looks up
//! `secret/` in the mount table, and dispatches to the KV engine.
//!
//! Mount entries are persisted through the barrier at `sys/mounts`.

use std::collections::HashMap;
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use tracing::info;

use crate::barrier::Barrier;
use crate::error::MountError;

/// Storage key for the serialized mount table.
const MOUNT_TABLE_KEY: &str = "sys/mounts";

/// A single mount entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MountEntry {
    /// The mount path (e.g., `secret/`, `database/`, `transit/`).
    pub path: String,
    /// The engine type (e.g., `kv`, `database`, `transit`, `pki`).
    pub engine_type: String,
    /// Optional description.
    pub description: String,
    /// Engine-specific configuration.
    pub config: serde_json::Value,
}

/// The full mount table — maps path prefixes to engine entries.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MountTable {
    /// Mount entries keyed by path.
    pub entries: HashMap<String, MountEntry>,
}

/// Manages the mount table with persistence through the barrier.
pub struct MountManager {
    barrier: Arc<Barrier>,
    /// In-memory cache of the mount table, protected by `RwLock` for
    /// read-heavy access (every request checks the mount table).
    table: RwLock<MountTable>,
}

impl MountManager {
    /// Create a mount manager with an empty table, without loading from storage.
    ///
    /// Used at startup when the vault is sealed and storage is inaccessible.
    /// The table will be populated when the vault is unsealed and engines are
    /// mounted.
    #[must_use]
    pub fn empty(barrier: Arc<Barrier>) -> Self {
        Self {
            barrier,
            table: RwLock::new(MountTable::default()),
        }
    }

    /// Create a new mount manager and load the table from storage.
    ///
    /// If no table exists in storage, starts with an empty table.
    ///
    /// # Errors
    ///
    /// Returns [`MountError::Barrier`] if storage access fails.
    pub async fn new(barrier: Arc<Barrier>) -> Result<Self, MountError> {
        let table = match barrier.get(MOUNT_TABLE_KEY).await {
            Ok(Some(data)) => serde_json::from_slice(&data).unwrap_or_default(),
            Ok(None) => MountTable::default(),
            Err(e) => return Err(MountError::Barrier(e)),
        };

        Ok(Self {
            barrier,
            table: RwLock::new(table),
        })
    }

    /// Mount a new engine at the given path.
    ///
    /// # Errors
    ///
    /// - [`MountError::AlreadyMounted`] if the path is already in use.
    /// - [`MountError::InvalidPath`] if the path is empty or doesn't end with `/`.
    /// - [`MountError::Barrier`] if persistence fails.
    pub async fn mount(&self, entry: MountEntry) -> Result<(), MountError> {
        if entry.path.is_empty() {
            return Err(MountError::InvalidPath {
                reason: "mount path cannot be empty".to_owned(),
            });
        }

        // Ensure path ends with `/`.
        let path = if entry.path.ends_with('/') {
            entry.path.clone()
        } else {
            format!("{}/", entry.path)
        };

        let mut table = self.table.write().await;

        if table.entries.contains_key(&path) {
            return Err(MountError::AlreadyMounted { path });
        }

        let normalized = MountEntry {
            path: path.clone(),
            ..entry
        };
        table.entries.insert(path.clone(), normalized);

        self.persist(&table).await?;

        info!(path = %path, "engine mounted");

        Ok(())
    }

    /// Unmount an engine at the given path.
    ///
    /// # Errors
    ///
    /// - [`MountError::NotFound`] if the path is not mounted.
    /// - [`MountError::Barrier`] if persistence fails.
    pub async fn unmount(&self, path: &str) -> Result<MountEntry, MountError> {
        let normalized = if path.ends_with('/') {
            path.to_owned()
        } else {
            format!("{path}/")
        };

        let mut table = self.table.write().await;

        let entry = table
            .entries
            .remove(&normalized)
            .ok_or_else(|| MountError::NotFound {
                path: normalized.clone(),
            })?;

        self.persist(&table).await?;

        info!(path = %normalized, "engine unmounted");

        Ok(entry)
    }

    /// Look up which engine handles a given request path.
    ///
    /// Returns the mount entry and the remaining path after the mount prefix.
    pub async fn resolve(&self, path: &str) -> Option<(MountEntry, String)> {
        let table = self.table.read().await;

        // Find the longest matching prefix.
        let mut best_match: Option<(&str, &MountEntry)> = None;

        for (mount_path, entry) in &table.entries {
            if path.starts_with(mount_path.as_str()) {
                match best_match {
                    None => best_match = Some((mount_path, entry)),
                    Some((current_best, _)) if mount_path.len() > current_best.len() => {
                        best_match = Some((mount_path, entry));
                    }
                    _ => {}
                }
            }
        }

        best_match.map(|(prefix, entry)| {
            let remainder = path.strip_prefix(prefix).unwrap_or(path);
            (entry.clone(), remainder.to_owned())
        })
    }

    /// List all mount entries.
    pub async fn list(&self) -> Vec<MountEntry> {
        let table = self.table.read().await;
        table.entries.values().cloned().collect()
    }

    /// Persist the mount table to storage through the barrier.
    async fn persist(&self, table: &MountTable) -> Result<(), MountError> {
        let bytes = serde_json::to_vec(table).map_err(|e| MountError::InvalidPath {
            reason: format!("mount table serialization failed: {e}"),
        })?;
        self.barrier.put(MOUNT_TABLE_KEY, &bytes).await?;
        Ok(())
    }
}

impl std::fmt::Debug for MountManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MountManager").finish_non_exhaustive()
    }
}

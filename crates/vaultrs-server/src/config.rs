//! Server configuration for `ZVault`.
//!
//! Loads configuration from environment variables with sensible defaults.
//! All settings can be overridden via `VAULTRS_*` environment variables.

use std::net::SocketAddr;

/// Server configuration.
#[derive(Debug, Clone)]
pub struct ServerConfig {
    /// Address to bind the HTTP listener to.
    pub bind_addr: SocketAddr,
    /// Storage backend type.
    pub storage_backend: StorageBackendType,
    /// Log level filter (e.g., `info`, `debug`, `warn`).
    pub log_level: String,
    /// Path to the audit log file (if file audit is enabled).
    pub audit_file_path: Option<String>,
    /// Whether to enable the default transit engine mount.
    pub enable_transit: bool,
    /// Lease expiry scan interval in seconds.
    pub lease_scan_interval_secs: u64,
    /// Whether to skip `mlock` (for development without root/`CAP_IPC_LOCK`).
    pub disable_mlock: bool,
}

/// Supported storage backend types.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StorageBackendType {
    /// In-memory (development only, data lost on restart).
    Memory,
    /// `RocksDB` persistent storage.
    RocksDb { path: String },
    /// Redb persistent storage.
    Redb { path: String },
}

impl ServerConfig {
    /// Load configuration from environment variables.
    ///
    /// Environment variables:
    /// - `PORT` — port to bind on (Railway convention, binds to `0.0.0.0`)
    /// - `VAULTRS_BIND_ADDR` — full bind address (overrides `PORT`, default: `127.0.0.1:8200`)
    /// - `VAULTRS_STORAGE` — `memory`, `rocksdb`, or `redb` (default: `memory`)
    /// - `VAULTRS_STORAGE_PATH` — path for persistent backends (default: `./data`)
    /// - `VAULTRS_LOG_LEVEL` — log filter (default: `info`)
    /// - `VAULTRS_AUDIT_FILE` — path to audit log file (optional)
    /// - `VAULTRS_ENABLE_TRANSIT` — enable transit engine (default: `true`)
    /// - `VAULTRS_LEASE_SCAN_INTERVAL` — seconds between lease scans (default: `60`)
    /// - `VAULTRS_DISABLE_MLOCK` — skip `mlockall` for dev environments (default: `false`)
    #[must_use]
    pub fn from_env() -> Self {
        // Priority: VAULTRS_BIND_ADDR > PORT (Railway) > default 127.0.0.1:8200
        let bind_addr = if let Ok(addr) = std::env::var("VAULTRS_BIND_ADDR") {
            addr.parse()
                .unwrap_or_else(|_| SocketAddr::from(([127, 0, 0, 1], 8200)))
        } else if let Ok(port_str) = std::env::var("PORT") {
            let port: u16 = port_str.parse().unwrap_or(8200);
            SocketAddr::from(([0, 0, 0, 0], port))
        } else {
            SocketAddr::from(([127, 0, 0, 1], 8200))
        };

        let storage_path = std::env::var("VAULTRS_STORAGE_PATH")
            .unwrap_or_else(|_| "./data".to_owned());

        let storage_backend = match std::env::var("VAULTRS_STORAGE")
            .unwrap_or_else(|_| "memory".to_owned())
            .to_lowercase()
            .as_str()
        {
            "rocksdb" => StorageBackendType::RocksDb { path: storage_path },
            "redb" => StorageBackendType::Redb { path: storage_path },
            _ => StorageBackendType::Memory,
        };

        let log_level = std::env::var("VAULTRS_LOG_LEVEL")
            .unwrap_or_else(|_| "info".to_owned());

        let audit_file_path = std::env::var("VAULTRS_AUDIT_FILE").ok();

        let enable_transit = std::env::var("VAULTRS_ENABLE_TRANSIT")
            .map(|v| v != "false" && v != "0")
            .unwrap_or(true);

        let lease_scan_interval_secs = std::env::var("VAULTRS_LEASE_SCAN_INTERVAL")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(60);

        let disable_mlock = std::env::var("VAULTRS_DISABLE_MLOCK")
            .map(|v| v == "true" || v == "1")
            .unwrap_or(false);

        Self {
            bind_addr,
            storage_backend,
            log_level,
            audit_file_path,
            enable_transit,
            lease_scan_interval_secs,
            disable_mlock,
        }
    }
}

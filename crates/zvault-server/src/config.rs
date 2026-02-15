//! Server configuration for `ZVault`.
//!
//! Loads configuration from environment variables with sensible defaults.
//! All settings can be overridden via `ZVAULT_*` environment variables.

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
    /// Spring OAuth configuration (optional — enables "Sign in with Spring").
    pub spring_oauth: Option<SpringOAuthConfig>,
}

/// Configuration for Spring OAuth 2.0 / OIDC integration.
#[derive(Debug, Clone)]
pub struct SpringOAuthConfig {
    /// Base URL of the Spring auth server (e.g., `https://auth.puddlesearch.in`).
    pub auth_url: String,
    /// OAuth client ID registered in Spring.
    pub client_id: String,
    /// OAuth client secret.
    pub client_secret: String,
    /// Redirect URI for the callback (auto-derived if not set).
    pub redirect_uri: Option<String>,
    /// Default vault policy to assign to Spring-authenticated users.
    pub default_policy: String,
    /// Vault policy to assign to Spring admin users.
    pub admin_policy: String,
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
    /// PostgreSQL persistent storage (recommended for Railway / cloud).
    Postgres { url: String },
}

impl ServerConfig {
    /// Load configuration from environment variables.
    ///
    /// Environment variables:
    /// - `PORT` — port to bind on (Railway convention, binds to `0.0.0.0`)
    /// - `ZVAULT_BIND_ADDR` — full bind address (overrides `PORT`, default: `127.0.0.1:8200`)
    /// - `ZVAULT_STORAGE` — `memory`, `rocksdb`, `redb`, or `postgres` (default: `memory`)
    /// - `ZVAULT_STORAGE_PATH` — path for persistent backends (default: `./data`)
    /// - `DATABASE_URL` — PostgreSQL connection string (required when `ZVAULT_STORAGE=postgres`)
    /// - `ZVAULT_STORAGE_PATH` — path for persistent backends (default: `./data`)
    /// - `ZVAULT_LOG_LEVEL` — log filter (default: `info`)
    /// - `ZVAULT_AUDIT_FILE` — path to audit log file (optional)
    /// - `ZVAULT_ENABLE_TRANSIT` — enable transit engine (default: `true`)
    /// - `ZVAULT_LEASE_SCAN_INTERVAL` — seconds between lease scans (default: `60`)
    /// - `ZVAULT_DISABLE_MLOCK` — skip `mlockall` for dev environments (default: `false`)
    #[must_use]
    pub fn from_env() -> Self {
        // Priority: ZVAULT_BIND_ADDR > PORT (Railway) > default 127.0.0.1:8200
        let bind_addr = if let Ok(addr) = std::env::var("ZVAULT_BIND_ADDR") {
            addr.parse()
                .unwrap_or_else(|_| SocketAddr::from(([127, 0, 0, 1], 8200)))
        } else if let Ok(port_str) = std::env::var("PORT") {
            let port: u16 = port_str.parse().unwrap_or(8200);
            SocketAddr::from(([0, 0, 0, 0], port))
        } else {
            SocketAddr::from(([127, 0, 0, 1], 8200))
        };

        let storage_path = std::env::var("ZVAULT_STORAGE_PATH")
            .unwrap_or_else(|_| "./data".to_owned());

        let storage_backend = match std::env::var("ZVAULT_STORAGE")
            .unwrap_or_else(|_| "memory".to_owned())
            .to_lowercase()
            .as_str()
        {
            "rocksdb" => StorageBackendType::RocksDb { path: storage_path },
            "redb" => StorageBackendType::Redb { path: storage_path },
            "postgres" | "postgresql" => {
                let url = std::env::var("DATABASE_URL")
                    .unwrap_or_else(|_| "postgres://localhost/zvault".to_owned());
                StorageBackendType::Postgres { url }
            }
            _ => StorageBackendType::Memory,
        };

        let log_level = std::env::var("ZVAULT_LOG_LEVEL")
            .unwrap_or_else(|_| "info".to_owned());

        let audit_file_path = std::env::var("ZVAULT_AUDIT_FILE").ok();

        let enable_transit = std::env::var("ZVAULT_ENABLE_TRANSIT")
            .map(|v| v != "false" && v != "0")
            .unwrap_or(true);

        let lease_scan_interval_secs = std::env::var("ZVAULT_LEASE_SCAN_INTERVAL")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(60);

        let disable_mlock = std::env::var("ZVAULT_DISABLE_MLOCK")
            .map(|v| v == "true" || v == "1")
            .unwrap_or(false);

        // Spring OAuth — enabled when SPRING_AUTH_URL is set.
        let spring_oauth = std::env::var("SPRING_AUTH_URL").ok().map(|auth_url| {
            SpringOAuthConfig {
                auth_url,
                client_id: std::env::var("SPRING_CLIENT_ID")
                    .unwrap_or_else(|_| "zvault-dashboard".to_owned()),
                client_secret: std::env::var("SPRING_CLIENT_SECRET")
                    .unwrap_or_default(),
                redirect_uri: std::env::var("SPRING_REDIRECT_URI").ok(),
                default_policy: std::env::var("SPRING_DEFAULT_POLICY")
                    .unwrap_or_else(|_| "default".to_owned()),
                admin_policy: std::env::var("SPRING_ADMIN_POLICY")
                    .unwrap_or_else(|_| "root".to_owned()),
            }
        });

        Self {
            bind_addr,
            storage_backend,
            log_level,
            audit_file_path,
            enable_transit,
            lease_scan_interval_secs,
            disable_mlock,
            spring_oauth,
        }
    }
}

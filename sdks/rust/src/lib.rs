//! Official `ZVault` SDK for Rust.
//!
//! Fetch secrets at runtime from `ZVault` Cloud with in-memory caching,
//! retry with backoff, and graceful degradation.
//!
//! # Example
//!
//! ```rust,no_run
//! use zvault_sdk::ZVault;
//!
//! # async fn example() -> Result<(), zvault_sdk::ZVaultError> {
//! let client = ZVault::new(std::env::var("ZVAULT_TOKEN").unwrap_or_default())?;
//! let secrets = client.get_all("production").await?;
//! if let Some(db_url) = secrets.get("DATABASE_URL") {
//!     println!("DB: {db_url}");
//! }
//! # Ok(())
//! # }
//! ```

mod client;
mod error;
mod types;

pub use error::ZVaultError;
pub use types::{HealthStatus, SecretEntry, SecretKey};

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use tokio::sync::RwLock;

const DEFAULT_BASE_URL: &str = "https://api.zvault.cloud";
const DEFAULT_CACHE_TTL: Duration = Duration::from_secs(300);
const DEFAULT_TIMEOUT: Duration = Duration::from_secs(10);
const DEFAULT_MAX_RETRIES: u32 = 3;
const RETRY_BASE_DELAY: Duration = Duration::from_millis(500);

/// Configuration for the `ZVault` client.
#[derive(Debug, Clone)]
pub struct ZVaultConfig {
    /// Service token or auth token.
    pub token: String,
    /// API base URL. Default: `https://api.zvault.cloud`.
    pub base_url: String,
    /// Organization ID.
    pub org_id: String,
    /// Project ID.
    pub project_id: String,
    /// Default environment slug.
    pub default_env: String,
    /// Cache TTL. Default: 5 minutes.
    pub cache_ttl: Duration,
    /// Request timeout. Default: 10 seconds.
    pub timeout: Duration,
    /// Max retry attempts. Default: 3.
    pub max_retries: u32,
}

impl Default for ZVaultConfig {
    fn default() -> Self {
        Self {
            token: String::new(),
            base_url: DEFAULT_BASE_URL.to_owned(),
            org_id: String::new(),
            project_id: String::new(),
            default_env: "development".to_owned(),
            cache_ttl: DEFAULT_CACHE_TTL,
            timeout: DEFAULT_TIMEOUT,
            max_retries: DEFAULT_MAX_RETRIES,
        }
    }
}

struct CacheEntry {
    secrets: HashMap<String, String>,
    expires_at: Instant,
}

/// `ZVault` SDK client.
pub struct ZVault {
    token: String,
    base_url: String,
    org_id: String,
    project_id: String,
    default_env: String,
    cache_ttl: Duration,
    max_retries: u32,
    client: reqwest::Client,
    cache: Arc<RwLock<HashMap<String, CacheEntry>>>,
}

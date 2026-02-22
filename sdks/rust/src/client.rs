//! `ZVault` client implementation.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use reqwest::StatusCode;
use tokio::sync::RwLock;

use crate::error::ZVaultError;
use crate::types::{
    ApiErrorBody, HealthStatus, SecretEntry, SecretKey, SecretKeysResponse, SecretResponse,
};
use crate::{
    CacheEntry, ZVault, ZVaultConfig, DEFAULT_BASE_URL, DEFAULT_CACHE_TTL, DEFAULT_MAX_RETRIES,
    DEFAULT_TIMEOUT, RETRY_BASE_DELAY,
};

impl ZVault {
    /// Create a new client with just a token. Reads other config from env vars.
    ///
    /// # Errors
    ///
    /// Returns `ZVaultError::Config` if the token is empty.
    pub fn new(token: String) -> Result<Self, ZVaultError> {
        Self::with_config(ZVaultConfig {
            token,
            ..Default::default()
        })
    }

    /// Create a new client with full configuration.
    ///
    /// # Errors
    ///
    /// Returns `ZVaultError::Config` if the token is empty.
    #[allow(clippy::needless_pass_by_value)]
    pub fn with_config(cfg: ZVaultConfig) -> Result<Self, ZVaultError> {
        let token = first_non_empty(&[
            &cfg.token,
            &std::env::var("ZVAULT_TOKEN").unwrap_or_default(),
        ]);
        if token.is_empty() {
            return Err(ZVaultError::Config(
                "missing token — set ZVAULT_TOKEN env var or pass token in config".to_owned(),
            ));
        }

        let base_url = first_non_empty(&[
            &cfg.base_url,
            &std::env::var("ZVAULT_URL").unwrap_or_default(),
            DEFAULT_BASE_URL,
        ])
        .trim_end_matches('/')
        .to_owned();

        let org_id = first_non_empty(&[
            &cfg.org_id,
            &std::env::var("ZVAULT_ORG_ID").unwrap_or_default(),
        ]);

        let project_id = first_non_empty(&[
            &cfg.project_id,
            &std::env::var("ZVAULT_PROJECT_ID").unwrap_or_default(),
        ]);

        let default_env = first_non_empty(&[
            &cfg.default_env,
            &std::env::var("ZVAULT_ENV").unwrap_or_default(),
            "development",
        ]);

        let cache_ttl = if cfg.cache_ttl.is_zero() {
            DEFAULT_CACHE_TTL
        } else {
            cfg.cache_ttl
        };

        let timeout = if cfg.timeout.is_zero() {
            DEFAULT_TIMEOUT
        } else {
            cfg.timeout
        };

        let max_retries = if cfg.max_retries == 0 {
            DEFAULT_MAX_RETRIES
        } else {
            cfg.max_retries
        };

        let client = reqwest::Client::builder()
            .timeout(timeout)
            .user_agent("zvault-rust-sdk/0.1.0")
            .build()
            .map_err(ZVaultError::Network)?;

        Ok(Self {
            token,
            base_url,
            org_id,
            project_id,
            default_env,
            cache_ttl,
            max_retries,
            client,
            cache: Arc::new(RwLock::new(HashMap::new())),
        })
    }

    /// Fetch all secrets for an environment.
    ///
    /// Results are cached in-memory. On network failure, returns last-known
    /// cached values (graceful degradation).
    ///
    /// # Errors
    ///
    /// Returns an error if the API is unreachable and no cached values exist.
    pub async fn get_all(&self, env: &str) -> Result<HashMap<String, String>, ZVaultError> {
        let env = self.resolve_env(env);
        self.require_project_config()?;

        let path = format!(
            "/orgs/{}/projects/{}/envs/{}/secrets",
            self.org_id, self.project_id, env
        );

        match self.request::<SecretKeysResponse>("GET", &path, None).await {
            Ok(keys_resp) => {
                let mut secrets = HashMap::with_capacity(keys_resp.keys.len());
                for k in &keys_resp.keys {
                    let secret_path =
                        format!("{}/{}", path, urlencoding::encode(&k.key));
                    if let Ok(resp) = self
                        .request::<SecretResponse>("GET", &secret_path, None)
                        .await
                    {
                        secrets.insert(resp.secret.key, resp.secret.value);
                    }
                }

                // Update cache
                let mut cache = self.cache.write().await;
                cache.insert(
                    env.clone(),
                    CacheEntry {
                        secrets: secrets.clone(),
                        expires_at: Instant::now() + self.cache_ttl,
                    },
                );

                Ok(secrets)
            }
            Err(err) => {
                // Graceful degradation
                let cache = self.cache.read().await;
                if let Some(entry) = cache.get(&env) {
                    if Instant::now() < entry.expires_at {
                        return Ok(entry.secrets.clone());
                    }
                }
                Err(err)
            }
        }
    }

    /// Fetch a single secret by key. Checks cache first.
    ///
    /// # Errors
    ///
    /// Returns `ZVaultError::NotFound` if the secret doesn't exist.
    pub async fn get(&self, key: &str, env: &str) -> Result<String, ZVaultError> {
        let env = self.resolve_env(env);
        self.require_project_config()?;

        // Check cache
        {
            let cache = self.cache.read().await;
            if let Some(entry) = cache.get(&env) {
                if Instant::now() < entry.expires_at {
                    if let Some(val) = entry.secrets.get(key) {
                        return Ok(val.clone());
                    }
                }
            }
        }

        let path = format!(
            "/orgs/{}/projects/{}/envs/{}/secrets/{}",
            self.org_id,
            self.project_id,
            env,
            urlencoding::encode(key)
        );

        match self.request::<SecretResponse>("GET", &path, None).await {
            Ok(resp) => {
                // Cache the value
                let mut cache = self.cache.write().await;
                let entry = cache.entry(env.clone()).or_insert_with(|| CacheEntry {
                    secrets: HashMap::new(),
                    expires_at: Instant::now() + self.cache_ttl,
                });
                entry.secrets.insert(key.to_owned(), resp.secret.value.clone());
                Ok(resp.secret.value)
            }
            Err(ZVaultError::Api { status_code: 404, .. }) => {
                Err(ZVaultError::NotFound {
                    key: key.to_owned(),
                    env,
                })
            }
            Err(e) => Err(e),
        }
    }

    /// List secret keys (no values) for an environment.
    ///
    /// # Errors
    ///
    /// Returns an error if the API request fails.
    pub async fn list_keys(&self, env: &str) -> Result<Vec<SecretKey>, ZVaultError> {
        let env = self.resolve_env(env);
        self.require_project_config()?;

        let path = format!(
            "/orgs/{}/projects/{}/envs/{}/secrets",
            self.org_id, self.project_id, env
        );
        let resp = self.request::<SecretKeysResponse>("GET", &path, None).await?;
        Ok(resp.keys)
    }

    /// Set a secret value. Requires write permission.
    ///
    /// # Errors
    ///
    /// Returns an error if the API request fails.
    pub async fn set(
        &self,
        key: &str,
        value: &str,
        env: &str,
        comment: &str,
    ) -> Result<SecretEntry, ZVaultError> {
        let env = self.resolve_env(env);
        self.require_project_config()?;

        let path = format!(
            "/orgs/{}/projects/{}/envs/{}/secrets/{}",
            self.org_id,
            self.project_id,
            env,
            urlencoding::encode(key)
        );
        let body = serde_json::json!({ "value": value, "comment": comment });
        let resp = self
            .request::<SecretResponse>("PUT", &path, Some(body))
            .await?;

        // Update cache
        let mut cache = self.cache.write().await;
        let entry = cache.entry(env).or_insert_with(|| CacheEntry {
            secrets: HashMap::new(),
            expires_at: Instant::now() + self.cache_ttl,
        });
        entry.secrets.insert(key.to_owned(), value.to_owned());

        Ok(resp.secret)
    }

    /// Delete a secret. Requires write permission.
    ///
    /// # Errors
    ///
    /// Returns an error if the API request fails.
    pub async fn delete(&self, key: &str, env: &str) -> Result<(), ZVaultError> {
        let env = self.resolve_env(env);
        self.require_project_config()?;

        let path = format!(
            "/orgs/{}/projects/{}/envs/{}/secrets/{}",
            self.org_id,
            self.project_id,
            env,
            urlencoding::encode(key)
        );
        self.request::<serde_json::Value>("DELETE", &path, None)
            .await?;
        Ok(())
    }

    /// Check if the API is reachable and the token is valid.
    pub async fn healthy(&self) -> HealthStatus {
        let start = Instant::now();
        let ok = self
            .request::<serde_json::Value>("GET", "/me", None)
            .await
            .is_ok();

        let cache = self.cache.read().await;
        let cached = cache
            .values()
            .filter(|e| Instant::now() < e.expires_at)
            .map(|e| e.secrets.len())
            .sum();

        HealthStatus {
            ok,
            latency_ms: start.elapsed().as_millis(),
            cached_secrets: cached,
        }
    }

    // --- Private ---

    fn resolve_env(&self, env: &str) -> String {
        if env.is_empty() {
            self.default_env.clone()
        } else {
            env.to_owned()
        }
    }

    fn require_project_config(&self) -> Result<(), ZVaultError> {
        if self.org_id.is_empty() {
            return Err(ZVaultError::Config(
                "missing org_id — set ZVAULT_ORG_ID env var or pass org_id in config".to_owned(),
            ));
        }
        if self.project_id.is_empty() {
            return Err(ZVaultError::Config(
                "missing project_id — set ZVAULT_PROJECT_ID env var or pass project_id in config"
                    .to_owned(),
            ));
        }
        Ok(())
    }

    async fn request<T: serde::de::DeserializeOwned>(
        &self,
        method: &str,
        path: &str,
        body: Option<serde_json::Value>,
    ) -> Result<T, ZVaultError> {
        let url = format!("{}/v1/cloud{}", self.base_url, path);
        let mut last_err = None;

        for attempt in 0..=self.max_retries {
            let mut req = match method {
                "PUT" => self.client.put(&url),
                "DELETE" => self.client.delete(&url),
                "POST" => self.client.post(&url),
                _ => self.client.get(&url),
            };

            req = req.header("Authorization", format!("Bearer {}", self.token));

            if let Some(ref b) = body {
                req = req.json(b);
            }

            match req.send().await {
                Ok(resp) => {
                    let status = resp.status();

                    if status.is_success() {
                        let text = resp.text().await.map_err(ZVaultError::Network)?;
                        if text.is_empty() {
                            // For DELETE responses, deserialize a default
                            return serde_json::from_str("{}").map_err(ZVaultError::Json);
                        }
                        return serde_json::from_str(&text).map_err(ZVaultError::Json);
                    }

                    // Parse error body
                    let error_text = resp.text().await.unwrap_or_default();
                    let msg = serde_json::from_str::<ApiErrorBody>(&error_text)
                        .ok()
                        .and_then(|b| b.error)
                        .and_then(|e| e.message)
                        .unwrap_or_else(|| format!("HTTP {}", status.as_u16()));

                    if status == StatusCode::UNAUTHORIZED || status == StatusCode::FORBIDDEN {
                        return Err(ZVaultError::Auth(msg));
                    }
                    if status == StatusCode::NOT_FOUND {
                        return Err(ZVaultError::Api {
                            status_code: 404,
                            message: msg,
                        });
                    }

                    last_err = Some(ZVaultError::Api {
                        status_code: status.as_u16(),
                        message: msg,
                    });

                    if attempt < self.max_retries && is_retryable(status) {
                        sleep_with_jitter(attempt).await;
                        continue;
                    }
                }
                Err(e) => {
                    if e.is_timeout() {
                        last_err = Some(ZVaultError::Timeout);
                    } else {
                        last_err = Some(ZVaultError::Network(e));
                    }

                    if attempt < self.max_retries {
                        sleep_with_jitter(attempt).await;
                        continue;
                    }
                }
            }

            break;
        }

        Err(last_err.unwrap_or(ZVaultError::Api {
            status_code: 0,
            message: "unknown error".to_owned(),
        }))
    }
}

fn is_retryable(status: StatusCode) -> bool {
    matches!(
        status,
        StatusCode::TOO_MANY_REQUESTS
            | StatusCode::INTERNAL_SERVER_ERROR
            | StatusCode::BAD_GATEWAY
            | StatusCode::SERVICE_UNAVAILABLE
            | StatusCode::GATEWAY_TIMEOUT
    )
}

async fn sleep_with_jitter(attempt: u32) {
    // RETRY_BASE_DELAY is 500ms, max attempt ~3, so values stay small.
    #[allow(clippy::cast_possible_truncation)]
    let base = (RETRY_BASE_DELAY.as_millis() as u64).saturating_mul(2u64.saturating_pow(attempt));
    #[allow(clippy::cast_precision_loss)]
    let base_f = base as f64;
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let jitter = (base_f * 0.3 * rand_f64()) as u64;
    tokio::time::sleep(Duration::from_millis(base.saturating_add(jitter))).await;
}

/// Simple pseudo-random f64 in [0, 1) using system time.
fn rand_f64() -> f64 {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .subsec_nanos();
    f64::from(nanos % 1000) / 1000.0
}

fn first_non_empty(vals: &[&str]) -> String {
    for v in vals {
        if !v.is_empty() {
            return (*v).to_owned();
        }
    }
    String::new()
}

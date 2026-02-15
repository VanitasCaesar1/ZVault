//! `ZVault` server entry point.
//!
//! Bootstraps the storage backend, barrier, seal manager, and all subsystems,
//! then starts the Axum HTTP server with graceful shutdown. A background
//! lease expiry worker runs alongside the server and is cancelled on shutdown.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use anyhow::Context;
use axum::http::HeaderValue;
use axum::middleware as axum_mw;
use axum::Router;
use tokio::net::TcpListener;
use tokio::sync::{watch, RwLock};
use tracing::{info, warn};

use zvault_core::audit::AuditManager;
use zvault_core::audit_file::FileAuditBackend;
use zvault_core::approle::AppRoleStore;
use zvault_core::barrier::Barrier;
use zvault_core::database::DatabaseEngine;
use zvault_core::engine::KvEngine;
use zvault_core::lease::LeaseManager;
use zvault_core::mount::{MountEntry, MountManager};
use zvault_core::pki::PkiEngine;
use zvault_core::policy::PolicyStore;
use zvault_core::seal::SealManager;
use zvault_core::token::TokenStore;
use zvault_core::transit::TransitEngine;
use zvault_storage::MemoryBackend;

use zvault_server::config::{ServerConfig, StorageBackendType};
use zvault_server::hardening;
use zvault_server::middleware::auth_middleware;
use zvault_server::routes;
use zvault_server::state::AppState;

use tower_http::set_header::SetResponseHeaderLayer;
use tower_http::trace::TraceLayer;
use tower_http::cors::{Any, CorsLayer};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Load configuration from environment.
    let config = ServerConfig::from_env();

    // Production hardening: disable core dumps (always) and lock memory (unless disabled).
    // These run before logging is initialized, so we use eprintln for warnings.
    apply_hardening(&config);

    // Initialize structured logging.
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new(&config.log_level)),
        )
        .json()
        .init();

    info!(storage = ?config.storage_backend, "ZVault starting");

    let (state, lease_manager) = build_app_state(&config).await?;

    // Shutdown signal channel.
    let (shutdown_tx, shutdown_rx) = watch::channel(false);

    // Spawn lease expiry background worker.
    let lease_worker_handle = {
        let lm = lease_manager;
        let mut rx = shutdown_rx.clone();
        let interval_secs = config.lease_scan_interval_secs;
        tokio::spawn(async move {
            lease_expiry_worker(lm, &mut rx, interval_secs).await;
        })
    };

    let app = build_router(Arc::clone(&state));

    // Bind and serve.
    let listener = TcpListener::bind(config.bind_addr)
        .await
        .with_context(|| format!("failed to bind to {}", config.bind_addr))?;

    info!(addr = %config.bind_addr, "ZVault server listening");

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal(shutdown_tx))
        .await
        .context("server error")?;

    // Wait for background workers to finish (with timeout).
    info!("waiting for background workers to stop");
    let _ = tokio::time::timeout(Duration::from_secs(10), lease_worker_handle).await;

    info!("ZVault server stopped");
    Ok(())
}

/// Build the shared application state and return it along with the lease manager.
async fn build_app_state(
    config: &ServerConfig,
) -> anyhow::Result<(Arc<AppState>, Arc<LeaseManager>)> {

    // Bootstrap storage backend.
    let storage: Arc<dyn zvault_storage::StorageBackend> = match &config.storage_backend {
        StorageBackendType::Memory => {
            info!("using in-memory storage (data will not persist)");
            Arc::new(MemoryBackend::new())
        }
        #[cfg(feature = "rocksdb-backend")]
        StorageBackendType::RocksDb { path } => {
            info!(path = %path, "using RocksDB storage");
            Arc::new(
                zvault_storage::RocksDbBackend::open(path)
                    .context("failed to open RocksDB storage")?,
            )
        }
        #[cfg(not(feature = "rocksdb-backend"))]
        StorageBackendType::RocksDb { .. } => {
            anyhow::bail!("RocksDB backend requested but feature 'rocksdb-backend' is not enabled");
        }
        #[cfg(feature = "redb-backend")]
        StorageBackendType::Redb { path } => {
            info!(path = %path, "using redb storage");
            Arc::new(
                zvault_storage::RedbBackend::open(path)
                    .context("failed to open redb storage")?,
            )
        }
        #[cfg(not(feature = "redb-backend"))]
        StorageBackendType::Redb { .. } => {
            anyhow::bail!("redb backend requested but feature 'redb-backend' is not enabled");
        }
        #[cfg(feature = "postgres-backend")]
        StorageBackendType::Postgres { url } => {
            info!(url = %"[redacted]", "using PostgreSQL storage");
            Arc::new(
                zvault_storage::PostgresBackend::connect(url)
                    .await
                    .context("failed to connect to PostgreSQL storage")?,
            )
        }
        #[cfg(not(feature = "postgres-backend"))]
        StorageBackendType::Postgres { .. } => {
            anyhow::bail!("PostgreSQL backend requested but feature 'postgres-backend' is not enabled");
        }
    };

    // Build core subsystems.
    let barrier = Arc::new(Barrier::new(storage));
    let seal_manager = Arc::new(SealManager::new(Arc::clone(&barrier)));
    let token_store = Arc::new(TokenStore::new(Arc::clone(&barrier)));
    let policy_store = Arc::new(PolicyStore::new(Arc::clone(&barrier)));
    // Generate a random 32-byte HMAC key for audit field hashing.
    // This ensures audit HMACs are unique per server instance. In production,
    // this should be persisted through the barrier so HMACs are consistent
    // across restarts (TODO: store at sys/audit/hmac_key on first init).
    let hmac_key: Vec<u8> = {
        // Two UUID v4s = 32 bytes of OS CSPRNG randomness.
        let a = uuid::Uuid::new_v4();
        let b = uuid::Uuid::new_v4();
        let mut key = Vec::with_capacity(32);
        key.extend_from_slice(a.as_bytes());
        key.extend_from_slice(b.as_bytes());
        key
    };
    let audit_manager = Arc::new(AuditManager::new(hmac_key));
    let lease_manager = Arc::new(LeaseManager::new(Arc::clone(&barrier)));

    // Register file audit backend if configured.
    if let Some(ref audit_path) = config.audit_file_path {
        let file_backend = Arc::new(FileAuditBackend::new(audit_path));
        audit_manager.add_backend(file_backend).await;
        info!(path = %audit_path, "file audit backend registered");
    }

    // Mount manager — starts empty when sealed, reloads on unseal.
    let mount_manager = Arc::new(match MountManager::new(Arc::clone(&barrier)).await {
        Ok(mgr) => mgr,
        Err(_) => MountManager::empty(Arc::clone(&barrier)),
    });

    // Pre-register the default `secret/` KV engine.
    let default_kv = Arc::new(KvEngine::new(
        Arc::clone(&barrier),
        "kv/secret/".to_owned(),
    ));
    let mut kv_engines = HashMap::new();
    kv_engines.insert("secret/".to_owned(), default_kv);

    // Register the default KV mount (ignore error if sealed).
    let _ = mount_manager
        .mount(MountEntry {
            path: "secret/".to_owned(),
            engine_type: "kv".to_owned(),
            description: "Default KV v2 secrets engine".to_owned(),
            config: serde_json::Value::Null,
        })
        .await;

    // Pre-register the default `transit/` engine.
    let mut transit_engines = HashMap::new();
    if config.enable_transit {
        let transit = Arc::new(TransitEngine::new(
            Arc::clone(&barrier),
            "transit/transit/".to_owned(),
        ));
        transit_engines.insert("transit/".to_owned(), transit);

        let _ = mount_manager
            .mount(MountEntry {
                path: "transit/".to_owned(),
                engine_type: "transit".to_owned(),
                description: "Default transit encryption engine".to_owned(),
                config: serde_json::Value::Null,
            })
            .await;

        info!("transit engine mounted at transit/");
    }

    // Pre-register the default `database/` engine.
    let mut database_engines = HashMap::new();
    let db_engine = Arc::new(DatabaseEngine::new(
        Arc::clone(&barrier),
        "db/database/".to_owned(),
    ));
    database_engines.insert("database/".to_owned(), db_engine);

    let _ = mount_manager
        .mount(MountEntry {
            path: "database/".to_owned(),
            engine_type: "database".to_owned(),
            description: "Database dynamic credentials engine".to_owned(),
            config: serde_json::Value::Null,
        })
        .await;

    info!("database engine mounted at database/");

    // Pre-register the default `pki/` engine.
    let mut pki_engines = HashMap::new();
    let pki_engine = Arc::new(PkiEngine::new(
        Arc::clone(&barrier),
        "pki/pki/".to_owned(),
    ));
    pki_engines.insert("pki/".to_owned(), pki_engine);

    let _ = mount_manager
        .mount(MountEntry {
            path: "pki/".to_owned(),
            engine_type: "pki".to_owned(),
            description: "PKI certificate authority engine".to_owned(),
            config: serde_json::Value::Null,
        })
        .await;

    info!("PKI engine mounted at pki/");

    // Initialize AppRole auth store.
    let approle_store = Arc::new(AppRoleStore::new(
        Arc::clone(&barrier),
        "sys/approle/".to_owned(),
    ));

    info!("AppRole auth method enabled");

    let state = Arc::new(AppState {
        barrier,
        seal_manager,
        token_store,
        policy_store,
        mount_manager,
        audit_manager,
        lease_manager: Arc::clone(&lease_manager),
        kv_engines: RwLock::new(kv_engines),
        transit_engines: RwLock::new(transit_engines),
        database_engines: RwLock::new(database_engines),
        pki_engines: RwLock::new(pki_engines),
        approle_store: Some(approle_store),
        spring_oauth: config.spring_oauth.clone(),
        audit_file_path: config.audit_file_path.clone(),
    });

    Ok((state, lease_manager))
}

/// Build the Axum router with all routes and middleware.
fn build_router(state: Arc<AppState>) -> Router {
    // Authenticated routes go through the auth middleware layer.
    let authenticated_routes = Router::new()
        .nest("/v1/auth/token", routes::auth::router())
        .nest("/v1/auth/approle", routes::approle::router())
        .nest("/v1/sys/policies", routes::policy::router())
        .nest("/v1/sys/mounts", routes::mounts::router())
        .nest("/v1/sys/leases", routes::leases::router())
        .nest("/v1/secret", routes::secrets::router())
        .nest("/v1/transit", routes::transit::router())
        .nest("/v1/database", routes::database::router())
        .nest("/v1/pki", routes::pki::router())
        .route_layer(axum_mw::from_fn_with_state(
            Arc::clone(&state),
            auth_middleware,
        ));

    // Concurrency-limit the sys routes (init/unseal) to prevent resource exhaustion.
    let sys_routes = Router::new()
        .nest("/v1/sys", routes::sys::router())
        .layer(tower::limit::ConcurrencyLimitLayer::new(10));

    // CORS — restrictive defaults, allow dashboard dev server.
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods([
            axum::http::Method::GET,
            axum::http::Method::POST,
            axum::http::Method::PUT,
            axum::http::Method::DELETE,
        ])
        .allow_headers([
            axum::http::header::CONTENT_TYPE,
            axum::http::header::AUTHORIZATION,
            axum::http::HeaderName::from_static("x-vault-token"),
        ]);

    // OIDC login routes (unauthenticated — these are the login flow).
    #[cfg(feature = "spring-oauth")]
    let oidc_routes = Router::new()
        .nest("/v1/auth/oidc", routes::oidc::router());

    let mut app = Router::new()
        .merge(sys_routes)
        .nest("/v1/auth/approle", routes::approle::login_router())
        .merge(authenticated_routes);

    #[cfg(feature = "spring-oauth")]
    {
        app = app.merge(oidc_routes);
    }

    // Metrics endpoint (unauthenticated — Prometheus scrapes this).
    app = app.nest("/v1/sys/metrics", routes::metrics::router());

    app.merge(routes::ui::router())
        .merge(routes::docs::router())
        .layer(TraceLayer::new_for_http())
        .layer(cors)
        .layer(SetResponseHeaderLayer::overriding(
            axum::http::header::X_CONTENT_TYPE_OPTIONS,
            HeaderValue::from_static("nosniff"),
        ))
        .layer(SetResponseHeaderLayer::overriding(
            axum::http::header::X_FRAME_OPTIONS,
            HeaderValue::from_static("DENY"),
        ))
        .layer(SetResponseHeaderLayer::overriding(
            axum::http::header::CACHE_CONTROL,
            HeaderValue::from_static("no-store"),
        ))
        .with_state(state)
}

/// Maximum retries per tick when the storage backend is unreachable.
const LEASE_SCAN_MAX_RETRIES: u32 = 3;

/// Background worker that periodically scans for expired leases and revokes them.
///
/// If the storage backend (DB) is unreachable during cleanup, the worker retries
/// with exponential backoff (1s, 2s, 4s) before giving up on that tick. A
/// consecutive-failure counter escalates log severity so operators notice
/// persistent issues without being spammed on transient blips.
async fn lease_expiry_worker(
    lease_manager: Arc<LeaseManager>,
    shutdown: &mut watch::Receiver<bool>,
    interval_secs: u64,
) {
    let mut interval = tokio::time::interval(Duration::from_secs(interval_secs));
    let mut consecutive_failures: u32 = 0;
    info!(interval_secs, "lease expiry worker started");

    loop {
        tokio::select! {
            _ = interval.tick() => {
                let scan_result = retry_scan(&lease_manager, shutdown).await;

                match scan_result {
                    Ok(None) => {
                        // Shutdown requested during retry — exit.
                        info!("lease expiry worker shutting down");
                        return;
                    }
                    Ok(Some(expired)) if expired.is_empty() => {
                        // Reset on success.
                        consecutive_failures = 0;
                    }
                    Ok(Some(expired)) => {
                        consecutive_failures = 0;
                        let total = expired.len();
                        let mut revoked = 0u32;
                        let mut failed = 0u32;
                        for lease in &expired {
                            match lease_manager.revoke(&lease.id).await {
                                Ok(()) => { revoked = revoked.saturating_add(1); }
                                Err(e) => {
                                    failed = failed.saturating_add(1);
                                    warn!(
                                        lease_id = %lease.id,
                                        error = %e,
                                        "failed to revoke expired lease"
                                    );
                                }
                            }
                        }
                        info!(total, revoked, failed, "lease expiry tick complete");
                    }
                    Err(last_err) => {
                        consecutive_failures = consecutive_failures.saturating_add(1);
                        if consecutive_failures >= 5 {
                            tracing::error!(
                                error = %last_err,
                                consecutive_failures,
                                "lease expiry scan persistently failing — storage may be down"
                            );
                        } else {
                            warn!(
                                error = %last_err,
                                consecutive_failures,
                                retries = LEASE_SCAN_MAX_RETRIES,
                                "lease expiry scan failed after retries, will retry next tick"
                            );
                        }
                    }
                }
            }
            _ = shutdown.changed() => {
                info!("lease expiry worker shutting down");
                return;
            }
        }
    }
}

/// Attempt `find_expired()` with exponential backoff. Returns:
/// - `Ok(Some(leases))` on success
/// - `Ok(None)` if shutdown was signalled during retry
/// - `Err(last_error)` if all retries exhausted
async fn retry_scan(
    lease_manager: &Arc<LeaseManager>,
    shutdown: &mut watch::Receiver<bool>,
) -> Result<Option<Vec<zvault_core::lease::Lease>>, String> {
    let mut last_err = String::new();

    for attempt in 0..=LEASE_SCAN_MAX_RETRIES {
        match lease_manager.find_expired().await {
            Ok(expired) => return Ok(Some(expired)),
            Err(e) => {
                last_err = e.to_string();

                if attempt == LEASE_SCAN_MAX_RETRIES {
                    break;
                }

                // Exponential backoff: 1s, 2s, 4s
                let backoff = Duration::from_secs(1u64 << attempt);
                tracing::debug!(
                    attempt = attempt.saturating_add(1),
                    max = LEASE_SCAN_MAX_RETRIES.saturating_add(1),
                    backoff_ms = backoff.as_millis() as u64,
                    error = %e,
                    "lease scan failed, retrying"
                );

                // Wait for backoff OR shutdown, whichever comes first.
                tokio::select! {
                    () = tokio::time::sleep(backoff) => {}
                    _ = shutdown.changed() => {
                        return Ok(None); // Shutdown requested.
                    }
                }
            }
        }
    }

    Err(last_err)
}

/// Wait for SIGINT or SIGTERM, then broadcast shutdown.
async fn shutdown_signal(shutdown_tx: watch::Sender<bool>) {
    let ctrl_c = async {
        tokio::signal::ctrl_c().await.ok();
    };

    #[cfg(unix)]
    let terminate = async {
        if let Ok(mut sig) =
            tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
        {
            sig.recv().await;
        }
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        () = ctrl_c => {},
        () = terminate => {},
    }

    info!("shutdown signal received, stopping server");
    let _ = shutdown_tx.send(true);
}

/// Apply production hardening before logging is initialized.
///
/// Uses `eprintln` because structured logging is not yet available.
#[allow(clippy::print_stderr)]
fn apply_hardening(config: &ServerConfig) {
    if let Err(e) = hardening::disable_core_dumps() {
        eprintln!("WARNING: failed to disable core dumps: {e}");
    }

    if config.disable_mlock {
        eprintln!("WARNING: mlock disabled via ZVAULT_DISABLE_MLOCK — secrets may be swapped to disk");
    } else if let Err(e) = hardening::lock_memory() {
        eprintln!("WARNING: failed to lock memory: {e} (set ZVAULT_DISABLE_MLOCK=true for dev)");
    }
}

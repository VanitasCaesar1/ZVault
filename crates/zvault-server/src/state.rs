//! Shared application state for `ZVault` server.
//!
//! A single [`AppState`] is constructed at startup and shared across all
//! Axum handlers via `Arc`. It holds references to the barrier, seal manager,
//! token store, policy store, mount manager, audit manager, and lease manager.

use std::collections::HashMap;
use std::sync::Arc;

use tokio::sync::RwLock;

use zvault_core::approle::AppRoleStore;
use zvault_core::audit::AuditManager;
use zvault_core::barrier::Barrier;
use zvault_core::database::DatabaseEngine;
use zvault_core::engine::KvEngine;
use zvault_core::lease::LeaseManager;
use zvault_core::mount::MountManager;
use zvault_core::pki::PkiEngine;
use zvault_core::policy::PolicyStore;
use zvault_core::seal::SealManager;
use zvault_core::token::TokenStore;
use zvault_core::transit::TransitEngine;

use crate::config::SpringOAuthConfig;

/// Shared application state passed to all HTTP handlers.
pub struct AppState {
    /// The encryption barrier.
    pub barrier: Arc<Barrier>,
    /// Seal/unseal lifecycle manager.
    pub seal_manager: Arc<SealManager>,
    /// Token creation, lookup, and revocation.
    pub token_store: Arc<TokenStore>,
    /// Policy CRUD and evaluation.
    pub policy_store: Arc<PolicyStore>,
    /// Engine mount table.
    pub mount_manager: Arc<MountManager>,
    /// Audit log manager.
    pub audit_manager: Arc<AuditManager>,
    /// Lease lifecycle manager.
    pub lease_manager: Arc<LeaseManager>,
    /// Registered KV engines keyed by mount path.
    pub kv_engines: RwLock<HashMap<String, Arc<KvEngine>>>,
    /// Registered transit engines keyed by mount path.
    pub transit_engines: RwLock<HashMap<String, Arc<TransitEngine>>>,
    /// Registered database engines keyed by mount path.
    pub database_engines: RwLock<HashMap<String, Arc<DatabaseEngine>>>,
    /// Registered PKI engines keyed by mount path.
    pub pki_engines: RwLock<HashMap<String, Arc<PkiEngine>>>,
    /// AppRole auth store (None if not enabled).
    pub approle_store: Option<Arc<AppRoleStore>>,
    /// Spring OAuth configuration (None if not configured).
    pub spring_oauth: Option<SpringOAuthConfig>,
    /// Path to the audit log file (for reading audit entries via API).
    pub audit_file_path: Option<String>,
}

impl std::fmt::Debug for AppState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AppState").finish_non_exhaustive()
    }
}

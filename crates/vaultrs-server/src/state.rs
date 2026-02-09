//! Shared application state for `VaultRS` server.
//!
//! A single [`AppState`] is constructed at startup and shared across all
//! Axum handlers via `Arc`. It holds references to the barrier, seal manager,
//! token store, policy store, mount manager, audit manager, and lease manager.

use std::collections::HashMap;
use std::sync::Arc;

use tokio::sync::RwLock;

use vaultrs_core::audit::AuditManager;
use vaultrs_core::barrier::Barrier;
use vaultrs_core::engine::KvEngine;
use vaultrs_core::lease::LeaseManager;
use vaultrs_core::mount::MountManager;
use vaultrs_core::policy::PolicyStore;
use vaultrs_core::seal::SealManager;
use vaultrs_core::token::TokenStore;
use vaultrs_core::transit::TransitEngine;

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
}

impl std::fmt::Debug for AppState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AppState").finish_non_exhaustive()
    }
}

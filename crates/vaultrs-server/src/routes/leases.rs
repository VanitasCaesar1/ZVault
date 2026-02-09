//! Lease management routes: `/v1/sys/leases/*`
//!
//! Lookup, renew, and revoke leases for dynamic secrets.

use std::sync::Arc;

use axum::extract::State;
use axum::http::StatusCode;
use axum::routing::post;
use axum::{Extension, Json, Router};
use serde::{Deserialize, Serialize};

use crate::error::AppError;
use crate::middleware::AuthContext;
use crate::state::AppState;
use vaultrs_core::policy::Capability;

/// Build the `/v1/sys/leases` router.
pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/lookup", post(lookup_lease))
        .route("/renew", post(renew_lease))
        .route("/revoke", post(revoke_lease))
}

// ── Request / Response types ─────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct LeaseLookupRequest {
    pub lease_id: String,
}

#[derive(Debug, Serialize)]
pub struct LeaseResponse {
    pub lease_id: String,
    pub engine_path: String,
    pub issued_at: String,
    pub ttl_secs: i64,
    pub renewable: bool,
    pub expired: bool,
}

#[derive(Debug, Deserialize)]
pub struct LeaseRenewRequest {
    pub lease_id: String,
    pub increment: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub struct LeaseRevokeRequest {
    pub lease_id: String,
}

// ── Handlers ─────────────────────────────────────────────────────────

/// Look up a lease by ID.
async fn lookup_lease(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthContext>,
    Json(body): Json<LeaseLookupRequest>,
) -> Result<Json<LeaseResponse>, AppError> {
    state
        .policy_store
        .check(&auth.policies, "sys/leases/lookup", &Capability::Read)
        .await?;

    let lease = state.lease_manager.lookup(&body.lease_id).await?;
    let expired = lease.is_expired();

    Ok(Json(LeaseResponse {
        lease_id: lease.id,
        engine_path: lease.engine_path,
        issued_at: lease.issued_at.to_rfc3339(),
        ttl_secs: lease.ttl_secs,
        renewable: lease.renewable,
        expired,
    }))
}

/// Renew a lease.
async fn renew_lease(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthContext>,
    Json(body): Json<LeaseRenewRequest>,
) -> Result<Json<LeaseResponse>, AppError> {
    state
        .policy_store
        .check(&auth.policies, "sys/leases/renew", &Capability::Update)
        .await?;

    let increment = body.increment.unwrap_or(3600);
    let lease = state
        .lease_manager
        .renew(&body.lease_id, increment)
        .await?;
    let expired = lease.is_expired();

    Ok(Json(LeaseResponse {
        lease_id: lease.id,
        engine_path: lease.engine_path,
        issued_at: lease.issued_at.to_rfc3339(),
        ttl_secs: lease.ttl_secs,
        renewable: lease.renewable,
        expired,
    }))
}

/// Revoke a lease immediately.
async fn revoke_lease(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthContext>,
    Json(body): Json<LeaseRevokeRequest>,
) -> Result<StatusCode, AppError> {
    state
        .policy_store
        .check(&auth.policies, "sys/leases/revoke", &Capability::Sudo)
        .await?;

    state.lease_manager.revoke(&body.lease_id).await?;

    Ok(StatusCode::NO_CONTENT)
}

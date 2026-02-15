//! Engine mount management routes: `/v1/sys/mounts/*`
//!
//! Mount, unmount, and list secrets engines.

use std::sync::Arc;

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::routing::{delete, get, post};
use axum::{Extension, Json, Router};
use serde::{Deserialize, Serialize};

use crate::error::AppError;
use crate::middleware::AuthContext;
use crate::state::AppState;
use zvault_core::engine::KvEngine;
use zvault_core::mount::MountEntry;
use zvault_core::policy::Capability;

/// Build the `/v1/sys/mounts` router.
pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/", get(list_mounts))
        .route("/{path}", post(mount_engine))
        .route("/{path}", delete(unmount_engine))
}

// ── Request / Response types ─────────────────────────────────────────

#[derive(Debug, Serialize)]
pub struct MountListResponse {
    pub mounts: Vec<MountEntryResponse>,
}

#[derive(Debug, Serialize)]
pub struct MountEntryResponse {
    pub path: String,
    pub engine_type: String,
    pub description: String,
}

#[derive(Debug, Deserialize)]
pub struct MountRequest {
    pub engine_type: String,
    pub description: Option<String>,
    pub config: Option<serde_json::Value>,
}

// ── Handlers ─────────────────────────────────────────────────────────

/// List all mounted engines.
async fn list_mounts(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthContext>,
) -> Result<Json<MountListResponse>, AppError> {
    state
        .policy_store
        .check(&auth.policies, "sys/mounts", &Capability::List)
        .await?;

    let entries = state.mount_manager.list().await;

    let mounts = entries
        .into_iter()
        .map(|e| MountEntryResponse {
            path: e.path,
            engine_type: e.engine_type,
            description: e.description,
        })
        .collect();

    Ok(Json(MountListResponse { mounts }))
}

/// Mount a new secrets engine.
async fn mount_engine(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthContext>,
    Path(path): Path<String>,
    Json(body): Json<MountRequest>,
) -> Result<StatusCode, AppError> {
    state
        .policy_store
        .check(&auth.policies, "sys/mounts", &Capability::Create)
        .await?;

    // Validate engine type.
    if body.engine_type != "kv" {
        return Err(AppError::BadRequest(format!(
            "unsupported engine type '{}', only 'kv' is supported",
            body.engine_type
        )));
    }

    let mount_path = if path.ends_with('/') {
        path.clone()
    } else {
        format!("{path}/")
    };

    let entry = MountEntry {
        path: mount_path.clone(),
        engine_type: body.engine_type,
        description: body.description.unwrap_or_default(),
        config: body.config.unwrap_or(serde_json::Value::Null),
    };

    state.mount_manager.mount(entry).await?;

    // Create and register the KV engine instance.
    let engine = Arc::new(KvEngine::new(
        Arc::clone(&state.barrier),
        format!("kv/{mount_path}"),
    ));
    state.kv_engines.write().await.insert(mount_path, engine);

    Ok(StatusCode::NO_CONTENT)
}

/// Unmount a secrets engine.
async fn unmount_engine(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthContext>,
    Path(path): Path<String>,
) -> Result<StatusCode, AppError> {
    state
        .policy_store
        .check(&auth.policies, "sys/mounts", &Capability::Delete)
        .await?;

    let mount_path = if path.ends_with('/') {
        path.clone()
    } else {
        format!("{path}/")
    };

    state.mount_manager.unmount(&mount_path).await?;

    // Remove the KV engine instance.
    state.kv_engines.write().await.remove(&mount_path);

    // Revoke all leases for this mount.
    let _ = state.lease_manager.revoke_prefix(&mount_path).await;

    Ok(StatusCode::NO_CONTENT)
}

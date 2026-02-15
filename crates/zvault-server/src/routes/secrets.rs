//! Secrets routes: `/v1/{mount_path}/*`
//!
//! Routes requests to the appropriate KV engine based on the mount table.
//! Supports read, write, delete, list, and metadata operations.

use std::sync::Arc;

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::routing::get;
use axum::{Extension, Json, Router};
use serde::Serialize;

use crate::error::AppError;
use crate::middleware::AuthContext;
use crate::state::AppState;
use zvault_core::engine::{EngineRequest, Operation};
use zvault_core::policy::Capability;

/// Validate a secret path against security rules.
///
/// - Only alphanumeric, `_`, `-`, `/` characters allowed.
/// - No `..` path traversal.
/// - No null bytes.
/// - Maximum 10 path segments.
/// - Path must not be empty.
fn validate_secret_path(path: &str) -> Result<(), AppError> {
    if path.is_empty() {
        return Err(AppError::BadRequest(
            "secret path must not be empty".to_owned(),
        ));
    }

    if path.contains("..") {
        return Err(AppError::BadRequest(
            "path traversal (..) is not allowed".to_owned(),
        ));
    }

    if path.contains('\0') {
        return Err(AppError::BadRequest(
            "null bytes are not allowed in paths".to_owned(),
        ));
    }

    // Only allow safe characters.
    if !path
        .bytes()
        .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-' || b == b'/')
    {
        return Err(AppError::BadRequest(
            "secret path may only contain alphanumeric characters, '_', '-', and '/'".to_owned(),
        ));
    }

    let segment_count = path.split('/').filter(|s| !s.is_empty()).count();
    if segment_count > 10 {
        return Err(AppError::BadRequest(
            "secret path exceeds maximum depth of 10 segments".to_owned(),
        ));
    }

    Ok(())
}

/// Build the `/v1/secret` router for the default KV mount.
///
/// Paths:
/// - `GET    /v1/secret/data/{*path}` — read
/// - `POST   /v1/secret/data/{*path}` — write
/// - `DELETE  /v1/secret/data/{*path}` — delete
/// - `GET    /v1/secret/metadata/{*path}` — metadata
/// - `GET    /v1/secret/list/{*path}` — list keys
pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route(
            "/data/{*path}",
            get(read_secret).post(write_secret).delete(delete_secret),
        )
        .route("/metadata/{*path}", get(get_metadata))
        .route("/list/{*path}", get(list_secrets))
}

// ── Response types ───────────────────────────────────────────────────

#[derive(Debug, Serialize)]
pub struct SecretResponse {
    pub data: Option<serde_json::Value>,
    pub lease_id: Option<String>,
    pub lease_duration: Option<i64>,
    pub renewable: bool,
}

#[derive(Debug, Serialize)]
pub struct MetadataResponse {
    pub current_version: u32,
    pub created_at: String,
    pub updated_at: String,
    pub version_count: u32,
    pub max_versions: u32,
}

#[derive(Debug, Serialize)]
pub struct ListResponse {
    pub keys: Vec<String>,
}

// ── Handlers ─────────────────────────────────────────────────────────

/// Read a secret from the KV engine.
async fn read_secret(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthContext>,
    Path(path): Path<String>,
) -> Result<Json<SecretResponse>, AppError> {
    validate_secret_path(&path)?;
    let mount_path = resolve_mount(&path);

    state
        .policy_store
        .check(
            &auth.policies,
            &format!("{mount_path}data/{path}"),
            &Capability::Read,
        )
        .await?;

    let engine = get_engine(&state, &mount_path).await?;

    let response = engine
        .handle(&EngineRequest {
            operation: Operation::Read,
            path: path.clone(),
            data: None,
        })
        .await?;

    Ok(Json(SecretResponse {
        data: response.data,
        lease_id: response.lease_id,
        lease_duration: response.lease_duration,
        renewable: response.renewable,
    }))
}

/// Write a secret to the KV engine.
async fn write_secret(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthContext>,
    Path(path): Path<String>,
    Json(body): Json<serde_json::Value>,
) -> Result<(StatusCode, Json<SecretResponse>), AppError> {
    validate_secret_path(&path)?;
    let mount_path = resolve_mount(&path);

    state
        .policy_store
        .check(
            &auth.policies,
            &format!("{mount_path}data/{path}"),
            &Capability::Create,
        )
        .await?;

    let engine = get_engine(&state, &mount_path).await?;

    let response = engine
        .handle(&EngineRequest {
            operation: Operation::Write,
            path: path.clone(),
            data: Some(body),
        })
        .await?;

    Ok((
        StatusCode::OK,
        Json(SecretResponse {
            data: response.data,
            lease_id: response.lease_id,
            lease_duration: response.lease_duration,
            renewable: response.renewable,
        }),
    ))
}

/// Delete a secret from the KV engine (soft delete).
async fn delete_secret(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthContext>,
    Path(path): Path<String>,
) -> Result<StatusCode, AppError> {
    validate_secret_path(&path)?;
    let mount_path = resolve_mount(&path);

    state
        .policy_store
        .check(
            &auth.policies,
            &format!("{mount_path}data/{path}"),
            &Capability::Delete,
        )
        .await?;

    let engine = get_engine(&state, &mount_path).await?;

    engine
        .handle(&EngineRequest {
            operation: Operation::Delete,
            path: path.clone(),
            data: None,
        })
        .await?;

    Ok(StatusCode::NO_CONTENT)
}

/// Get metadata about a secret.
async fn get_metadata(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthContext>,
    Path(path): Path<String>,
) -> Result<Json<MetadataResponse>, AppError> {
    validate_secret_path(&path)?;
    let mount_path = resolve_mount(&path);

    state
        .policy_store
        .check(
            &auth.policies,
            &format!("{mount_path}metadata/{path}"),
            &Capability::Read,
        )
        .await?;

    let engine = get_engine(&state, &mount_path).await?;

    let meta = engine.metadata(&path).await?;

    Ok(Json(MetadataResponse {
        current_version: meta.current_version,
        created_at: meta.created_at.to_rfc3339(),
        updated_at: meta.updated_at.to_rfc3339(),
        version_count: meta.version_count,
        max_versions: meta.max_versions,
    }))
}

/// List secret keys under a prefix.
async fn list_secrets(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthContext>,
    Path(path): Path<String>,
) -> Result<Json<SecretResponse>, AppError> {
    validate_secret_path(&path)?;
    let mount_path = resolve_mount(&path);

    state
        .policy_store
        .check(
            &auth.policies,
            &format!("{mount_path}list/{path}"),
            &Capability::List,
        )
        .await?;

    let engine = get_engine(&state, &mount_path).await?;

    let response = engine
        .handle(&EngineRequest {
            operation: Operation::List,
            path: path.clone(),
            data: None,
        })
        .await?;

    Ok(Json(SecretResponse {
        data: response.data,
        lease_id: None,
        lease_duration: None,
        renewable: false,
    }))
}

// ── Helpers ──────────────────────────────────────────────────────────

/// Resolve the mount path for a given secret path.
///
/// For now, all secrets go through the default `secret/` mount.
/// A full implementation would use the mount table to resolve dynamically.
fn resolve_mount(_path: &str) -> String {
    "secret/".to_owned()
}

/// Get the KV engine for a mount path.
async fn get_engine(
    state: &AppState,
    mount_path: &str,
) -> Result<Arc<zvault_core::engine::KvEngine>, AppError> {
    state
        .kv_engines
        .read()
        .await
        .get(mount_path)
        .cloned()
        .ok_or_else(|| AppError::NotFound(format!("no engine mounted at '{mount_path}'")))
}

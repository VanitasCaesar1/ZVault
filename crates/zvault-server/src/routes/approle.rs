//! HTTP route handlers for the `AppRole` auth method.
//!
//! Endpoints:
//! - `POST /v1/auth/approle/role/:name` — create a role
//! - `GET  /v1/auth/approle/role/:name` — read a role
//! - `DELETE /v1/auth/approle/role/:name` — delete a role
//! - `GET  /v1/auth/approle/role` — list all roles
//! - `GET  /v1/auth/approle/role/:name/role-id` — get role ID
//! - `POST /v1/auth/approle/role/:name/secret-id` — generate secret ID
//! - `POST /v1/auth/approle/login` — login with `role_id` + `secret_id`

use std::sync::Arc;

use axum::extract::{Path, State};
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::Deserialize;

use zvault_core::approle::AppRole;

use crate::error::AppError;
use crate::state::AppState;

/// Build the `AppRole` auth router (authenticated — role management).
pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/role", get(list_roles))
        .route(
            "/role/{name}",
            post(create_role).get(get_role).delete(delete_role),
        )
        .route("/role/{name}/role-id", get(get_role_id))
        .route("/role/{name}/secret-id", post(generate_secret_id))
}

/// Build the public `AppRole` login router (no auth required).
pub fn login_router() -> Router<Arc<AppState>> {
    Router::new().route("/login", post(login))
}

#[derive(Deserialize)]
struct CreateRoleRequest {
    policies: Vec<String>,
    #[serde(default = "default_ttl")]
    token_ttl_secs: i64,
    #[serde(default = "default_max_ttl")]
    token_max_ttl_secs: i64,
    #[serde(default = "default_true")]
    bind_secret_id: bool,
    #[serde(default)]
    secret_id_num_uses: u32,
    #[serde(default)]
    secret_id_ttl_secs: i64,
}

fn default_ttl() -> i64 {
    3600
}
fn default_max_ttl() -> i64 {
    86400
}
fn default_true() -> bool {
    true
}

async fn create_role(
    State(state): State<Arc<AppState>>,
    Path(name): Path<String>,
    Json(body): Json<CreateRoleRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    let store = state
        .approle_store
        .as_ref()
        .ok_or_else(|| AppError::NotFound("AppRole auth not enabled".to_owned()))?;
    let role = store
        .create_role(AppRole {
            name,
            role_id: String::new(), // Will be generated.
            policies: body.policies,
            token_ttl_secs: body.token_ttl_secs,
            token_max_ttl_secs: body.token_max_ttl_secs,
            bind_secret_id: body.bind_secret_id,
            secret_id_num_uses: body.secret_id_num_uses,
            secret_id_ttl_secs: body.secret_id_ttl_secs,
        })
        .await
        .map_err(AppError::from)?;
    Ok(Json(serde_json::json!({
        "role_id": role.role_id,
        "status": "ok",
    })))
}

async fn get_role(
    State(state): State<Arc<AppState>>,
    Path(name): Path<String>,
) -> Result<Json<serde_json::Value>, AppError> {
    let store = state
        .approle_store
        .as_ref()
        .ok_or_else(|| AppError::NotFound("AppRole auth not enabled".to_owned()))?;
    let role = store.get_role(&name).await.map_err(AppError::from)?;
    Ok(Json(serde_json::json!({
        "name": role.name,
        "role_id": role.role_id,
        "policies": role.policies,
        "token_ttl_secs": role.token_ttl_secs,
        "token_max_ttl_secs": role.token_max_ttl_secs,
        "bind_secret_id": role.bind_secret_id,
        "secret_id_num_uses": role.secret_id_num_uses,
    })))
}

async fn delete_role(
    State(state): State<Arc<AppState>>,
    Path(name): Path<String>,
) -> Result<Json<serde_json::Value>, AppError> {
    let store = state
        .approle_store
        .as_ref()
        .ok_or_else(|| AppError::NotFound("AppRole auth not enabled".to_owned()))?;
    store.delete_role(&name).await.map_err(AppError::from)?;
    Ok(Json(serde_json::json!({"status": "deleted"})))
}

async fn list_roles(
    State(state): State<Arc<AppState>>,
) -> Result<Json<serde_json::Value>, AppError> {
    let store = state
        .approle_store
        .as_ref()
        .ok_or_else(|| AppError::NotFound("AppRole auth not enabled".to_owned()))?;
    let names = store.list_roles().await.map_err(AppError::from)?;
    Ok(Json(serde_json::json!({"keys": names})))
}

async fn get_role_id(
    State(state): State<Arc<AppState>>,
    Path(name): Path<String>,
) -> Result<Json<serde_json::Value>, AppError> {
    let store = state
        .approle_store
        .as_ref()
        .ok_or_else(|| AppError::NotFound("AppRole auth not enabled".to_owned()))?;
    let role_id = store.get_role_id(&name).await.map_err(AppError::from)?;
    Ok(Json(serde_json::json!({"role_id": role_id})))
}

async fn generate_secret_id(
    State(state): State<Arc<AppState>>,
    Path(name): Path<String>,
) -> Result<Json<serde_json::Value>, AppError> {
    let store = state
        .approle_store
        .as_ref()
        .ok_or_else(|| AppError::NotFound("AppRole auth not enabled".to_owned()))?;
    let secret_id = store
        .generate_secret_id(&name)
        .await
        .map_err(AppError::from)?;
    Ok(Json(serde_json::json!({"secret_id": secret_id})))
}

#[derive(Deserialize)]
struct LoginRequest {
    role_id: String,
    secret_id: String,
}

async fn login(
    State(state): State<Arc<AppState>>,
    Json(body): Json<LoginRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    let store = state
        .approle_store
        .as_ref()
        .ok_or_else(|| AppError::NotFound("AppRole auth not enabled".to_owned()))?;
    let (plaintext_token, token_entry) = store
        .login(&body.role_id, &body.secret_id, &state.token_store)
        .await
        .map_err(AppError::from)?;

    let ttl_secs = token_entry
        .expires_at
        .map_or(0, |exp| (exp - chrono::Utc::now()).num_seconds().max(0));

    Ok(Json(serde_json::json!({
        "client_token": plaintext_token,
        "token_hash": token_entry.token_hash,
        "policies": token_entry.policies,
        "ttl": ttl_secs,
        "renewable": token_entry.renewable,
    })))
}

//! HTTP route handlers for the database secrets engine.
//!
//! Endpoints:
//! - `POST /v1/database/config/:name` — configure a database connection
//! - `GET  /v1/database/config/:name` — read a database config
//! - `DELETE /v1/database/config/:name` — delete a database config
//! - `GET  /v1/database/config` — list all configs
//! - `POST /v1/database/roles/:name` — create a role
//! - `GET  /v1/database/roles/:name` — read a role
//! - `DELETE /v1/database/roles/:name` — delete a role
//! - `GET  /v1/database/roles` — list all roles
//! - `GET  /v1/database/creds/:name` — generate credentials

use std::sync::Arc;

use axum::extract::{Path, State};
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::Deserialize;

use vaultrs_core::database::{DatabaseConfig, DatabaseRole};

use crate::error::AppError;
use crate::state::AppState;

/// Build the database engine router.
pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/config", get(list_configs))
        .route("/config/{name}", post(configure).get(get_config).delete(delete_config))
        .route("/roles", get(list_roles))
        .route("/roles/{name}", post(create_role).get(get_role).delete(delete_role))
        .route("/creds/{name}", get(generate_creds))
}

#[derive(Deserialize)]
struct ConfigureRequest {
    plugin: String,
    connection_url: String,
    #[serde(default = "default_max_conn")]
    max_open_connections: u32,
    #[serde(default)]
    allowed_roles: Vec<String>,
}

fn default_max_conn() -> u32 { 4 }

async fn configure(
    State(state): State<Arc<AppState>>,
    Path(name): Path<String>,
    Json(body): Json<ConfigureRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    let engines = state.database_engines.read().await;
    let engine = engines.get("database/").ok_or_else(|| {
        AppError::NotFound("database engine not mounted".to_owned())
    })?;
    engine
        .configure(DatabaseConfig {
            name,
            plugin: body.plugin,
            connection_url: body.connection_url,
            max_open_connections: body.max_open_connections,
            allowed_roles: body.allowed_roles,
        })
        .await
        .map_err(AppError::from)?;
    Ok(Json(serde_json::json!({"status": "ok"})))
}

async fn get_config(
    State(state): State<Arc<AppState>>,
    Path(name): Path<String>,
) -> Result<Json<serde_json::Value>, AppError> {
    let engines = state.database_engines.read().await;
    let engine = engines.get("database/").ok_or_else(|| {
        AppError::NotFound("database engine not mounted".to_owned())
    })?;
    let config = engine.get_config(&name).await.map_err(AppError::from)?;
    // Redact connection_url in response.
    Ok(Json(serde_json::json!({
        "name": config.name,
        "plugin": config.plugin,
        "connection_url": "***",
        "max_open_connections": config.max_open_connections,
        "allowed_roles": config.allowed_roles,
    })))
}

async fn delete_config(
    State(state): State<Arc<AppState>>,
    Path(name): Path<String>,
) -> Result<Json<serde_json::Value>, AppError> {
    let engines = state.database_engines.read().await;
    let engine = engines.get("database/").ok_or_else(|| {
        AppError::NotFound("database engine not mounted".to_owned())
    })?;
    engine.delete_config(&name).await.map_err(AppError::from)?;
    Ok(Json(serde_json::json!({"status": "deleted"})))
}

async fn list_configs(
    State(state): State<Arc<AppState>>,
) -> Result<Json<serde_json::Value>, AppError> {
    let engines = state.database_engines.read().await;
    let engine = engines.get("database/").ok_or_else(|| {
        AppError::NotFound("database engine not mounted".to_owned())
    })?;
    let names = engine.list_configs().await.map_err(AppError::from)?;
    Ok(Json(serde_json::json!({"keys": names})))
}

#[derive(Deserialize)]
struct CreateRoleRequest {
    db_name: String,
    creation_statements: Vec<String>,
    #[serde(default)]
    revocation_statements: Vec<String>,
    #[serde(default = "default_ttl")]
    default_ttl_secs: i64,
    #[serde(default = "default_max_ttl")]
    max_ttl_secs: i64,
}

fn default_ttl() -> i64 { 3600 }
fn default_max_ttl() -> i64 { 86400 }

async fn create_role(
    State(state): State<Arc<AppState>>,
    Path(name): Path<String>,
    Json(body): Json<CreateRoleRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    let engines = state.database_engines.read().await;
    let engine = engines.get("database/").ok_or_else(|| {
        AppError::NotFound("database engine not mounted".to_owned())
    })?;
    engine
        .create_role(DatabaseRole {
            name,
            db_name: body.db_name,
            creation_statements: body.creation_statements,
            revocation_statements: body.revocation_statements,
            default_ttl_secs: body.default_ttl_secs,
            max_ttl_secs: body.max_ttl_secs,
        })
        .await
        .map_err(AppError::from)?;
    Ok(Json(serde_json::json!({"status": "ok"})))
}

async fn get_role(
    State(state): State<Arc<AppState>>,
    Path(name): Path<String>,
) -> Result<Json<serde_json::Value>, AppError> {
    let engines = state.database_engines.read().await;
    let engine = engines.get("database/").ok_or_else(|| {
        AppError::NotFound("database engine not mounted".to_owned())
    })?;
    let role = engine.get_role(&name).await.map_err(AppError::from)?;
    Ok(Json(serde_json::to_value(role).unwrap_or_default()))
}

async fn delete_role(
    State(state): State<Arc<AppState>>,
    Path(name): Path<String>,
) -> Result<Json<serde_json::Value>, AppError> {
    let engines = state.database_engines.read().await;
    let engine = engines.get("database/").ok_or_else(|| {
        AppError::NotFound("database engine not mounted".to_owned())
    })?;
    engine.delete_role(&name).await.map_err(AppError::from)?;
    Ok(Json(serde_json::json!({"status": "deleted"})))
}

async fn list_roles(
    State(state): State<Arc<AppState>>,
) -> Result<Json<serde_json::Value>, AppError> {
    let engines = state.database_engines.read().await;
    let engine = engines.get("database/").ok_or_else(|| {
        AppError::NotFound("database engine not mounted".to_owned())
    })?;
    let names = engine.list_roles().await.map_err(AppError::from)?;
    Ok(Json(serde_json::json!({"keys": names})))
}

async fn generate_creds(
    State(state): State<Arc<AppState>>,
    Path(name): Path<String>,
) -> Result<Json<serde_json::Value>, AppError> {
    let engines = state.database_engines.read().await;
    let engine = engines.get("database/").ok_or_else(|| {
        AppError::NotFound("database engine not mounted".to_owned())
    })?;
    let (creds, role) = engine
        .generate_credentials(&name)
        .await
        .map_err(AppError::from)?;

    // Create a lease for the credentials.
    let lease = vaultrs_core::lease::Lease {
        id: uuid::Uuid::new_v4().to_string(),
        engine_path: format!("database/creds/{name}"),
        issued_at: chrono::Utc::now(),
        ttl_secs: role.default_ttl_secs,
        renewable: true,
        data: serde_json::json!({"username": creds.username}),
        token_hash: String::new(),
    };
    let lease_id = state
        .lease_manager
        .create(&lease)
        .await
        .map_err(AppError::from)?;

    Ok(Json(serde_json::json!({
        "username": creds.username,
        "password": creds.password,
        "lease_id": lease_id,
        "lease_duration": role.default_ttl_secs,
        "renewable": true,
    })))
}

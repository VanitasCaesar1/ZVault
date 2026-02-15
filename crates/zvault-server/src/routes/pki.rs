//! HTTP route handlers for the PKI secrets engine.
//!
//! Endpoints:
//! - `POST /v1/pki/root/generate` — generate a self-signed root CA
//! - `GET  /v1/pki/ca` — get the CA certificate
//! - `POST /v1/pki/roles/:name` — create a PKI role
//! - `GET  /v1/pki/roles/:name` — read a PKI role
//! - `GET  /v1/pki/roles` — list all roles
//! - `POST /v1/pki/issue/:role` — issue a certificate
//! - `GET  /v1/pki/certs` — list issued certificates

use std::sync::Arc;

use axum::extract::{Path, State};
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::Deserialize;

use zvault_core::pki::PkiRole;

use crate::error::AppError;
use crate::state::AppState;

/// Build the PKI engine router.
pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/root/generate", post(generate_root))
        .route("/ca", get(get_ca))
        .route("/roles", get(list_roles))
        .route("/roles/{name}", post(create_role).get(get_role))
        .route("/issue/{role}", post(issue_cert))
        .route("/certs", get(list_certs))
}

#[derive(Deserialize)]
struct GenerateRootRequest {
    common_name: String,
    #[serde(default = "default_ca_ttl")]
    ttl_hours: u64,
}

fn default_ca_ttl() -> u64 { 87600 } // 10 years

async fn generate_root(
    State(state): State<Arc<AppState>>,
    Json(body): Json<GenerateRootRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    let engines = state.pki_engines.read().await;
    let engine = engines.get("pki/").ok_or_else(|| {
        AppError::NotFound("PKI engine not mounted".to_owned())
    })?;
    let ca = engine
        .generate_root(&body.common_name, body.ttl_hours)
        .await
        .map_err(AppError::from)?;
    Ok(Json(serde_json::json!({
        "certificate": ca.certificate_pem,
        "common_name": ca.common_name,
        "ttl_hours": ca.ttl_hours,
    })))
}

async fn get_ca(
    State(state): State<Arc<AppState>>,
) -> Result<Json<serde_json::Value>, AppError> {
    let engines = state.pki_engines.read().await;
    let engine = engines.get("pki/").ok_or_else(|| {
        AppError::NotFound("PKI engine not mounted".to_owned())
    })?;
    let ca = engine.get_ca().await.map_err(AppError::from)?;
    Ok(Json(serde_json::json!({
        "certificate": ca.certificate_pem,
        "common_name": ca.common_name,
        "ttl_hours": ca.ttl_hours,
    })))
}

#[derive(Deserialize)]
struct CreatePkiRoleRequest {
    allowed_domains: Vec<String>,
    #[serde(default)]
    allow_subdomains: bool,
    #[serde(default = "default_role_ttl")]
    max_ttl_hours: u64,
    #[serde(default = "default_true")]
    generate_key: bool,
    #[serde(default = "default_key_type")]
    key_type: String,
    #[serde(default = "default_key_bits")]
    key_bits: u32,
}

fn default_role_ttl() -> u64 { 720 } // 30 days
fn default_true() -> bool { true }
fn default_key_type() -> String { "ec".to_owned() }
fn default_key_bits() -> u32 { 256 }

async fn create_role(
    State(state): State<Arc<AppState>>,
    Path(name): Path<String>,
    Json(body): Json<CreatePkiRoleRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    let engines = state.pki_engines.read().await;
    let engine = engines.get("pki/").ok_or_else(|| {
        AppError::NotFound("PKI engine not mounted".to_owned())
    })?;
    engine
        .create_role(PkiRole {
            name,
            allowed_domains: body.allowed_domains,
            allow_subdomains: body.allow_subdomains,
            max_ttl_hours: body.max_ttl_hours,
            generate_key: body.generate_key,
            key_type: body.key_type,
            key_bits: body.key_bits,
        })
        .await
        .map_err(AppError::from)?;
    Ok(Json(serde_json::json!({"status": "ok"})))
}

async fn get_role(
    State(state): State<Arc<AppState>>,
    Path(name): Path<String>,
) -> Result<Json<serde_json::Value>, AppError> {
    let engines = state.pki_engines.read().await;
    let engine = engines.get("pki/").ok_or_else(|| {
        AppError::NotFound("PKI engine not mounted".to_owned())
    })?;
    let role = engine.get_role(&name).await.map_err(AppError::from)?;
    Ok(Json(serde_json::to_value(role).unwrap_or_default()))
}

async fn list_roles(
    State(state): State<Arc<AppState>>,
) -> Result<Json<serde_json::Value>, AppError> {
    let engines = state.pki_engines.read().await;
    let engine = engines.get("pki/").ok_or_else(|| {
        AppError::NotFound("PKI engine not mounted".to_owned())
    })?;
    let names = engine.list_roles().await.map_err(AppError::from)?;
    Ok(Json(serde_json::json!({"keys": names})))
}

#[derive(Deserialize)]
struct IssueCertRequest {
    common_name: String,
    ttl_hours: Option<u64>,
}

async fn issue_cert(
    State(state): State<Arc<AppState>>,
    Path(role): Path<String>,
    Json(body): Json<IssueCertRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    let engines = state.pki_engines.read().await;
    let engine = engines.get("pki/").ok_or_else(|| {
        AppError::NotFound("PKI engine not mounted".to_owned())
    })?;
    let cert = engine
        .issue(&role, &body.common_name, body.ttl_hours)
        .await
        .map_err(AppError::from)?;
    Ok(Json(serde_json::json!({
        "certificate": cert.certificate_pem,
        "private_key": cert.private_key_pem,
        "ca_chain": cert.ca_chain_pem,
        "serial_number": cert.serial_number,
        "expiration": cert.expiration,
    })))
}

async fn list_certs(
    State(state): State<Arc<AppState>>,
) -> Result<Json<serde_json::Value>, AppError> {
    let engines = state.pki_engines.read().await;
    let engine = engines.get("pki/").ok_or_else(|| {
        AppError::NotFound("PKI engine not mounted".to_owned())
    })?;
    let serials = engine.list_certs().await.map_err(AppError::from)?;
    Ok(Json(serde_json::json!({"keys": serials})))
}

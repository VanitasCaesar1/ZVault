//! Transit secrets engine routes: `/v1/transit/*`
//!
//! Encryption-as-a-service: create named keys, encrypt/decrypt data,
//! rotate keys, rewrap ciphertext, and generate data encryption keys.

use std::sync::Arc;

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::routing::{get, post};
use axum::{Extension, Json, Router};
use base64::Engine as _;
use base64::engine::general_purpose::STANDARD as BASE64;
use serde::{Deserialize, Serialize};

use crate::error::AppError;
use crate::middleware::AuthContext;
use crate::state::AppState;
use vaultrs_core::policy::Capability;
use vaultrs_core::transit::TransitEngine;

/// Build the `/v1/transit` router.
///
/// Paths:
/// - `POST /v1/transit/keys/{name}` — create key
/// - `POST /v1/transit/keys/{name}/rotate` — rotate key
/// - `POST /v1/transit/encrypt/{name}` — encrypt
/// - `POST /v1/transit/decrypt/{name}` — decrypt
/// - `POST /v1/transit/rewrap/{name}` — rewrap
/// - `POST /v1/transit/datakey/{name}` — generate data key
/// - `GET  /v1/transit/keys` — list keys
/// - `GET  /v1/transit/keys/{name}` — key info
pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/keys", get(list_keys))
        .route("/keys/{name}", get(key_info).post(create_key))
        .route("/keys/{name}/rotate", post(rotate_key))
        .route("/encrypt/{name}", post(encrypt))
        .route("/decrypt/{name}", post(decrypt))
        .route("/rewrap/{name}", post(rewrap))
        .route("/datakey/{name}", post(generate_data_key))
}

// ── Request / Response types ─────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct EncryptRequest {
    /// Base64-encoded plaintext.
    pub plaintext: String,
}

#[derive(Debug, Serialize)]
pub struct EncryptResponse {
    pub ciphertext: String,
}

#[derive(Debug, Deserialize)]
pub struct DecryptRequest {
    /// Ciphertext in `vault:v{N}:{base64}` format.
    pub ciphertext: String,
}

#[derive(Debug, Serialize)]
pub struct DecryptResponse {
    /// Base64-encoded plaintext.
    pub plaintext: String,
}

#[derive(Debug, Deserialize)]
pub struct RewrapRequest {
    /// Ciphertext to re-wrap under the latest key version.
    pub ciphertext: String,
}

#[derive(Debug, Serialize)]
pub struct RewrapResponse {
    pub ciphertext: String,
}

#[derive(Debug, Serialize)]
pub struct DataKeyResponse {
    /// Base64-encoded plaintext data key.
    pub plaintext: String,
    /// Transit-encrypted data key.
    pub ciphertext: String,
}

#[derive(Debug, Serialize)]
pub struct KeyListResponse {
    pub keys: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct KeyInfoResponse {
    pub name: String,
    pub latest_version: u32,
    pub min_decryption_version: u32,
    pub supports_encryption: bool,
    pub supports_decryption: bool,
    pub version_count: u32,
    pub created_at: String,
}

#[derive(Debug, Serialize)]
pub struct RotateResponse {
    pub new_version: u32,
}

// ── Handlers ─────────────────────────────────────────────────────────

/// Create a new named transit key.
async fn create_key(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthContext>,
    Path(name): Path<String>,
) -> Result<StatusCode, AppError> {
    state
        .policy_store
        .check(&auth.policies, &format!("transit/keys/{name}"), &Capability::Create)
        .await?;

    let engine = get_transit_engine(&state).await?;
    engine.create_key(&name).await?;

    Ok(StatusCode::NO_CONTENT)
}

/// Rotate a named transit key.
async fn rotate_key(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthContext>,
    Path(name): Path<String>,
) -> Result<Json<RotateResponse>, AppError> {
    state
        .policy_store
        .check(&auth.policies, &format!("transit/keys/{name}"), &Capability::Update)
        .await?;

    let engine = get_transit_engine(&state).await?;
    let new_version = engine.rotate_key(&name).await?;

    Ok(Json(RotateResponse { new_version }))
}

/// Encrypt plaintext using a named transit key.
async fn encrypt(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthContext>,
    Path(name): Path<String>,
    Json(body): Json<EncryptRequest>,
) -> Result<Json<EncryptResponse>, AppError> {
    state
        .policy_store
        .check(&auth.policies, &format!("transit/encrypt/{name}"), &Capability::Update)
        .await?;

    let plaintext_bytes = base64_decode(&body.plaintext)?;
    let engine = get_transit_engine(&state).await?;
    let ciphertext = engine.encrypt(&name, &plaintext_bytes).await?;

    Ok(Json(EncryptResponse { ciphertext }))
}

/// Decrypt ciphertext using a named transit key.
async fn decrypt(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthContext>,
    Path(name): Path<String>,
    Json(body): Json<DecryptRequest>,
) -> Result<Json<DecryptResponse>, AppError> {
    state
        .policy_store
        .check(&auth.policies, &format!("transit/decrypt/{name}"), &Capability::Update)
        .await?;

    let engine = get_transit_engine(&state).await?;
    let plaintext = engine.decrypt(&name, &body.ciphertext).await?;

    let plaintext_b64 = BASE64.encode(&plaintext);

    Ok(Json(DecryptResponse { plaintext: plaintext_b64 }))
}

/// Re-wrap ciphertext under the latest key version.
async fn rewrap(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthContext>,
    Path(name): Path<String>,
    Json(body): Json<RewrapRequest>,
) -> Result<Json<RewrapResponse>, AppError> {
    state
        .policy_store
        .check(&auth.policies, &format!("transit/rewrap/{name}"), &Capability::Update)
        .await?;

    let engine = get_transit_engine(&state).await?;
    let ciphertext = engine.rewrap(&name, &body.ciphertext).await?;

    Ok(Json(RewrapResponse { ciphertext }))
}

/// Generate a data encryption key wrapped by a named transit key.
async fn generate_data_key(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthContext>,
    Path(name): Path<String>,
) -> Result<Json<DataKeyResponse>, AppError> {
    state
        .policy_store
        .check(&auth.policies, &format!("transit/datakey/{name}"), &Capability::Update)
        .await?;

    let engine = get_transit_engine(&state).await?;
    let dk = engine.generate_data_key(&name).await?;

    Ok(Json(DataKeyResponse {
        plaintext: dk.plaintext,
        ciphertext: dk.ciphertext,
    }))
}

/// List all transit key names.
async fn list_keys(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthContext>,
) -> Result<Json<KeyListResponse>, AppError> {
    state
        .policy_store
        .check(&auth.policies, "transit/keys", &Capability::List)
        .await?;

    let engine = get_transit_engine(&state).await?;
    let keys = engine.list_keys().await?;

    Ok(Json(KeyListResponse { keys }))
}

/// Get metadata about a named transit key.
async fn key_info(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthContext>,
    Path(name): Path<String>,
) -> Result<Json<KeyInfoResponse>, AppError> {
    state
        .policy_store
        .check(&auth.policies, &format!("transit/keys/{name}"), &Capability::Read)
        .await?;

    let engine = get_transit_engine(&state).await?;
    let info = engine.key_info(&name).await?;

    Ok(Json(KeyInfoResponse {
        name: info.name,
        latest_version: info.latest_version,
        min_decryption_version: info.min_decryption_version,
        supports_encryption: info.supports_encryption,
        supports_decryption: info.supports_decryption,
        version_count: info.version_count,
        created_at: info.created_at.to_rfc3339(),
    }))
}

// ── Helpers ──────────────────────────────────────────────────────────

/// Get the default transit engine from state.
async fn get_transit_engine(state: &AppState) -> Result<Arc<TransitEngine>, AppError> {
    state
        .transit_engines
        .read()
        .await
        .get("transit/")
        .cloned()
        .ok_or_else(|| AppError::NotFound("no transit engine mounted".to_owned()))
}

/// Decode base64 input, returning a user-friendly error.
fn base64_decode(input: &str) -> Result<Vec<u8>, AppError> {
    BASE64
        .decode(input)
        .map_err(|e| AppError::BadRequest(format!("invalid base64 input: {e}")))
}

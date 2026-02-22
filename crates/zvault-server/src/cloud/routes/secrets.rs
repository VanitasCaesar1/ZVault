//! Secret management routes.
//!
//! CRUD operations on secrets within a project environment. Secret values
//! are encrypted with per-org AES-256-GCM keys before storage and decrypted
//! on read. Nonces are generated fresh for every write via `OsRng`.

use axum::extract::{Path, State};
use axum::routing::get;
use axum::{Extension, Json, Router};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

use crate::cloud::auth::CloudIdentity;
use crate::cloud::error::CloudError;
use crate::cloud::models::{SecretEntry, SecretKey};
use crate::cloud::repository;

/// Request body for setting a secret.
#[derive(Debug, Deserialize)]
pub struct SetSecretRequest {
    pub value: String,
    #[serde(default)]
    pub comment: String,
}

/// Response for a single secret.
#[derive(Debug, Serialize)]
pub struct SecretResponse {
    pub secret: SecretEntry,
}

/// Response for secret key listing (no values).
#[derive(Debug, Serialize)]
pub struct SecretKeysResponse {
    pub keys: Vec<SecretKey>,
}

/// Build the secrets router.
pub fn router() -> Router<PgPool> {
    Router::new()
        .route(
            "/orgs/{org_id}/projects/{project_id}/envs/{env_slug}/secrets",
            get(list_secrets),
        )
        .route(
            "/orgs/{org_id}/projects/{project_id}/envs/{env_slug}/secrets/{key}",
            get(get_secret).put(set_secret).delete(delete_secret),
        )
}

/// Encrypt a secret value with the org's AES-256-GCM key.
///
/// Returns `(ciphertext, nonce)`. Nonce is generated fresh via `OsRng`.
///
/// # Errors
///
/// Returns `CloudError::Internal` if encryption fails.
fn encrypt_secret(org_key: &[u8], plaintext: &str) -> Result<(Vec<u8>, Vec<u8>), CloudError> {
    use aes_gcm::aead::{Aead, OsRng};
    use aes_gcm::aead::rand_core::RngCore;
    use aes_gcm::{Aes256Gcm, KeyInit, Nonce};

    if org_key.len() != 32 {
        return Err(CloudError::Internal(
            "invalid org encryption key length".to_owned(),
        ));
    }

    let cipher = Aes256Gcm::new_from_slice(org_key)
        .map_err(|e| CloudError::Internal(format!("cipher init: {e}")))?;

    // Fresh 96-bit nonce from OS CSPRNG.
    let mut nonce_bytes = [0u8; 12];
    OsRng.fill_bytes(&mut nonce_bytes);
    let nonce = Nonce::from_slice(&nonce_bytes);

    let ciphertext = cipher
        .encrypt(nonce, plaintext.as_bytes())
        .map_err(|e| CloudError::Internal(format!("encryption failed: {e}")))?;

    Ok((ciphertext, nonce_bytes.to_vec()))
}

/// Decrypt a secret value with the org's AES-256-GCM key.
///
/// # Errors
///
/// Returns `CloudError::Internal` if decryption fails.
fn decrypt_secret(
    org_key: &[u8],
    ciphertext: &[u8],
    nonce_bytes: &[u8],
) -> Result<String, CloudError> {
    use aes_gcm::aead::Aead;
    use aes_gcm::{Aes256Gcm, KeyInit, Nonce};

    if org_key.len() != 32 {
        return Err(CloudError::Internal(
            "invalid org encryption key length".to_owned(),
        ));
    }
    if nonce_bytes.len() != 12 {
        return Err(CloudError::Internal(
            "invalid nonce length".to_owned(),
        ));
    }

    let cipher = Aes256Gcm::new_from_slice(org_key)
        .map_err(|e| CloudError::Internal(format!("cipher init: {e}")))?;

    let nonce = Nonce::from_slice(nonce_bytes);

    let plaintext = cipher
        .decrypt(nonce, ciphertext)
        .map_err(|_| CloudError::Internal("decryption failed — key mismatch or corrupted data".to_owned()))?;

    String::from_utf8(plaintext)
        .map_err(|e| CloudError::Internal(format!("decrypted value is not valid UTF-8: {e}")))
}

/// Resolve org + project + environment from path params.
///
/// Returns `(org, environment)` after verifying access.
async fn resolve_env(
    pool: &PgPool,
    identity: &CloudIdentity,
    org_id: Uuid,
    project_id: Uuid,
    env_slug: &str,
) -> Result<(super::super::models::Organization, super::super::models::Environment), CloudError> {
    // For service tokens, verify the token is scoped to this project.
    match identity {
        CloudIdentity::User { user_id, .. } => {
            repository::check_org_access(pool, org_id, *user_id).await?;
        }
        CloudIdentity::ServiceToken {
            project_id: token_project_id,
            environment_id: token_env_id,
            ..
        } => {
            // Service token must be scoped to this project.
            if *token_project_id != project_id {
                return Err(CloudError::Forbidden(
                    "service token is not scoped to this project".to_owned(),
                ));
            }
            // If scoped to a specific environment, verify it matches.
            if let Some(env_id) = token_env_id {
                let env = repository::get_environment_by_slug(pool, project_id, env_slug).await?;
                if env.id != *env_id {
                    return Err(CloudError::Forbidden(
                        "service token is not scoped to this environment".to_owned(),
                    ));
                }
            }
        }
    }

    // Verify project belongs to org.
    repository::get_project(pool, project_id, org_id).await?;

    let org = repository::get_org(pool, org_id).await?;
    let env = repository::get_environment_by_slug(pool, project_id, env_slug).await?;

    Ok((org, env))
}

/// `GET /v1/cloud/orgs/{org_id}/projects/{project_id}/envs/{env_slug}/secrets`
///
/// List secret keys (no values) for an environment.
async fn list_secrets(
    State(pool): State<PgPool>,
    Extension(identity): Extension<CloudIdentity>,
    Path((org_id, project_id, env_slug)): Path<(Uuid, Uuid, String)>,
) -> Result<Json<SecretKeysResponse>, CloudError> {
    let (_org, env) = resolve_env(&pool, &identity, org_id, project_id, &env_slug).await?;
    let keys = repository::list_secret_keys(&pool, env.id).await?;

    Ok(Json(SecretKeysResponse { keys }))
}

/// `GET /v1/cloud/orgs/{org_id}/projects/{project_id}/envs/{env_slug}/secrets/{key}`
///
/// Get a single secret (decrypted).
async fn get_secret(
    State(pool): State<PgPool>,
    Extension(identity): Extension<CloudIdentity>,
    Path((org_id, project_id, env_slug, key)): Path<(Uuid, Uuid, String, String)>,
) -> Result<Json<SecretResponse>, CloudError> {
    let (org, env) = resolve_env(&pool, &identity, org_id, project_id, &env_slug).await?;
    let encrypted = repository::get_secret(&pool, env.id, &key).await?;

    let value = decrypt_secret(&org.encryption_key, &encrypted.encrypted_value, &encrypted.nonce)?;

    Ok(Json(SecretResponse {
        secret: SecretEntry {
            key: encrypted.key,
            value,
            version: encrypted.version,
            comment: encrypted.comment,
            created_at: encrypted.created_at,
            updated_at: encrypted.updated_at,
        },
    }))
}

/// `PUT /v1/cloud/orgs/{org_id}/projects/{project_id}/envs/{env_slug}/secrets/{key}`
///
/// Set (create or update) a secret. Value is encrypted with per-org AES-256-GCM.
async fn set_secret(
    State(pool): State<PgPool>,
    Extension(identity): Extension<CloudIdentity>,
    Path((org_id, project_id, env_slug, key)): Path<(Uuid, Uuid, String, String)>,
    Json(body): Json<SetSecretRequest>,
) -> Result<Json<SecretResponse>, CloudError> {
    // Check write permission for service tokens.
    if let CloudIdentity::ServiceToken { permissions, .. } = &identity {
        if !permissions.contains(&"write".to_owned()) {
            return Err(CloudError::Forbidden(
                "service token does not have write permission".to_owned(),
            ));
        }
    }

    let (org, env) = resolve_env(&pool, &identity, org_id, project_id, &env_slug).await?;

    // Validate key format.
    if key.is_empty() || key.len() > 256 {
        return Err(CloudError::BadRequest(
            "secret key must be 1-256 characters".to_owned(),
        ));
    }

    // Validate value size (max 1MB).
    if body.value.len() > 1_048_576 {
        return Err(CloudError::BadRequest(
            "secret value must be under 1MB".to_owned(),
        ));
    }

    let (ciphertext, nonce) = encrypt_secret(&org.encryption_key, &body.value)?;

    let actor_id = match &identity {
        CloudIdentity::User { user_id, .. } => Some(*user_id),
        CloudIdentity::ServiceToken { token_id, .. } => Some(*token_id),
    };

    let encrypted = repository::upsert_secret(
        &pool,
        env.id,
        &key,
        &ciphertext,
        &nonce,
        &body.comment,
        actor_id,
    )
    .await?;

    Ok(Json(SecretResponse {
        secret: SecretEntry {
            key: encrypted.key,
            value: body.value,
            version: encrypted.version,
            comment: encrypted.comment,
            created_at: encrypted.created_at,
            updated_at: encrypted.updated_at,
        },
    }))
}

/// `DELETE /v1/cloud/orgs/{org_id}/projects/{project_id}/envs/{env_slug}/secrets/{key}`
///
/// Delete a secret.
async fn delete_secret(
    State(pool): State<PgPool>,
    Extension(identity): Extension<CloudIdentity>,
    Path((org_id, project_id, env_slug, key)): Path<(Uuid, Uuid, String, String)>,
) -> Result<Json<serde_json::Value>, CloudError> {
    // Check write permission for service tokens.
    if let CloudIdentity::ServiceToken { permissions, .. } = &identity {
        if !permissions.contains(&"write".to_owned()) {
            return Err(CloudError::Forbidden(
                "service token does not have write permission".to_owned(),
            ));
        }
    }

    let (_org, env) = resolve_env(&pool, &identity, org_id, project_id, &env_slug).await?;
    repository::delete_secret(&pool, env.id, &key).await?;

    Ok(Json(serde_json::json!({ "ok": true })))
}

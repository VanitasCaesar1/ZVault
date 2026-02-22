//! Service token management routes.
//!
//! Create, list, and revoke service tokens scoped to a project
//! (optionally to a specific environment). Tokens are SHA-256 hashed
//! before storage — the plaintext is returned only once at creation.

use axum::extract::{Path, State};
use axum::routing::post;
use axum::{Extension, Json, Router};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

use crate::cloud::auth::{generate_service_token, hash_token, token_prefix, CloudIdentity};
use crate::cloud::error::CloudError;
use crate::cloud::models::ServiceToken;
use crate::cloud::repository;

/// Request body for creating a service token.
#[derive(Debug, Deserialize)]
pub struct CreateTokenRequest {
    pub name: String,
    pub environment_id: Option<Uuid>,
    #[serde(default = "default_permissions")]
    pub permissions: Vec<String>,
    pub expires_at: Option<DateTime<Utc>>,
}

fn default_permissions() -> Vec<String> {
    vec!["read".to_owned()]
}

/// Response for token creation (includes plaintext token — shown only once).
#[derive(Debug, Serialize)]
pub struct CreateTokenResponse {
    pub token: ServiceToken,
    /// The plaintext token. Store it securely — it cannot be retrieved again.
    pub plaintext_token: String,
}

/// Response for token listing.
#[derive(Debug, Serialize)]
pub struct TokenListResponse {
    pub tokens: Vec<ServiceToken>,
}

/// Build the tokens router.
pub fn router() -> Router<PgPool> {
    Router::new()
        .route(
            "/orgs/{org_id}/projects/{project_id}/tokens",
            post(create_token).get(list_tokens),
        )
        .route(
            "/orgs/{org_id}/projects/{project_id}/tokens/{token_id}/revoke",
            post(revoke_token),
        )
}

/// `POST /v1/cloud/orgs/{org_id}/projects/{project_id}/tokens` — create a service token.
///
/// Returns the plaintext token exactly once. It cannot be retrieved again.
async fn create_token(
    State(pool): State<PgPool>,
    Extension(identity): Extension<CloudIdentity>,
    Path((org_id, project_id)): Path<(Uuid, Uuid)>,
    Json(body): Json<CreateTokenRequest>,
) -> Result<Json<CreateTokenResponse>, CloudError> {
    let CloudIdentity::User { user_id, .. } = identity else {
        return Err(CloudError::Forbidden(
            "service tokens cannot create other service tokens".to_owned(),
        ));
    };

    // Verify access (admin or developer).
    let role = repository::check_org_access(&pool, org_id, user_id).await?;
    if role == "viewer" {
        return Err(CloudError::Forbidden(
            "viewers cannot create service tokens".to_owned(),
        ));
    }

    // Verify project belongs to org.
    repository::get_project(&pool, project_id, org_id).await?;

    if body.name.is_empty() {
        return Err(CloudError::BadRequest("name is required".to_owned()));
    }

    // Validate permissions.
    let valid_perms = ["read", "write"];
    for perm in &body.permissions {
        if !valid_perms.contains(&perm.as_str()) {
            return Err(CloudError::BadRequest(format!(
                "invalid permission '{perm}' — must be one of: {}",
                valid_perms.join(", ")
            )));
        }
    }

    // If scoped to an environment, verify it exists.
    if let Some(env_id) = body.environment_id {
        let envs = repository::list_environments(&pool, project_id).await?;
        if !envs.iter().any(|e| e.id == env_id) {
            return Err(CloudError::NotFound(
                "environment not found in this project".to_owned(),
            ));
        }
    }

    // Generate token.
    let plaintext = generate_service_token();
    let hash = hash_token(&plaintext);
    let prefix = token_prefix(&plaintext);

    let token = repository::create_service_token(
        &pool,
        project_id,
        body.environment_id,
        &body.name,
        &hash,
        &prefix,
        &body.permissions,
        body.expires_at,
        Some(user_id),
    )
    .await?;

    Ok(Json(CreateTokenResponse {
        token,
        plaintext_token: plaintext,
    }))
}

/// `GET /v1/cloud/orgs/{org_id}/projects/{project_id}/tokens` — list service tokens.
async fn list_tokens(
    State(pool): State<PgPool>,
    Extension(identity): Extension<CloudIdentity>,
    Path((org_id, project_id)): Path<(Uuid, Uuid)>,
) -> Result<Json<TokenListResponse>, CloudError> {
    let CloudIdentity::User { user_id, .. } = identity else {
        return Err(CloudError::Forbidden(
            "service tokens cannot list other tokens".to_owned(),
        ));
    };

    repository::check_org_access(&pool, org_id, user_id).await?;
    repository::get_project(&pool, project_id, org_id).await?;

    let tokens = repository::list_service_tokens(&pool, project_id).await?;

    Ok(Json(TokenListResponse { tokens }))
}

/// `POST /v1/cloud/orgs/{org_id}/projects/{project_id}/tokens/{token_id}/revoke`
///
/// Revoke a service token. Revoked tokens cannot be used for authentication.
async fn revoke_token(
    State(pool): State<PgPool>,
    Extension(identity): Extension<CloudIdentity>,
    Path((org_id, project_id, token_id)): Path<(Uuid, Uuid, Uuid)>,
) -> Result<Json<serde_json::Value>, CloudError> {
    let CloudIdentity::User { user_id, .. } = identity else {
        return Err(CloudError::Forbidden(
            "service tokens cannot revoke other tokens".to_owned(),
        ));
    };

    let role = repository::check_org_access(&pool, org_id, user_id).await?;
    if role == "viewer" {
        return Err(CloudError::Forbidden(
            "viewers cannot revoke service tokens".to_owned(),
        ));
    }

    repository::get_project(&pool, project_id, org_id).await?;
    repository::revoke_service_token(&pool, token_id, project_id).await?;

    Ok(Json(serde_json::json!({ "ok": true })))
}

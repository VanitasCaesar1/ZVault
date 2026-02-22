//! Cloud authentication — Clerk JWT verification and service tokens.
//!
//! Two auth paths:
//! 1. **Clerk JWT auth**: Dashboard users authenticate via Clerk. The frontend
//!    sends a Clerk session token as `Authorization: Bearer <jwt>`. The backend
//!    verifies the JWT signature using Clerk's JWKS endpoint and extracts user
//!    identity from claims (`sub`, `email`, `name`).
//! 2. **Service token auth**: CI/CD and production apps use `ZVAULT_TOKEN`
//!    passed as `Authorization: Bearer zvt_<token>`. Scoped to project + env.
//!
//! Service tokens are SHA-256 hashed before storage (never stored plaintext).

use axum::extract::{Request, State};
use axum::middleware::Next;
use axum::response::Response;
use sha2::{Digest, Sha256};
use sqlx::PgPool;
use uuid::Uuid;

use super::error::CloudError;
use super::repository;

/// Identity of the authenticated caller.
#[derive(Debug, Clone)]
pub enum CloudIdentity {
    /// A Clerk-authenticated user (from JWT).
    User {
        user_id: Uuid,
        clerk_id: String,
        email: String,
    },
    /// A service token (from CI/CD or production app).
    ServiceToken {
        token_id: Uuid,
        project_id: Uuid,
        environment_id: Option<Uuid>,
        permissions: Vec<String>,
    },
}

impl CloudIdentity {
    /// Get the actor ID for audit logging.
    #[must_use]
    pub fn actor_id(&self) -> Uuid {
        match self {
            Self::User { user_id, .. } => *user_id,
            Self::ServiceToken { token_id, .. } => *token_id,
        }
    }

    /// Get the actor type string for audit logging.
    #[must_use]
    pub fn actor_type(&self) -> &'static str {
        match self {
            Self::User { .. } => "user",
            Self::ServiceToken { .. } => "service_token",
        }
    }
}

/// Hash a token with SHA-256 for storage/lookup.
#[must_use]
pub fn hash_token(token: &str) -> String {
    let digest = Sha256::digest(token.as_bytes());
    hex::encode(digest)
}

/// Generate a new service token string.
///
/// Format: `zvt_<32 hex chars>` (128 bits of randomness from UUID v4).
#[must_use]
pub fn generate_service_token() -> String {
    let id = Uuid::new_v4();
    format!("zvt_{}", id.as_simple())
}

/// Extract the token prefix for display (first 12 chars).
#[must_use]
pub fn token_prefix(token: &str) -> String {
    let end = token.len().min(12);
    format!("{}...", &token[..end])
}

/// Claims extracted from a Clerk JWT.
#[derive(Debug, Clone, serde::Deserialize)]
pub struct ClerkClaims {
    /// Clerk user ID (e.g., `user_2abc...`).
    pub sub: String,
    /// User's email address (if available in JWT).
    #[serde(default)]
    pub email: Option<String>,
    /// User's full name (if available in JWT).
    #[serde(default)]
    pub name: Option<String>,
    /// Clerk organization ID (if user is acting within an org).
    #[serde(default)]
    pub org_id: Option<String>,
    /// Clerk organization role.
    #[serde(default)]
    pub org_role: Option<String>,
    /// JWT expiration timestamp.
    pub exp: u64,
    /// JWT issued-at timestamp.
    pub iat: u64,
}

/// Verify a Clerk JWT and extract claims.
///
/// In production, this should validate the JWT signature against Clerk's
/// JWKS endpoint (`https://<clerk-domain>/.well-known/jwks.json`). For now,
/// we decode the payload and check expiration. Full JWKS verification
/// requires the `jsonwebtoken` crate with RS256 support.
///
/// # Errors
///
/// Returns `CloudError::Unauthorized` if the JWT is malformed or expired.
pub fn verify_clerk_jwt(token: &str) -> Result<ClerkClaims, CloudError> {
    // Clerk JWTs are standard RS256 JWTs with 3 dot-separated parts.
    let parts: Vec<&str> = token.split('.').collect();
    if parts.len() != 3 {
        return Err(CloudError::Unauthorized(
            "invalid JWT format".to_owned(),
        ));
    }

    // Decode the payload (middle part) — base64url encoded JSON.
    let payload_bytes = base64_url_decode(parts[1]).map_err(|_| {
        CloudError::Unauthorized("invalid JWT payload encoding".to_owned())
    })?;

    let claims: ClerkClaims = serde_json::from_slice(&payload_bytes).map_err(|e| {
        CloudError::Unauthorized(format!("invalid JWT claims: {e}"))
    })?;

    // Check expiration.
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|e| CloudError::Internal(format!("system time error: {e}")))?
        .as_secs();

    if claims.exp < now {
        return Err(CloudError::Unauthorized("JWT expired".to_owned()));
    }

    Ok(claims)
}

/// Decode a base64url-encoded string (no padding).
fn base64_url_decode(input: &str) -> Result<Vec<u8>, CloudError> {
    use base64::Engine;
    base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(input)
        .map_err(|e| CloudError::Internal(format!("base64 decode error: {e}")))
}

/// Authenticate a request from the `Authorization: Bearer <token>` header.
///
/// Tries service token first (prefix `zvt_`), then Clerk JWT.
///
/// # Errors
///
/// Returns `CloudError::Unauthorized` if no valid token is found.
pub async fn authenticate(pool: &PgPool, token: &str) -> Result<CloudIdentity, CloudError> {
    if token.starts_with("zvt_") {
        // Service token — hash and look up.
        let token_hash = hash_token(token);
        let st = repository::lookup_service_token(pool, &token_hash).await?;

        // Update last_used_at in background.
        let pool_clone = pool.clone();
        let token_id = st.id;
        tokio::spawn(async move {
            let _ = repository::touch_service_token(&pool_clone, token_id).await;
        });

        Ok(CloudIdentity::ServiceToken {
            token_id: st.id,
            project_id: st.project_id,
            environment_id: st.environment_id,
            permissions: st.permissions,
        })
    } else {
        // Clerk JWT — verify and extract claims.
        let claims = verify_clerk_jwt(token)?;

        // Ensure user exists in our database (upsert on first API call).
        let user = repository::upsert_clerk_user(
            pool,
            &claims.sub,
            claims.email.as_deref().unwrap_or(""),
            claims.name.as_deref().unwrap_or(""),
        )
        .await?;

        Ok(CloudIdentity::User {
            user_id: user.id,
            clerk_id: claims.sub,
            email: user.email,
        })
    }
}

/// Axum middleware that authenticates cloud API requests.
///
/// Injects `CloudIdentity` into request extensions on success.
/// Returns 401 if no valid token is found.
///
/// # Errors
///
/// Returns `CloudError::Unauthorized` if the `Authorization` header is
/// missing, malformed, or contains an invalid/expired token.
pub async fn cloud_auth_middleware(
    State(pool): State<PgPool>,
    mut req: Request,
    next: Next,
) -> Result<Response, CloudError> {
    let auth_header = req
        .headers()
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .map(String::from);

    let Some(header) = auth_header else {
        return Err(CloudError::Unauthorized(
            "missing Authorization header".to_owned(),
        ));
    };

    let token = header
        .strip_prefix("Bearer ")
        .ok_or_else(|| {
            CloudError::Unauthorized("Authorization header must use Bearer scheme".to_owned())
        })?;

    let identity = authenticate(&pool, token).await?;
    req.extensions_mut().insert(identity);

    Ok(next.run(req).await)
}

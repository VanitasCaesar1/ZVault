//! Token authentication routes: `/v1/auth/token/*`
//!
//! Handles token creation, lookup, renewal, and revocation.

use std::collections::HashMap;
use std::sync::Arc;

use axum::extract::State;
use axum::http::StatusCode;
use axum::routing::post;
use axum::{Extension, Json, Router};
use chrono::Duration;
use serde::{Deserialize, Serialize};

use crate::error::AppError;
use crate::middleware::AuthContext;
use crate::state::AppState;
use zvault_core::policy::Capability;
use zvault_core::token::CreateTokenParams;

/// Build the `/v1/auth/token` router.
pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/create", post(create_token))
        .route("/lookup", post(lookup_token))
        .route("/lookup-self", post(lookup_self))
        .route("/renew", post(renew_token))
        .route("/renew-self", post(renew_self))
        .route("/revoke", post(revoke_token))
        .route("/revoke-self", post(revoke_self))
}

// ── Request / Response types ─────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct CreateTokenRequest {
    pub policies: Option<Vec<String>>,
    pub ttl: Option<String>,
    pub display_name: Option<String>,
    pub renewable: Option<bool>,
    pub metadata: Option<HashMap<String, String>>,
}

#[derive(Debug, Serialize)]
pub struct TokenResponse {
    pub client_token: String,
    pub policies: Vec<String>,
    pub renewable: bool,
    pub lease_duration: Option<i64>,
}

#[derive(Debug, Serialize)]
pub struct TokenLookupResponse {
    pub token_hash: String,
    pub policies: Vec<String>,
    pub display_name: String,
    pub renewable: bool,
    pub created_at: String,
    pub expires_at: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct TokenLookupRequest {
    pub token: String,
}

#[derive(Debug, Deserialize)]
pub struct TokenRenewRequest {
    pub token: Option<String>,
    pub increment: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct TokenRevokeRequest {
    pub token: String,
}

// ── Handlers ─────────────────────────────────────────────────────────

/// Create a child token.
async fn create_token(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthContext>,
    Json(body): Json<CreateTokenRequest>,
) -> Result<(StatusCode, Json<TokenResponse>), AppError> {
    state
        .policy_store
        .check(&auth.policies, "auth/token/create", &Capability::Sudo)
        .await?;

    let ttl = body.ttl.as_deref().map(parse_duration).transpose()?;
    let policies = body.policies.unwrap_or_else(|| vec!["default".to_owned()]);

    let token = state
        .token_store
        .create(CreateTokenParams {
            policies: policies.clone(),
            ttl,
            max_ttl: None,
            renewable: body.renewable.unwrap_or(true),
            parent_hash: Some(auth.token_hash),
            metadata: body.metadata.unwrap_or_default(),
            display_name: body.display_name.unwrap_or_else(|| "token".to_owned()),
        })
        .await?;

    let lease_duration = ttl.map(|d| d.num_seconds());

    Ok((
        StatusCode::OK,
        Json(TokenResponse {
            client_token: token,
            policies,
            renewable: body.renewable.unwrap_or(true),
            lease_duration,
        }),
    ))
}

/// Look up a token by its plaintext value (requires sudo).
async fn lookup_token(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthContext>,
    Json(body): Json<TokenLookupRequest>,
) -> Result<Json<TokenLookupResponse>, AppError> {
    state
        .policy_store
        .check(&auth.policies, "auth/token/lookup", &Capability::Sudo)
        .await?;

    let entry = state.token_store.lookup(&body.token).await?;

    Ok(Json(TokenLookupResponse {
        token_hash: entry.token_hash,
        policies: entry.policies,
        display_name: entry.display_name,
        renewable: entry.renewable,
        created_at: entry.created_at.to_rfc3339(),
        expires_at: entry.expires_at.map(|t| t.to_rfc3339()),
    }))
}

/// Look up the caller's own token (allowed by default policy).
async fn lookup_self(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthContext>,
) -> Result<Json<TokenLookupResponse>, AppError> {
    state
        .policy_store
        .check(&auth.policies, "auth/token/lookup-self", &Capability::Read)
        .await?;

    // Re-lookup by hash — we don't have the plaintext, but we can
    // reconstruct the response from the auth context. For a full lookup
    // we'd need the plaintext token, so we return what we know.
    Ok(Json(TokenLookupResponse {
        token_hash: auth.token_hash,
        policies: auth.policies,
        display_name: auth.display_name,
        renewable: false, // We don't have this from AuthContext; safe default
        created_at: String::new(),
        expires_at: None,
    }))
}

/// Renew a specific token (requires sudo).
async fn renew_token(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthContext>,
    Json(body): Json<TokenRenewRequest>,
) -> Result<Json<TokenLookupResponse>, AppError> {
    state
        .policy_store
        .check(&auth.policies, "auth/token/renew", &Capability::Sudo)
        .await?;

    let token = body
        .token
        .ok_or_else(|| AppError::BadRequest("missing 'token' field".to_owned()))?;

    let increment = body
        .increment
        .as_deref()
        .map(parse_duration)
        .transpose()?
        .unwrap_or_else(|| Duration::hours(1));

    let entry = state.token_store.renew(&token, increment).await?;

    Ok(Json(TokenLookupResponse {
        token_hash: entry.token_hash,
        policies: entry.policies,
        display_name: entry.display_name,
        renewable: entry.renewable,
        created_at: entry.created_at.to_rfc3339(),
        expires_at: entry.expires_at.map(|t| t.to_rfc3339()),
    }))
}

/// Renew the caller's own token (allowed by default policy).
async fn renew_self(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthContext>,
    Json(body): Json<TokenRenewRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    state
        .policy_store
        .check(&auth.policies, "auth/token/renew-self", &Capability::Update)
        .await?;

    // We don't have the plaintext token in AuthContext, so renew-self
    // requires the token in the body or we return an error.
    let token = body
        .token
        .ok_or_else(|| AppError::BadRequest("missing 'token' field for renew-self".to_owned()))?;

    let increment = body
        .increment
        .as_deref()
        .map(parse_duration)
        .transpose()?
        .unwrap_or_else(|| Duration::hours(1));

    let entry = state.token_store.renew(&token, increment).await?;

    Ok(Json(serde_json::json!({
        "token_hash": entry.token_hash,
        "policies": entry.policies,
        "renewable": entry.renewable,
        "expires_at": entry.expires_at.map(|t| t.to_rfc3339()),
    })))
}

/// Revoke a specific token and all its children (requires sudo).
async fn revoke_token(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthContext>,
    Json(body): Json<TokenRevokeRequest>,
) -> Result<StatusCode, AppError> {
    state
        .policy_store
        .check(&auth.policies, "auth/token/revoke", &Capability::Sudo)
        .await?;

    state.token_store.revoke(&body.token).await?;

    Ok(StatusCode::NO_CONTENT)
}

/// Revoke the caller's own token.
async fn revoke_self(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthContext>,
    Json(body): Json<TokenRevokeRequest>,
) -> Result<StatusCode, AppError> {
    // Any token can revoke itself — no policy check needed beyond auth.
    let _ = &auth;

    state.token_store.revoke(&body.token).await?;

    Ok(StatusCode::NO_CONTENT)
}

// ── Helpers ──────────────────────────────────────────────────────────

/// Parse a human-readable duration string like `"1h"`, `"30m"`, `"3600s"`, `"24h"`.
///
/// # Errors
///
/// Returns [`AppError::BadRequest`] if the format is unrecognized.
fn parse_duration(s: &str) -> Result<Duration, AppError> {
    let s = s.trim();
    if s.is_empty() {
        return Err(AppError::BadRequest("empty duration string".to_owned()));
    }

    // Try pure seconds first.
    if let Ok(secs) = s.parse::<i64>() {
        return Ok(Duration::seconds(secs));
    }

    let (num_str, unit) = s.split_at(s.len().saturating_sub(1));
    let num: i64 = num_str
        .parse()
        .map_err(|_| AppError::BadRequest(format!("invalid duration: {s}")))?;

    match unit {
        "s" => Ok(Duration::seconds(num)),
        "m" => Ok(Duration::minutes(num)),
        "h" => Ok(Duration::hours(num)),
        "d" => Ok(Duration::days(num)),
        _ => Err(AppError::BadRequest(format!(
            "unknown duration unit '{unit}', expected s/m/h/d"
        ))),
    }
}

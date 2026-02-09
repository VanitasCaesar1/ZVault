//! Authentication middleware for `VaultRS`.
//!
//! Extracts the `X-Vault-Token` header, validates it against the token store,
//! and injects the token entry into the request extensions for downstream
//! handlers to use for policy checks.

use std::sync::Arc;

use axum::extract::{Request, State};
use axum::http::StatusCode;
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};

use crate::state::AppState;

/// Authentication context injected into request extensions.
#[derive(Debug, Clone)]
pub struct AuthContext {
    /// The token hash (for audit logging).
    pub token_hash: String,
    /// Policies attached to this token.
    pub policies: Vec<String>,
    /// Display name for audit.
    pub display_name: String,
}

/// Middleware that validates the `X-Vault-Token` header.
///
/// Skips auth for health and seal-status endpoints.
pub async fn auth_middleware(
    State(state): State<Arc<AppState>>,
    mut req: Request,
    next: Next,
) -> Response {
    let path = req.uri().path().to_owned();

    // Skip auth for public endpoints.
    if path == "/v1/sys/health"
        || path == "/v1/sys/seal-status"
        || path == "/v1/sys/init"
        || path == "/v1/sys/unseal"
        || path.starts_with("/app")
        || path == "/"
    {
        return next.run(req).await;
    }

    let token = req
        .headers()
        .get("X-Vault-Token")
        .and_then(|v| v.to_str().ok())
        .map(String::from);

    let Some(token) = token else {
        return (
            StatusCode::UNAUTHORIZED,
            axum::Json(serde_json::json!({"error": "unauthorized", "message": "missing X-Vault-Token header"})),
        ).into_response();
    };

    match state.token_store.lookup(&token).await {
        Ok(entry) => {
            let ctx = AuthContext {
                token_hash: entry.token_hash.clone(),
                policies: entry.policies.clone(),
                display_name: entry.display_name.clone(),
            };
            req.extensions_mut().insert(ctx);
            next.run(req).await
        }
        Err(_) => (
            StatusCode::UNAUTHORIZED,
            axum::Json(serde_json::json!({"error": "unauthorized", "message": "invalid or expired token"})),
        ).into_response(),
    }
}

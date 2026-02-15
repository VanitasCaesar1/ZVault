//! OIDC authentication routes: `/v1/auth/oidc/*`
//!
//! Integrates with Spring (or any OIDC provider) to allow users to
//! authenticate via OAuth 2.0 Authorization Code + PKCE flow.
//! On successful callback, a ZVault token is minted with the configured
//! default policy.

use std::collections::HashMap;
use std::sync::Arc;

use axum::extract::{Query, State};
use axum::response::{IntoResponse, Redirect, Response};
use axum::routing::get;
use axum::{Json, Router};
use base64::Engine as _;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tracing::{info, warn};

use crate::error::AppError;
use crate::state::AppState;
use zvault_core::token::CreateTokenParams;

/// Build the `/v1/auth/oidc` router (no auth required — these are login endpoints).
pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/login", get(oidc_login))
        .route("/callback", get(oidc_callback))
        .route("/config", get(oidc_config))
}

// ── Types ────────────────────────────────────────────────────────────

/// Query parameters returned by the OIDC provider on callback.
#[derive(Debug, Deserialize)]
pub struct OidcCallbackQuery {
    pub code: Option<String>,
    pub state: Option<String>,
    pub error: Option<String>,
    pub error_description: Option<String>,
}

/// Token endpoint response from the OIDC provider.
#[derive(Debug, Deserialize)]
struct TokenResponse {
    access_token: String,
    #[allow(dead_code)]
    token_type: Option<String>,
    #[allow(dead_code)]
    expires_in: Option<u64>,
    #[allow(dead_code)]
    id_token: Option<String>,
}

/// UserInfo response from the OIDC provider.
#[derive(Debug, Deserialize)]
struct UserInfoResponse {
    sub: String,
    email: Option<String>,
    name: Option<String>,
    #[serde(default)]
    roles: Vec<String>,
}

/// Public OIDC configuration response.
#[derive(Debug, Serialize)]
pub struct OidcConfigResponse {
    pub enabled: bool,
    pub provider: Option<String>,
    pub login_url: Option<String>,
}

// ── Handlers ─────────────────────────────────────────────────────────

/// `GET /v1/auth/oidc/config` — Check if OIDC is enabled and get login URL.
async fn oidc_config(
    State(state): State<Arc<AppState>>,
) -> Json<OidcConfigResponse> {
    match &state.spring_oauth {
        Some(_) => Json(OidcConfigResponse {
            enabled: true,
            provider: Some("spring".to_owned()),
            login_url: Some("/v1/auth/oidc/login".to_owned()),
        }),
        None => Json(OidcConfigResponse {
            enabled: false,
            provider: None,
            login_url: None,
        }),
    }
}

/// `GET /v1/auth/oidc/login` — Redirect to the OIDC provider's authorize endpoint.
///
/// Constructs the authorization URL with PKCE (S256) and redirects the user.
/// The `state` parameter carries a CSRF nonce + code verifier (base64-encoded).
async fn oidc_login(
    State(state): State<Arc<AppState>>,
) -> Result<Response, AppError> {
    let cfg = state.spring_oauth.as_ref().ok_or_else(|| {
        AppError::BadRequest("OIDC authentication is not configured".to_owned())
    })?;

    // Generate PKCE code verifier (64 hex chars from two UUIDs).
    let code_verifier = uuid::Uuid::new_v4().to_string().replace('-', "")
        + &uuid::Uuid::new_v4().to_string().replace('-', "");

    // S256 code challenge = BASE64URL(SHA256(code_verifier)).
    let code_challenge = {
        let hash = Sha256::digest(code_verifier.as_bytes());
        base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(hash)
    };

    // CSRF nonce.
    let csrf_state = uuid::Uuid::new_v4().to_string();

    let redirect_uri = cfg.redirect_uri.clone().unwrap_or_else(|| {
        "/v1/auth/oidc/callback".to_owned()
    });

    // Pack (csrf_state, code_verifier) into the OAuth state parameter.
    let combined_state = base64::engine::general_purpose::STANDARD
        .encode(format!("{csrf_state}|{code_verifier}"));

    let authorize_url = format!(
        "{}/authorize?response_type=code\
         &client_id={}\
         &redirect_uri={}\
         &scope={}\
         &state={}\
         &code_challenge={}\
         &code_challenge_method=S256",
        cfg.auth_url,
        urlencoding::encode(&cfg.client_id),
        urlencoding::encode(&redirect_uri),
        urlencoding::encode("openid email profile"),
        urlencoding::encode(&combined_state),
        urlencoding::encode(&code_challenge),
    );

    Ok(Redirect::temporary(&authorize_url).into_response())
}

/// `GET /v1/auth/oidc/callback` — Handle the OIDC provider's callback.
///
/// Exchanges the authorization code for tokens, fetches user info,
/// and mints a ZVault token with the appropriate policies.
async fn oidc_callback(
    State(state): State<Arc<AppState>>,
    Query(query): Query<OidcCallbackQuery>,
) -> Result<Response, AppError> {
    // Check for errors from the provider.
    if let Some(err) = &query.error {
        let desc = query.error_description.as_deref().unwrap_or("unknown error");
        warn!(error = %err, description = %desc, "OIDC provider returned error");
        let dashboard_url = std::env::var("DASHBOARD_URL")
            .unwrap_or_else(|_| "http://localhost:5173".to_owned());
        let redirect_url = format!(
            "{}/login?error={}",
            dashboard_url,
            urlencoding::encode(&format!("{err}: {desc}"))
        );
        return Ok(Redirect::temporary(&redirect_url).into_response());
    }

    let code = query.code.as_deref().ok_or_else(|| {
        AppError::BadRequest("missing authorization code".to_owned())
    })?;

    let combined_state = query.state.as_deref().ok_or_else(|| {
        AppError::BadRequest("missing state parameter".to_owned())
    })?;

    let cfg = state.spring_oauth.as_ref().ok_or_else(|| {
        AppError::Internal("OIDC not configured".to_owned())
    })?;

    // Decode the combined state to extract code_verifier.
    let decoded_state = base64::engine::general_purpose::STANDARD
        .decode(combined_state)
        .map_err(|_| AppError::BadRequest("invalid state parameter".to_owned()))?;

    let decoded_str = String::from_utf8(decoded_state)
        .map_err(|_| AppError::BadRequest("invalid state encoding".to_owned()))?;

    let parts: Vec<&str> = decoded_str.splitn(2, '|').collect();
    if parts.len() != 2 {
        return Err(AppError::BadRequest("malformed state parameter".to_owned()));
    }
    let code_verifier = parts[1];

    let redirect_uri = cfg.redirect_uri.clone().unwrap_or_else(|| {
        "/v1/auth/oidc/callback".to_owned()
    });

    // Exchange authorization code for tokens.
    let http_client = reqwest::Client::new();
    let token_resp = http_client
        .post(format!("{}/token", cfg.auth_url))
        .form(&[
            ("grant_type", "authorization_code"),
            ("code", code),
            ("redirect_uri", &redirect_uri),
            ("client_id", &cfg.client_id),
            ("client_secret", &cfg.client_secret),
            ("code_verifier", code_verifier),
        ])
        .send()
        .await
        .map_err(|e| AppError::Internal(format!("token exchange failed: {e}")))?;

    if !token_resp.status().is_success() {
        let status = token_resp.status();
        let body = token_resp.text().await.unwrap_or_default();
        warn!(status = %status, body = %body, "OIDC token exchange failed");
        let dashboard_url = std::env::var("DASHBOARD_URL")
            .unwrap_or_else(|_| "http://localhost:5173".to_owned());
        let redirect_url = format!(
            "{}/login?error={}",
            dashboard_url,
            urlencoding::encode("Authentication failed")
        );
        return Ok(Redirect::temporary(&redirect_url).into_response());
    }

    let tokens: TokenResponse = token_resp
        .json()
        .await
        .map_err(|e| AppError::Internal(format!("failed to parse token response: {e}")))?;

    // Fetch user info from the OIDC provider.
    let userinfo_resp = http_client
        .get(format!("{}/userinfo", cfg.auth_url))
        .header("Authorization", format!("Bearer {}", tokens.access_token))
        .send()
        .await
        .map_err(|e| AppError::Internal(format!("userinfo request failed: {e}")))?;

    let userinfo: UserInfoResponse = if userinfo_resp.status().is_success() {
        userinfo_resp
            .json()
            .await
            .map_err(|e| AppError::Internal(format!("failed to parse userinfo: {e}")))?
    } else {
        UserInfoResponse {
            sub: "unknown".to_owned(),
            email: None,
            name: None,
            roles: Vec::new(),
        }
    };

    // Determine policies based on user roles.
    let is_admin = userinfo.roles.iter().any(|r| {
        r.eq_ignore_ascii_case("admin") || r.eq_ignore_ascii_case("superadmin")
    });

    let policies = if is_admin {
        vec![cfg.admin_policy.clone(), cfg.default_policy.clone()]
    } else {
        vec![cfg.default_policy.clone()]
    };

    let display_name = userinfo
        .name
        .clone()
        .or_else(|| userinfo.email.clone())
        .unwrap_or_else(|| format!("oidc:{}", userinfo.sub));

    // Mint a ZVault token for this user.
    let mut metadata = HashMap::new();
    metadata.insert("auth_method".to_owned(), "oidc".to_owned());
    metadata.insert("oidc_sub".to_owned(), userinfo.sub.clone());
    if let Some(ref email) = userinfo.email {
        metadata.insert("email".to_owned(), email.clone());
    }

    let vault_token = state
        .token_store
        .create(CreateTokenParams {
            policies: policies.clone(),
            ttl: Some(chrono::Duration::hours(8)),
            max_ttl: Some(chrono::Duration::hours(24)),
            renewable: true,
            parent_hash: None,
            metadata,
            display_name: display_name.clone(),
        })
        .await
        .map_err(|e| AppError::Internal(format!("failed to create vault token: {e}")))?;

    info!(
        sub = %userinfo.sub,
        email = ?userinfo.email,
        policies = ?policies,
        "OIDC login successful, vault token minted"
    );

    // Redirect to dashboard with the token as a query parameter.
    // The dashboard JS will store it in a cookie and redirect to /.
    let dashboard_url = std::env::var("DASHBOARD_URL")
        .unwrap_or_else(|_| "http://localhost:5173".to_owned());
    let redirect_url = format!(
        "{}/?token={}",
        dashboard_url,
        urlencoding::encode(&vault_token),
    );

    Ok(Redirect::temporary(&redirect_url).into_response())
}

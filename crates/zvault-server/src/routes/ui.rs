//! Landing page and web UI routes.
//!
//! Serves a minimal landing page at `/` and handles the Spring OAuth
//! callback at `/auth/callback`. The dashboard SPA is deployed as a
//! separate service and talks to this server via `VITE_API_URL`.

use axum::extract::{Query, State};
use axum::response::{Html, IntoResponse, Redirect, Response};
use axum::routing::get;
use axum::Router;
use std::sync::Arc;

use crate::state::AppState;

/// Build the UI router.
pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/", get(landing_page))
        .route("/auth/callback", get(spring_oauth_callback))
}

// ── Spring OAuth callback ────────────────────────────────────────────

/// Query parameters from Spring's OAuth redirect.
#[derive(serde::Deserialize)]
struct OAuthCallbackParams {
    code: Option<String>,
    error: Option<String>,
}

/// Spring token endpoint response.
#[derive(serde::Deserialize)]
struct SpringTokenResponse {
    access_token: String,
    #[allow(dead_code)]
    token_type: Option<String>,
    #[allow(dead_code)]
    expires_in: Option<u64>,
}

/// Spring userinfo endpoint response.
#[derive(serde::Deserialize)]
struct SpringUserInfo {
    sub: String,
    name: Option<String>,
    email: Option<String>,
}

/// Handle the OAuth callback from Spring.
///
/// Exchanges the authorization code for tokens, fetches user info,
/// creates a vault token with appropriate policies, and redirects
/// to the dashboard service with the token as a query parameter.
async fn spring_oauth_callback(
    State(state): State<Arc<AppState>>,
    Query(params): Query<OAuthCallbackParams>,
) -> Response {
    let dashboard_url = std::env::var("DASHBOARD_URL")
        .unwrap_or_else(|_| "http://localhost:5173".to_owned());

    // If Spring returned an error, redirect to login with message.
    if let Some(err) = params.error {
        return Redirect::to(&format!("{}/login?error={}", dashboard_url, err)).into_response();
    }

    let code = match params.code {
        Some(c) => c,
        None => return Redirect::to(&format!("{}/login?error=missing_code", dashboard_url)).into_response(),
    };

    let oauth_config = match &state.spring_oauth {
        Some(cfg) => cfg,
        None => return Redirect::to(&format!("{}/login?error=oauth_not_configured", dashboard_url)).into_response(),
    };

    // Build redirect URI — use configured value or derive from callback.
    let redirect_uri = oauth_config
        .redirect_uri
        .clone()
        .unwrap_or_else(|| format!("{}/auth/callback", "http://localhost:8200"));

    // Exchange authorization code for tokens.
    let token_response = match exchange_code(oauth_config, &code, &redirect_uri).await {
        Ok(resp) => resp,
        Err(e) => {
            tracing::warn!(error = %e, "Spring token exchange failed");
            return Redirect::to(&format!("{}/login?error=token_exchange_failed", dashboard_url)).into_response();
        }
    };

    // Fetch user info from Spring.
    let user_info = match fetch_userinfo(oauth_config, &token_response.access_token).await {
        Ok(info) => info,
        Err(e) => {
            tracing::warn!(error = %e, "Spring userinfo fetch failed");
            return Redirect::to(&format!("{}/login?error=userinfo_failed", dashboard_url)).into_response();
        }
    };

    // Determine vault policy based on user.
    let policy = oauth_config.default_policy.clone();

    // Create a vault token for this user.
    let display_name = user_info
        .name
        .unwrap_or_else(|| user_info.sub.clone());

    let mut metadata = std::collections::HashMap::new();
    metadata.insert("spring_sub".to_owned(), user_info.sub);
    if let Some(email) = user_info.email {
        metadata.insert("email".to_owned(), email);
    }

    let vault_token = match state
        .token_store
        .create(zvault_core::token::CreateTokenParams {
            policies: vec![policy],
            ttl: Some(chrono::Duration::hours(24)),
            max_ttl: None,
            renewable: true,
            parent_hash: None,
            metadata,
            display_name,
        })
        .await
    {
        Ok(token) => token,
        Err(e) => {
            tracing::warn!(error = %e, "failed to create vault token for Spring user");
            return Redirect::to(&format!("{}/login?error=vault_token_failed", dashboard_url)).into_response();
        }
    };

    // Redirect to dashboard with token as query param (dashboard stores it).
    Redirect::to(&format!("{}/?token={}", dashboard_url, vault_token)).into_response()
}

/// Exchange an authorization code for tokens at Spring's `/token` endpoint.
async fn exchange_code(
    config: &crate::config::SpringOAuthConfig,
    code: &str,
    redirect_uri: &str,
) -> Result<SpringTokenResponse, String> {
    let client = reqwest::Client::new();
    let token_url = format!("{}/token", config.auth_url);

    let resp = client
        .post(&token_url)
        .form(&[
            ("grant_type", "authorization_code"),
            ("code", code),
            ("redirect_uri", redirect_uri),
            ("client_id", &config.client_id),
            ("client_secret", &config.client_secret),
        ])
        .send()
        .await
        .map_err(|e| format!("HTTP error: {e}"))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        return Err(format!("Spring token endpoint returned {status}: {body}"));
    }

    resp.json::<SpringTokenResponse>()
        .await
        .map_err(|e| format!("failed to parse token response: {e}"))
}

/// Fetch user info from Spring's `/userinfo` endpoint.
async fn fetch_userinfo(
    config: &crate::config::SpringOAuthConfig,
    access_token: &str,
) -> Result<SpringUserInfo, String> {
    let client = reqwest::Client::new();
    let userinfo_url = format!("{}/userinfo", config.auth_url);

    let resp = client
        .get(&userinfo_url)
        .bearer_auth(access_token)
        .send()
        .await
        .map_err(|e| format!("HTTP error: {e}"))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        return Err(format!("Spring userinfo endpoint returned {status}: {body}"));
    }

    resp.json::<SpringUserInfo>()
        .await
        .map_err(|e| format!("failed to parse userinfo response: {e}"))
}

// ── Landing page ─────────────────────────────────────────────────────

async fn landing_page() -> Html<String> {
    let dashboard_url = std::env::var("DASHBOARD_URL")
        .unwrap_or_else(|_| "http://localhost:5173".to_owned());
    let docs_url = std::env::var("DOCS_URL")
        .unwrap_or_else(|_| "https://docs.zvault.cloud".to_owned());

    let mut html = String::with_capacity(32768);
    html.push_str(LANDING_CSS);
    // Replace placeholder URLs in the body.
    let body = LANDING_BODY
        .replace("{{DASHBOARD_URL}}", &dashboard_url)
        .replace("{{DOCS_URL}}", &docs_url);
    html.push_str(&body);
    Html(html)
}

/// CSS and HTML head for the marketing landing page at `/`.
const LANDING_CSS: &str = r##"<!DOCTYPE html>
<html lang="en"><head><meta charset="utf-8"/><meta name="viewport" content="width=device-width,initial-scale=1"/>
<title>ZVault &mdash; Secrets Management</title>
<link rel="preconnect" href="https://fonts.googleapis.com"/>
<link rel="preconnect" href="https://fonts.gstatic.com" crossorigin/>
<link href="https://fonts.googleapis.com/css2?family=Plus+Jakarta+Sans:wght@400;500;600;700;800&display=swap" rel="stylesheet"/>
<style>
*,*::before,*::after{box-sizing:border-box;margin:0;padding:0}
:root{--bg:#1E1610;--text:#F5E6B8;--text-muted:#A69274;--primary:#F5C842;--glass:rgba(255,255,255,.04);--glass-border:rgba(255,255,255,.08);--font:'Plus Jakarta Sans',-apple-system,sans-serif}
body{font-family:var(--font);background:var(--bg);color:var(--text);line-height:1.6;-webkit-font-smoothing:antialiased;overflow-x:hidden}
a{color:inherit;text-decoration:none}
.nav{display:flex;align-items:center;justify-content:space-between;max-width:1100px;margin:0 auto;padding:24px}
.nav-logo{display:flex;align-items:center;gap:12px;font-size:20px;font-weight:800;color:#F5E6B8}
.nav-logo svg{width:32px;height:32px}
.nav-links{display:flex;align-items:center;gap:8px}
.nav-links a{color:#A69274;transition:all .2s;font-size:14px;font-weight:600;padding:8px 16px;border-radius:50px}
.nav-links a:hover{color:#F5E6B8;background:rgba(255,255,255,.05)}
.nav-links .nav-pill{background:rgba(245,200,66,.12);color:#F5C842;border:1px solid rgba(245,200,66,.2)}
.nav-links .nav-pill:hover{background:rgba(245,200,66,.2)}
.btn{display:inline-flex;align-items:center;justify-content:center;gap:6px;padding:12px 28px;border-radius:50px;font-size:14px;font-weight:700;font-family:var(--font);border:none;cursor:pointer;transition:all .25s}
.btn-primary{background:linear-gradient(135deg,#F5C842,#E8A817);color:#2D1F0E;box-shadow:0 4px 20px rgba(245,200,66,.2)}.btn-primary:hover{box-shadow:0 8px 32px rgba(245,200,66,.35);transform:translateY(-2px)}
.btn-outline{background:transparent;color:#F5E6B8;border:1.5px solid rgba(245,230,184,.2)}.btn-outline:hover{border-color:rgba(245,230,184,.4);background:rgba(245,230,184,.05)}
.hero{text-align:center;max-width:800px;margin:0 auto;padding:120px 24px 80px;position:relative}
.hero::before{content:'';position:absolute;top:50%;left:50%;transform:translate(-50%,-50%);width:600px;height:600px;background:radial-gradient(circle,rgba(245,200,66,.08) 0%,transparent 70%);pointer-events:none}
.hero h1{font-size:60px;font-weight:800;line-height:1.06;letter-spacing:-2.5px;margin-bottom:24px;color:#FFFDF7;position:relative}
.hero h1 span{background:linear-gradient(135deg,#F5C842,#F5E6B8,#D4A843);-webkit-background-clip:text;-webkit-text-fill-color:transparent;background-clip:text}
.hero p{font-size:18px;color:#A69274;max-width:520px;margin:0 auto 40px;line-height:1.75;position:relative}
.hero-actions{display:flex;gap:14px;justify-content:center;position:relative}
.features{max-width:1100px;margin:0 auto;padding:40px 24px 80px;display:grid;grid-template-columns:repeat(3,1fr);gap:18px}
.feature{background:var(--glass);border:1px solid var(--glass-border);border-radius:20px;padding:32px;transition:all .25s;backdrop-filter:blur(8px)}
.feature:hover{background:rgba(255,255,255,.07);border-color:rgba(245,200,66,.15);transform:translateY(-2px)}
.feature-icon{width:48px;height:48px;border-radius:14px;display:flex;align-items:center;justify-content:center;margin-bottom:20px;background:rgba(245,200,66,.1)}
.feature-icon svg{width:24px;height:24px;stroke:#F5C842}
.feature h3{font-size:16px;font-weight:700;margin-bottom:8px;color:#F5E6B8}
.feature p{font-size:14px;color:#7A6543;line-height:1.7}
.cta{text-align:center;padding:60px 24px 80px;position:relative}
.cta::before{content:'';position:absolute;top:0;left:50%;transform:translateX(-50%);width:400px;height:1px;background:linear-gradient(90deg,transparent,rgba(245,200,66,.2),transparent)}
.cta h2{font-size:36px;font-weight:800;margin-bottom:16px;color:#FFFDF7;letter-spacing:-.5px}
.cta p{font-size:16px;color:#A69274;margin-bottom:32px}
.footer{border-top:1px solid rgba(245,230,184,.06);max-width:1100px;margin:0 auto;padding:24px;display:flex;justify-content:space-between;font-size:13px;color:#5A4A36}
@media(max-width:768px){.hero h1{font-size:36px}.features{grid-template-columns:1fr}.nav-links{display:none}}
</style></head>
"##;

/// HTML body for the marketing landing page.
const LANDING_BODY: &str = r##"<body>
<nav class="nav">
  <div class="nav-logo">
    <svg viewBox="0 0 32 32" fill="none"><defs><linearGradient id="zg" x1="0" y1="0" x2="32" y2="32"><stop offset="0%" stop-color="#F5C842"/><stop offset="100%" stop-color="#E8A817"/></linearGradient></defs><rect width="32" height="32" rx="8" fill="url(#zg)"/><path d="M9 11h14l-14 10h14" stroke="#2D1F0E" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round"/></svg>
    ZVault
  </div>
  <div class="nav-links">
    <a href="{{DASHBOARD_URL}}">Dashboard</a>
    <a href="{{DOCS_URL}}">Docs</a>
    <a href="https://github.com/VanitasCaesar1/ZVault">GitHub</a>
    <a href="{{DASHBOARD_URL}}/init" class="nav-pill">Get Started</a>
  </div>
</nav>
<section class="hero">
  <h1>Your secrets deserve<br/>a <span>proper vault</span></h1>
  <p>A secure, high-performance secrets manager built entirely in Rust. AES-256-GCM encryption, Shamir unseal, zero unsafe crypto.</p>
  <div class="hero-actions">
    <a href="{{DASHBOARD_URL}}/init" class="btn btn-primary">Initialize Vault</a>
    <a href="https://github.com/VanitasCaesar1/ZVault" class="btn btn-outline">View Source</a>
  </div>
</section>
<section class="features">
  <div class="feature"><div class="feature-icon"><svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><rect x="3" y="11" width="18" height="11" rx="2"/><path d="M7 11V7a5 5 0 0110 0v4"/></svg></div><h3>Shamir Unseal</h3><p>Split the root key into shares. No single operator can unseal alone.</p></div>
  <div class="feature"><div class="feature-icon"><svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M12 22s8-4 8-10V5l-8-3-8 3v7c0 6 8 10 8 10z"/></svg></div><h3>Encryption Barrier</h3><p>All data at rest is AES-256-GCM encrypted. Fresh nonces on every write.</p></div>
  <div class="feature"><div class="feature-icon"><svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M14 2H6a2 2 0 00-2 2v16a2 2 0 002 2h12a2 2 0 002-2V8z"/><path d="M14 2v6h6"/><path d="M16 13H8m8 4H8m2-8H8"/></svg></div><h3>Audit Logging</h3><p>Every operation logged before response. Fail-closed. Sensitive fields HMAC'd.</p></div>
  <div class="feature"><div class="feature-icon"><svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M21 2l-2 2m-7.61 7.61a5.5 5.5 0 11-7.778 7.778 5.5 5.5 0 017.777-7.777zm0 0L15.5 7.5m0 0l3 3L22 7l-3-3m-3.5 3.5L19 4"/></svg></div><h3>Dynamic Secrets</h3><p>Short-lived credentials on demand. Automatic lease tracking and revocation.</p></div>
  <div class="feature"><div class="feature-icon"><svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><circle cx="12" cy="12" r="10"/><path d="M12 6v6l4 2"/></svg></div><h3>Lease Management</h3><p>Every secret has a TTL. Leases tracked, renewable, auto-revoked on expiry.</p></div>
  <div class="feature"><div class="feature-icon"><svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><rect x="3" y="3" width="7" height="7" rx="1"/><rect x="14" y="3" width="7" height="7" rx="1"/><rect x="3" y="14" width="7" height="7" rx="1"/><rect x="14" y="14" width="7" height="7" rx="1"/></svg></div><h3>Pluggable Engines</h3><p>KV, Transit, PKI, Database. Mount at any path. Custom engines via traits.</p></div>
</section>
<section class="cta">
  <h2>Ready to lock down your secrets?</h2>
  <p>Initialize your vault in under a minute. No external dependencies required.</p>
  <a href="{{DASHBOARD_URL}}/init" class="btn btn-primary">Initialize Vault</a>
</section>
<footer class="footer">
  <span>ZVault v0.1.0 &mdash; MIT / Apache-2.0</span>
  <span>Built with Rust, Axum &amp; RustCrypto</span>
</footer>
</body></html>
"##;

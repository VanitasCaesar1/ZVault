//! Landing page and web UI routes.
//!
//! Serves the marketing landing page at `/` and the application dashboard
//! at `/app/*`. Both are server-rendered HTML with inline CSS — no JS
//! framework required. Themed with a warm golden "treasure chest" aesthetic
//! using Plus Jakarta Sans.

use axum::response::Html;
use axum::routing::get;
use axum::Router;
use std::sync::Arc;

use super::dashboard::{
    AUDIT_CONTENT, AUTH_CONTENT, DASHBOARD_CONTENT, INIT_CONTENT, LEASES_CONTENT,
    POLICIES_CONTENT, SECRETS_CONTENT, SIDEBAR_SCRIPT, UNSEAL_CONTENT,
};
use crate::state::AppState;

/// Build the UI router.
pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/", get(landing_page))
        .route("/app", get(dashboard_page))
        .route("/app/init", get(init_page))
        .route("/app/unseal", get(unseal_page))
        .route("/app/secrets", get(secrets_page))
        .route("/app/policies", get(policies_page))
        .route("/app/audit", get(audit_page))
        .route("/app/leases", get(leases_page))
        .route("/app/auth", get(auth_page))
}

async fn landing_page() -> Html<String> {
    let mut html = String::with_capacity(32768);
    html.push_str(LANDING_CSS);
    html.push_str(LANDING_BODY);
    Html(html)
}

async fn dashboard_page() -> Html<String> {
    Html(app_shell("Dashboard", "dashboard", DASHBOARD_CONTENT))
}

async fn init_page() -> Html<String> {
    Html(app_shell("Initialize Vault", "init", INIT_CONTENT))
}

async fn unseal_page() -> Html<String> {
    Html(app_shell("Unseal Vault", "unseal", UNSEAL_CONTENT))
}

async fn secrets_page() -> Html<String> {
    Html(app_shell("Secrets", "secrets", SECRETS_CONTENT))
}

async fn policies_page() -> Html<String> {
    Html(app_shell("Policies", "policies", POLICIES_CONTENT))
}

async fn audit_page() -> Html<String> {
    Html(app_shell("Audit Log", "audit", AUDIT_CONTENT))
}

async fn leases_page() -> Html<String> {
    Html(app_shell("Leases", "leases", LEASES_CONTENT))
}

async fn auth_page() -> Html<String> {
    Html(app_shell("Auth Methods", "auth", AUTH_CONTENT))
}

/// Render the app shell with sidebar, topbar, and page content.
fn app_shell(title: &str, active: &str, content: &str) -> String {
    let nav_item = |href: &str, id: &str, icon: &str, label: &str| -> String {
        let class = if active == id {
            "sidebar-link active"
        } else {
            "sidebar-link"
        };
        let mut s = String::with_capacity(256);
        s.push_str("<a href=\"");
        s.push_str(href);
        s.push_str("\" class=\"");
        s.push_str(class);
        s.push_str("\"><svg viewBox=\"0 0 24 24\" fill=\"none\" stroke=\"currentColor\" stroke-width=\"2\">");
        s.push_str(icon);
        s.push_str("</svg>");
        s.push_str(label);
        s.push_str("</a>");
        s
    };

    let mut html = String::with_capacity(16384);
    html.push_str(APP_CSS);
    html.push_str("<body>\n");

    // Sidebar
    html.push_str(r##"<aside class="sidebar"><div class="sidebar-logo"><svg viewBox="0 0 28 28" fill="none"><rect width="28" height="28" rx="6" fill="#D4A843"/><path d="M8 14l4 4 8-8" stroke="#3D2B1F" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round"/></svg>VaultRS</div><nav class="sidebar-nav">"##);
    html.push_str(r##"<div class="sidebar-section"><div class="sidebar-section-label">System</div>"##);
    html.push_str(&nav_item("/app","dashboard",r##"<rect x="3" y="3" width="7" height="7" rx="1"/><rect x="14" y="3" width="7" height="7" rx="1"/><rect x="3" y="14" width="7" height="7" rx="1"/><rect x="14" y="14" width="7" height="7" rx="1"/>"##,"Dashboard"));
    html.push_str(&nav_item("/app/init","init",r##"<path d="M12 2v4m0 12v4M2 12h4m12 0h4"/><circle cx="12" cy="12" r="3"/>"##,"Initialize"));
    html.push_str(&nav_item("/app/unseal","unseal",r##"<rect x="3" y="11" width="18" height="11" rx="2"/><path d="M7 11V7a5 5 0 0110 0v4"/>"##,"Unseal"));
    html.push_str("</div>");
    html.push_str(r##"<div class="sidebar-section"><div class="sidebar-section-label">Manage</div>"##);
    html.push_str(&nav_item("/app/secrets","secrets",r##"<path d="M21 2l-2 2m-7.61 7.61a5.5 5.5 0 11-7.778 7.778 5.5 5.5 0 017.777-7.777zm0 0L15.5 7.5m0 0l3 3L22 7l-3-3m-3.5 3.5L19 4"/>"##,"Secrets"));
    html.push_str(&nav_item("/app/policies","policies",r##"<path d="M12 22s8-4 8-10V5l-8-3-8 3v7c0 6 8 10 8 10z"/>"##,"Policies"));
    html.push_str(&nav_item("/app/leases","leases",r##"<circle cx="12" cy="12" r="10"/><path d="M12 6v6l4 2"/>"##,"Leases"));
    html.push_str(&nav_item("/app/audit","audit",r##"<path d="M14 2H6a2 2 0 00-2 2v16a2 2 0 002 2h12a2 2 0 002-2V8z"/><path d="M14 2v6h6"/><path d="M16 13H8m8 4H8m2-8H8"/>"##,"Audit Log"));
    html.push_str(&nav_item("/app/auth","auth",r##"<path d="M16 21v-2a4 4 0 00-4-4H6a4 4 0 00-4 4v2"/><circle cx="9" cy="7" r="4"/><path d="M22 21v-2a4 4 0 00-3-3.87M16 3.13a4 4 0 010 7.75"/>"##,"Auth Methods"));
    html.push_str("</div>");
    html.push_str(r##"</nav><div class="sidebar-footer"><a href="/docs" style="color:#A69274;text-decoration:none;font-size:12px;display:block;margin-bottom:6px">Documentation</a>VaultRS v0.1.0</div></aside>"##);

    // Main content area
    html.push_str(r##"<div class="main"><header class="topbar"><div class="topbar-title">"##);
    html.push_str(title);
    html.push_str(r##"</div><div class="topbar-actions"><div class="topbar-status"><span class="status-dot sealed" id="seal-dot"></span><span id="seal-text">Sealed</span></div><a href="/" class="btn btn-secondary btn-sm">Back to Site</a></div></header><div class="content">"##);
    html.push_str(content);
    html.push_str("</div></div>\n");
    html.push_str(SIDEBAR_SCRIPT);
    html.push_str("\n</body>\n</html>");
    html
}

/// CSS for the dashboard app shell — warm golden chest theme.
const APP_CSS: &str = r##"<!DOCTYPE html>
<html lang="en"><head><meta charset="utf-8"/><meta name="viewport" content="width=device-width,initial-scale=1"/><title>VaultRS</title>
<link rel="preconnect" href="https://fonts.googleapis.com"/>
<link rel="preconnect" href="https://fonts.gstatic.com" crossorigin/>
<link href="https://fonts.googleapis.com/css2?family=Plus+Jakarta+Sans:wght@400;500;600;700;800&display=swap" rel="stylesheet"/>
<style>
*,*::before,*::after{box-sizing:border-box;margin:0;padding:0}
:root{
  --bg:#FDF6E3;
  --bg-warm:#F9EDCC;
  --surface:#FFFDF7;
  --surface-warm:#FFF8E7;
  --border:#E8D5A3;
  --border-light:#F0E4C4;
  --text:#3D2B1F;
  --text-muted:#8B7355;
  --text-light:#A69274;
  --primary:#D4A843;
  --primary-hover:#C49A35;
  --primary-light:#F5E6B8;
  --primary-glow:rgba(212,168,67,.15);
  --sidebar-bg:#2C1E12;
  --sidebar-text:#A69274;
  --sidebar-active:#F5E6B8;
  --sidebar-active-bg:rgba(212,168,67,.2);
  --success:#6B8E4E;
  --success-light:#E8F0DE;
  --warning:#D4A843;
  --warning-light:#FFF3D0;
  --danger:#C25B4A;
  --danger-light:#FBEAE7;
  --info:#7B8EA8;
  --info-light:#E8EDF2;
  --accent:#B8860B;
  --radius:12px;
  --radius-sm:8px;
  --radius-lg:16px;
  --shadow:0 1px 3px rgba(61,43,31,.06),0 1px 2px rgba(61,43,31,.04);
  --shadow-md:0 4px 12px rgba(61,43,31,.08);
  --shadow-lg:0 8px 24px rgba(61,43,31,.1);
  --mono:'JetBrains Mono','SF Mono',Monaco,Consolas,monospace;
  --font:'Plus Jakarta Sans',-apple-system,BlinkMacSystemFont,'Segoe UI',sans-serif
}
body{font-family:var(--font);background:var(--bg);color:var(--text);display:flex;min-height:100vh;line-height:1.6;-webkit-font-smoothing:antialiased}
.sidebar{width:250px;background:var(--sidebar-bg);color:var(--sidebar-text);display:flex;flex-direction:column;position:fixed;top:0;left:0;bottom:0;z-index:100;overflow-y:auto}
.sidebar-logo{display:flex;align-items:center;gap:10px;padding:24px 20px 28px;font-size:18px;font-weight:800;color:#F5E6B8;letter-spacing:-.3px}
.sidebar-logo svg{width:28px;height:28px;flex-shrink:0}
.sidebar-nav{flex:1;padding:0 12px}
.sidebar-section{margin-bottom:24px}
.sidebar-section-label{font-size:11px;font-weight:700;text-transform:uppercase;letter-spacing:1px;color:#6B5740;padding:0 10px;margin-bottom:8px}
.sidebar-link{display:flex;align-items:center;gap:10px;padding:9px 12px;border-radius:var(--radius-sm);color:var(--sidebar-text);text-decoration:none;font-size:13.5px;font-weight:500;transition:all .15s}
.sidebar-link:hover{color:#D4C4A8;background:rgba(255,255,255,.05)}
.sidebar-link.active{color:var(--sidebar-active);background:var(--sidebar-active-bg);font-weight:600}
.sidebar-link svg{width:18px;height:18px;flex-shrink:0;opacity:.7}
.sidebar-link.active svg{opacity:1}
.sidebar-footer{padding:16px 20px;font-size:11px;color:#6B5740;border-top:1px solid rgba(255,255,255,.06)}
.main{margin-left:250px;flex:1;min-height:100vh}
.topbar{display:flex;align-items:center;justify-content:space-between;padding:18px 32px;background:var(--surface);border-bottom:1px solid var(--border-light)}
.topbar-title{font-size:20px;font-weight:700;color:var(--text);letter-spacing:-.3px}
.topbar-actions{display:flex;align-items:center;gap:12px}
.topbar-status{display:flex;align-items:center;gap:6px;font-size:13px;color:var(--text-muted);font-weight:600}
.status-dot{width:8px;height:8px;border-radius:50%;display:inline-block}
.status-dot.sealed{background:var(--warning)}.status-dot.unsealed{background:var(--success)}.status-dot.uninitialized{background:var(--danger)}
.content{padding:28px 32px}
.btn{display:inline-flex;align-items:center;justify-content:center;gap:6px;padding:9px 18px;border-radius:var(--radius-sm);font-size:13px;font-weight:600;font-family:var(--font);text-decoration:none;border:none;cursor:pointer;transition:all .15s}
.btn-primary{background:var(--primary);color:#3D2B1F}.btn-primary:hover{background:var(--primary-hover);box-shadow:0 2px 8px rgba(212,168,67,.3)}
.btn-secondary{background:var(--surface);color:var(--text);border:1px solid var(--border)}.btn-secondary:hover{background:var(--bg-warm);border-color:var(--primary)}
.btn-danger{background:var(--danger);color:#fff}.btn-danger:hover{background:#B04E3E}
.btn-sm{padding:6px 12px;font-size:12px}
.stat-grid{display:grid;grid-template-columns:repeat(auto-fit,minmax(200px,1fr));gap:16px;margin-bottom:28px}
.stat-card{background:var(--surface);border:1px solid var(--border-light);border-radius:var(--radius);padding:22px;box-shadow:var(--shadow);transition:box-shadow .2s}
.stat-card:hover{box-shadow:var(--shadow-md)}
.stat-card-label{font-size:11px;font-weight:700;text-transform:uppercase;letter-spacing:.6px;color:var(--text-muted);margin-bottom:8px}
.stat-card-value{font-size:30px;font-weight:800;color:var(--text);line-height:1.1;letter-spacing:-.5px}
.stat-card-value.primary{color:var(--primary)}.stat-card-value.accent{color:var(--accent)}.stat-card-value.warning{color:var(--warning)}.stat-card-value.danger{color:var(--danger)}.stat-card-value.success{color:var(--success)}
.stat-card-sub{font-size:12px;color:var(--text-light);margin-top:6px}
.card{background:var(--surface);border:1px solid var(--border-light);border-radius:var(--radius);box-shadow:var(--shadow);margin-bottom:20px;overflow:hidden}
.card-header{display:flex;align-items:center;justify-content:space-between;padding:16px 20px;border-bottom:1px solid var(--border-light);background:var(--surface-warm)}
.card-title{font-size:14px;font-weight:700;color:var(--text)}
.card-body{padding:20px}
.table{width:100%;border-collapse:collapse;font-size:13px}
.table th{text-align:left;font-weight:700;color:var(--text-muted);font-size:11px;text-transform:uppercase;letter-spacing:.6px;padding:10px 16px;border-bottom:1px solid var(--border-light);background:var(--surface-warm)}
.table td{padding:12px 16px;border-bottom:1px solid var(--border-light);color:var(--text)}
.table tr:last-child td{border-bottom:none}.table tr:hover{background:var(--primary-glow)}
.badge{display:inline-block;padding:3px 10px;border-radius:20px;font-size:11px;font-weight:700;letter-spacing:.2px}
.badge-success{background:var(--success-light);color:#4A6B33}
.badge-warning{background:var(--warning-light);color:#8B6914}
.badge-danger{background:var(--danger-light);color:#943D2E}
.badge-info{background:var(--info-light);color:#556B82}
.badge-primary{background:var(--primary-light);color:#8B6914}
.badge-muted{background:#F0E8D8;color:var(--text-muted)}
.form-group{margin-bottom:18px}
.form-label{display:block;font-size:13px;font-weight:700;margin-bottom:6px;color:var(--text)}
.form-input{width:100%;padding:10px 14px;border:1px solid var(--border);border-radius:var(--radius-sm);font-size:13px;font-family:var(--font);background:var(--surface);color:var(--text);transition:all .15s}
.form-input:focus{outline:none;border-color:var(--primary);box-shadow:0 0 0 3px var(--primary-glow);background:#fff}
.form-input.mono{font-family:var(--mono);font-size:12px}
.form-hint{font-size:12px;color:var(--text-light);margin-top:5px}
.code-block{background:var(--sidebar-bg);color:#E8D5A3;padding:16px;border-radius:var(--radius-sm);font-family:var(--mono);font-size:12px;overflow-x:auto;line-height:1.7;white-space:pre-wrap;word-break:break-all;margin-bottom:8px}
.code-block .accent{color:#D4A843}.code-block .key{color:#6B8E4E}
.wizard{max-width:640px}
.wizard-step{background:var(--surface);border:1px solid var(--border-light);border-radius:var(--radius);padding:28px;margin-bottom:20px;box-shadow:var(--shadow)}
.wizard-step h3{font-size:17px;font-weight:800;margin-bottom:8px;letter-spacing:-.2px}
.wizard-step p{font-size:14px;color:var(--text-muted);line-height:1.7;margin-bottom:16px}
.wizard-step-num{display:inline-flex;align-items:center;justify-content:center;width:34px;height:34px;border-radius:50%;background:var(--primary);color:#3D2B1F;font-size:14px;font-weight:800;margin-bottom:14px}
code{font-family:var(--mono);font-size:12px;background:var(--primary-glow);color:var(--accent);padding:2px 6px;border-radius:4px}
@media(max-width:900px){.main{margin-left:0}.sidebar{display:none}}
</style></head>
"##;

/// CSS and HTML head for the marketing landing page at `/`.
const LANDING_CSS: &str = r##"<!DOCTYPE html>
<html lang="en"><head><meta charset="utf-8"/><meta name="viewport" content="width=device-width,initial-scale=1"/>
<title>VaultRS &mdash; Secrets Management</title>
<link rel="preconnect" href="https://fonts.googleapis.com"/>
<link rel="preconnect" href="https://fonts.gstatic.com" crossorigin/>
<link href="https://fonts.googleapis.com/css2?family=Plus+Jakarta+Sans:wght@400;500;600;700;800&display=swap" rel="stylesheet"/>
<style>
*,*::before,*::after{box-sizing:border-box;margin:0;padding:0}
:root{
  --bg:#2C1E12;
  --surface:#FFFDF7;
  --text:#3D2B1F;
  --text-muted:#A69274;
  --primary:#D4A843;
  --primary-hover:#C49A35;
  --accent:#B8860B;
  --gold-glow:rgba(212,168,67,.15);
  --mono:'JetBrains Mono','SF Mono',Monaco,Consolas,monospace;
  --font:'Plus Jakarta Sans',-apple-system,BlinkMacSystemFont,'Segoe UI',sans-serif
}
body{font-family:var(--font);background:var(--bg);color:#F5E6B8;line-height:1.6;-webkit-font-smoothing:antialiased}
a{color:inherit;text-decoration:none}
.nav{display:flex;align-items:center;justify-content:space-between;max-width:1100px;margin:0 auto;padding:24px}
.nav-logo{display:flex;align-items:center;gap:10px;font-size:18px;font-weight:800;letter-spacing:-.3px;color:#F5E6B8}
.nav-logo svg{width:28px;height:28px}
.nav-links{display:flex;align-items:center;gap:24px;font-size:14px;font-weight:600}
.nav-links a{color:#A69274;transition:color .15s}.nav-links a:hover{color:#F5E6B8}
.btn{display:inline-flex;align-items:center;justify-content:center;gap:6px;padding:10px 22px;border-radius:10px;font-size:14px;font-weight:700;font-family:var(--font);border:none;cursor:pointer;transition:all .2s}
.btn-primary{background:var(--primary);color:#3D2B1F}.btn-primary:hover{background:var(--primary-hover);box-shadow:0 4px 20px rgba(212,168,67,.35)}
.btn-outline{background:transparent;color:#F5E6B8;border:1.5px solid rgba(245,230,184,.25)}.btn-outline:hover{border-color:#F5E6B8;background:rgba(245,230,184,.06)}
.btn-sm{padding:8px 16px;font-size:13px}
.hero{text-align:center;max-width:800px;margin:0 auto;padding:100px 24px 70px}
.hero h1{font-size:56px;font-weight:800;line-height:1.08;letter-spacing:-2px;margin-bottom:22px;color:#FFFDF7}
.hero h1 span{background:linear-gradient(135deg,#D4A843,#F5E6B8,#B8860B);-webkit-background-clip:text;-webkit-text-fill-color:transparent;background-clip:text}
.hero p{font-size:18px;color:#A69274;max-width:540px;margin:0 auto 36px;line-height:1.7}
.hero-actions{display:flex;gap:14px;justify-content:center}
.features{max-width:1100px;margin:0 auto;padding:40px 24px 80px;display:grid;grid-template-columns:repeat(3,1fr);gap:20px}
.feature{background:rgba(245,230,184,.04);border:1px solid rgba(245,230,184,.08);border-radius:16px;padding:30px;transition:all .2s}
.feature:hover{background:rgba(245,230,184,.07);border-color:rgba(212,168,67,.2);box-shadow:0 4px 20px rgba(212,168,67,.08)}
.feature-icon{width:44px;height:44px;border-radius:12px;display:flex;align-items:center;justify-content:center;margin-bottom:18px;background:rgba(212,168,67,.12)}
.feature-icon svg{width:22px;height:22px;stroke:#D4A843}
.feature h3{font-size:16px;font-weight:700;margin-bottom:8px;color:#F5E6B8}
.feature p{font-size:14px;color:#8B7355;line-height:1.65}
.cta{text-align:center;padding:60px 24px 80px}
.cta h2{font-size:34px;font-weight:800;margin-bottom:14px;color:#FFFDF7;letter-spacing:-.5px}
.cta p{font-size:16px;color:#A69274;margin-bottom:28px}
.footer{border-top:1px solid rgba(245,230,184,.06);max-width:1100px;margin:0 auto;padding:24px;display:flex;justify-content:space-between;font-size:13px;color:#6B5740}
@media(max-width:768px){.hero h1{font-size:34px}.features{grid-template-columns:1fr}.nav-links{display:none}}
</style></head>
"##;

/// HTML body for the marketing landing page.
const LANDING_BODY: &str = r##"<body>
<nav class="nav">
  <div class="nav-logo">
    <svg viewBox="0 0 28 28" fill="none"><rect width="28" height="28" rx="6" fill="#D4A843"/><path d="M8 14l4 4 8-8" stroke="#3D2B1F" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round"/></svg>
    VaultRS
  </div>
  <div class="nav-links">
    <a href="/app">Dashboard</a>
    <a href="/docs">Docs</a>
    <a href="https://github.com/vaultrs/vaultrs">GitHub</a>
    <a href="/app/init" class="btn btn-sm btn-outline">Get Started</a>
  </div>
</nav>

<section class="hero">
  <h1>Your secrets deserve<br/>a <span>proper vault</span></h1>
  <p>A secure, high-performance secrets manager built entirely in Rust. AES-256-GCM encryption, Shamir unseal, zero unsafe crypto. Your treasure, locked tight.</p>
  <div class="hero-actions">
    <a href="/app/init" class="btn btn-primary">Initialize Vault</a>
    <a href="https://github.com/vaultrs/vaultrs" class="btn btn-outline">View Source</a>
  </div>
</section>

<section class="features">
  <div class="feature">
    <div class="feature-icon"><svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><rect x="3" y="11" width="18" height="11" rx="2"/><path d="M7 11V7a5 5 0 0110 0v4"/></svg></div>
    <h3>Shamir Unseal</h3>
    <p>Split the root key into shares using Shamir's Secret Sharing. No single operator can unseal alone.</p>
  </div>
  <div class="feature">
    <div class="feature-icon"><svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M12 22s8-4 8-10V5l-8-3-8 3v7c0 6 8 10 8 10z"/></svg></div>
    <h3>Encryption Barrier</h3>
    <p>All data at rest is AES-256-GCM encrypted. Storage backends never see plaintext. Fresh nonces on every write.</p>
  </div>
  <div class="feature">
    <div class="feature-icon"><svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M14 2H6a2 2 0 00-2 2v16a2 2 0 002 2h12a2 2 0 002-2V8z"/><path d="M14 2v6h6"/><path d="M16 13H8m8 4H8m2-8H8"/></svg></div>
    <h3>Audit Logging</h3>
    <p>Every operation is logged before the response is sent. Fail-closed design. Sensitive fields are HMAC'd.</p>
  </div>
  <div class="feature">
    <div class="feature-icon"><svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M21 2l-2 2m-7.61 7.61a5.5 5.5 0 11-7.778 7.778 5.5 5.5 0 017.777-7.777zm0 0L15.5 7.5m0 0l3 3L22 7l-3-3m-3.5 3.5L19 4"/></svg></div>
    <h3>Dynamic Secrets</h3>
    <p>Generate short-lived credentials on demand. Automatic lease tracking and revocation when TTL expires.</p>
  </div>
  <div class="feature">
    <div class="feature-icon"><svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><circle cx="12" cy="12" r="10"/><path d="M12 6v6l4 2"/></svg></div>
    <h3>Lease Management</h3>
    <p>Every secret has a TTL. Leases are tracked, renewable, and automatically revoked on expiry.</p>
  </div>
  <div class="feature">
    <div class="feature-icon"><svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><rect x="3" y="3" width="7" height="7" rx="1"/><rect x="14" y="3" width="7" height="7" rx="1"/><rect x="3" y="14" width="7" height="7" rx="1"/><rect x="14" y="14" width="7" height="7" rx="1"/></svg></div>
    <h3>Pluggable Engines</h3>
    <p>KV, Transit, PKI, Database. Mount engines at any path. Add custom engines via the trait interface.</p>
  </div>
</section>

<section class="cta">
  <h2>Ready to lock down your secrets?</h2>
  <p>Initialize your vault in under a minute. No external dependencies required.</p>
  <a href="/app/init" class="btn btn-primary">Initialize Vault</a>
</section>

<footer class="footer">
  <span>VaultRS v0.1.0 &mdash; MIT / Apache-2.0</span>
  <span>Built with Rust, Axum &amp; RustCrypto</span>
</footer>
</body></html>
"##;

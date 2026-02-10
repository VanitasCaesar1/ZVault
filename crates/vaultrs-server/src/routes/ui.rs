//! Landing page and web UI routes.
//!
//! Serves the marketing landing page at `/` and the application dashboard
//! at `/app/*`. Both are server-rendered HTML with inline CSS — no JS
//! framework required. Themed with a warm amber Crextio-inspired aesthetic
//! using Plus Jakarta Sans — glassmorphism cards, bento grid, pill nav.

use axum::response::Html;
use axum::routing::get;
use axum::Router;
use std::sync::Arc;

use super::dashboard::{
    AUDIT_CONTENT, AUTH_CONTENT, DASHBOARD_CONTENT, INIT_CONTENT, LEASES_CONTENT,
    LOGIN_CONTENT, POLICIES_CONTENT, SECRETS_CONTENT, SIDEBAR_SCRIPT, UNSEAL_CONTENT,
};
use crate::state::AppState;

/// Build the UI router.
pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/", get(landing_page))
        .route("/app/login", get(login_page))
        .route("/app/logout", get(logout_page))
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

async fn login_page() -> Html<String> {
    Html(login_shell(LOGIN_CONTENT))
}

async fn logout_page() -> Html<String> {
    Html(login_shell(LOGOUT_SCRIPT))
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

/// Script that clears the token cookie and redirects to login.
const LOGOUT_SCRIPT: &str = r##"
<script>
document.cookie='zvault-token=;path=/;max-age=0';
window.location.href='/app/login';
</script>
"##;

/// Auth gate script injected into every `/app/*` page (except login).
/// Checks for the `zvault-token` cookie and validates it against the API.
/// Redirects to `/app/login` if missing or invalid.
const AUTH_GATE_SCRIPT: &str = r##"
<script>
(function(){
  function getCookie(n){var m=document.cookie.match(new RegExp('(?:^|; )'+n+'=([^;]*)'));return m?decodeURIComponent(m[1]):null}
  var token=getCookie('zvault-token');
  if(!token){window.location.href='/app/login';return}
  fetch('/v1/auth/token/lookup-self',{method:'POST',headers:{'X-Vault-Token':token,'Content-Type':'application/json'},body:'{}'})
    .then(function(r){if(!r.ok)throw new Error('invalid');return r.json()})
    .catch(function(){document.cookie='zvault-token=;path=/;max-age=0';window.location.href='/app/login'});
})();
</script>
"##;

/// Render the login page shell (no sidebar, standalone page).
fn login_shell(content: &str) -> String {
    let mut html = String::with_capacity(8192);
    html.push_str(APP_CSS);
    html.push_str("<body style=\"display:flex;align-items:center;justify-content:center;min-height:100vh;background:var(--bg)\">\n");
    html.push_str(content);
    html.push_str("\n</body>\n</html>");
    html
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
    html.push_str(r##"<aside class="sidebar"><div class="sidebar-logo"><svg viewBox="0 0 32 32" fill="none"><defs><linearGradient id="zg" x1="0" y1="0" x2="32" y2="32"><stop offset="0%" stop-color="#F5C842"/><stop offset="100%" stop-color="#E8A817"/></linearGradient></defs><rect width="32" height="32" rx="8" fill="url(#zg)"/><path d="M9 11h14l-14 10h14" stroke="#2D1F0E" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round"/></svg>ZVault</div><nav class="sidebar-nav">"##);
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
    html.push_str(r##"</nav><div class="sidebar-footer"><a href="/docs" class="sidebar-docs-link">Documentation</a><a href="/app/logout" class="sidebar-docs-link" style="color:#E74C3C">Sign Out</a>ZVault v0.1.0</div></aside>"##);

    // Main content area
    html.push_str(r##"<div class="main"><header class="topbar"><div class="topbar-title">"##);
    html.push_str(title);
    html.push_str(r##"</div><div class="topbar-actions"><div class="topbar-status"><span class="status-dot sealed" id="seal-dot"></span><span id="seal-text">Sealed</span></div><a href="/app/logout" class="btn btn-danger btn-sm">Sign Out</a><a href="/" class="btn btn-secondary btn-sm">Back to Site</a></div></header><div class="content">"##);
    html.push_str(content);
    html.push_str("</div></div>\n");
    html.push_str(SIDEBAR_SCRIPT);
    html.push_str(AUTH_GATE_SCRIPT);
    html.push_str("\n</body>\n</html>");
    html
}

/// CSS for the dashboard app shell — Crextio-inspired warm amber glassmorphism.
const APP_CSS: &str = r##"<!DOCTYPE html>
<html lang="en"><head><meta charset="utf-8"/><meta name="viewport" content="width=device-width,initial-scale=1"/><title>ZVault</title>
<link rel="preconnect" href="https://fonts.googleapis.com"/>
<link rel="preconnect" href="https://fonts.gstatic.com" crossorigin/>
<link href="https://fonts.googleapis.com/css2?family=Plus+Jakarta+Sans:wght@400;500;600;700;800&display=swap" rel="stylesheet"/>
<style>
*,*::before,*::after{box-sizing:border-box;margin:0;padding:0}
:root{
  --bg:linear-gradient(145deg,#FEF3D0 0%,#FAEAB5 30%,#F5DFA0 60%,#EDD48C 100%);
  --bg-flat:#FBF0C8;
  --surface:rgba(255,255,253,.72);
  --surface-solid:#FFFDF7;
  --surface-warm:rgba(255,248,231,.8);
  --glass:rgba(255,255,255,.45);
  --glass-border:rgba(255,255,255,.6);
  --glass-shadow:0 8px 32px rgba(180,140,50,.1);
  --border:rgba(212,168,67,.2);
  --border-light:rgba(212,168,67,.12);
  --text:#2D1F0E;
  --text-muted:#7A6543;
  --text-light:#A69274;
  --primary:#E8A817;
  --primary-hover:#D49A0F;
  --primary-light:#FFF0C2;
  --primary-glow:rgba(232,168,23,.12);
  --sidebar-bg:#1E1610;
  --sidebar-text:#A69274;
  --sidebar-active:#F5E6B8;
  --sidebar-active-bg:rgba(232,168,23,.18);
  --success:#4CAF50;
  --success-light:rgba(76,175,80,.12);
  --warning:#F5A623;
  --warning-light:rgba(245,166,35,.12);
  --danger:#E74C3C;
  --danger-light:rgba(231,76,60,.1);
  --info:#5B9BD5;
  --info-light:rgba(91,155,213,.1);
  --accent:#D4A843;
  --dark-card:#1E1610;
  --dark-card-text:#F5E6B8;
  --radius:16px;
  --radius-sm:10px;
  --radius-lg:20px;
  --radius-pill:50px;
  --shadow-sm:0 2px 8px rgba(45,31,14,.06);
  --shadow:0 4px 16px rgba(45,31,14,.08);
  --shadow-lg:0 12px 40px rgba(45,31,14,.12);
  --mono:'JetBrains Mono','SF Mono',Monaco,Consolas,monospace;
  --font:'Plus Jakarta Sans',-apple-system,BlinkMacSystemFont,'Segoe UI',sans-serif
}
body{font-family:var(--font);background:var(--bg);background-attachment:fixed;color:var(--text);display:flex;min-height:100vh;line-height:1.6;-webkit-font-smoothing:antialiased}
.sidebar{width:260px;background:var(--sidebar-bg);color:var(--sidebar-text);display:flex;flex-direction:column;position:fixed;top:0;left:0;bottom:0;z-index:100;overflow-y:auto}
.sidebar-logo{display:flex;align-items:center;gap:12px;padding:28px 24px 32px;font-size:20px;font-weight:800;color:#F5E6B8;letter-spacing:-.3px}
.sidebar-logo svg{width:32px;height:32px;flex-shrink:0}
.sidebar-nav{flex:1;padding:0 14px}
.sidebar-section{margin-bottom:28px}
.sidebar-section-label{font-size:10px;font-weight:700;text-transform:uppercase;letter-spacing:1.2px;color:#5A4A36;padding:0 12px;margin-bottom:8px}
.sidebar-link{display:flex;align-items:center;gap:11px;padding:10px 14px;border-radius:var(--radius-sm);color:var(--sidebar-text);text-decoration:none;font-size:13.5px;font-weight:500;transition:all .2s}
.sidebar-link:hover{color:#D4C4A8;background:rgba(255,255,255,.05)}
.sidebar-link.active{color:var(--sidebar-active);background:var(--sidebar-active-bg);font-weight:600}
.sidebar-link svg{width:18px;height:18px;flex-shrink:0;opacity:.6}
.sidebar-link.active svg{opacity:1}
.sidebar-footer{padding:18px 24px;font-size:11px;color:#5A4A36;border-top:1px solid rgba(255,255,255,.06)}
.sidebar-docs-link{color:#A69274;text-decoration:none;font-size:12px;display:block;margin-bottom:6px;transition:color .15s}
.sidebar-docs-link:hover{color:#F5E6B8}
.main{margin-left:260px;flex:1;min-height:100vh}
.topbar{display:flex;align-items:center;justify-content:space-between;padding:20px 36px;background:var(--glass);backdrop-filter:blur(20px);-webkit-backdrop-filter:blur(20px);border-bottom:1px solid var(--glass-border)}
.topbar-title{font-size:22px;font-weight:800;color:var(--text);letter-spacing:-.4px}
.topbar-actions{display:flex;align-items:center;gap:14px}
.topbar-status{display:flex;align-items:center;gap:7px;font-size:13px;color:var(--text-muted);font-weight:600;background:var(--surface);padding:6px 14px;border-radius:var(--radius-pill);border:1px solid var(--border-light)}
.status-dot{width:8px;height:8px;border-radius:50%;display:inline-block}
.status-dot.sealed{background:var(--warning)}.status-dot.unsealed{background:var(--success)}.status-dot.uninitialized{background:var(--danger)}
.content{padding:32px 36px}
.btn{display:inline-flex;align-items:center;justify-content:center;gap:6px;padding:10px 20px;border-radius:var(--radius-sm);font-size:13px;font-weight:600;font-family:var(--font);text-decoration:none;border:none;cursor:pointer;transition:all .2s}
.btn-primary{background:var(--primary);color:#2D1F0E;border-radius:var(--radius-pill)}.btn-primary:hover{background:var(--primary-hover);box-shadow:0 4px 20px rgba(232,168,23,.3);transform:translateY(-1px)}
.btn-secondary{background:var(--glass);backdrop-filter:blur(10px);color:var(--text);border:1px solid var(--border);border-radius:var(--radius-pill)}.btn-secondary:hover{background:var(--surface);border-color:var(--primary)}
.btn-danger{background:var(--danger);color:#fff;border-radius:var(--radius-pill)}.btn-danger:hover{background:#D04030}
.btn-sm{padding:7px 14px;font-size:12px}
.stat-grid{display:grid;grid-template-columns:repeat(auto-fit,minmax(200px,1fr));gap:18px;margin-bottom:32px}
.stat-card{background:var(--glass);backdrop-filter:blur(16px);-webkit-backdrop-filter:blur(16px);border:1px solid var(--glass-border);border-radius:var(--radius-lg);padding:24px;box-shadow:var(--glass-shadow);transition:all .25s}
.stat-card:hover{transform:translateY(-2px);box-shadow:var(--shadow-lg)}
.stat-card-label{font-size:11px;font-weight:700;text-transform:uppercase;letter-spacing:.7px;color:var(--text-muted);margin-bottom:10px}
.stat-card-value{font-size:34px;font-weight:800;color:var(--text);line-height:1.1;letter-spacing:-.5px}
.stat-card-value.primary{color:var(--primary)}.stat-card-value.accent{color:var(--accent)}.stat-card-value.warning{color:var(--warning)}.stat-card-value.danger{color:var(--danger)}.stat-card-value.success{color:var(--success)}
.stat-card-sub{font-size:12px;color:var(--text-light);margin-top:8px}
.stat-card.dark{background:var(--dark-card);border-color:rgba(255,255,255,.06);color:var(--dark-card-text)}
.stat-card.dark .stat-card-label{color:#8B7355}
.stat-card.dark .stat-card-value{color:#F5E6B8}
.stat-card.dark .stat-card-sub{color:#7A6543}
.card{background:var(--glass);backdrop-filter:blur(16px);-webkit-backdrop-filter:blur(16px);border:1px solid var(--glass-border);border-radius:var(--radius-lg);box-shadow:var(--glass-shadow);margin-bottom:22px;overflow:hidden}
.card.dark{background:var(--dark-card);border-color:rgba(255,255,255,.06)}
.card-header{display:flex;align-items:center;justify-content:space-between;padding:18px 22px;border-bottom:1px solid var(--border-light);background:rgba(255,255,255,.25)}
.card.dark .card-header{background:rgba(255,255,255,.04);border-bottom-color:rgba(255,255,255,.06)}
.card-title{font-size:15px;font-weight:700;color:var(--text)}
.card.dark .card-title{color:#F5E6B8}
.card-body{padding:22px}
.table{width:100%;border-collapse:collapse;font-size:13px}
.table th{text-align:left;font-weight:700;color:var(--text-muted);font-size:11px;text-transform:uppercase;letter-spacing:.7px;padding:11px 18px;border-bottom:1px solid var(--border-light);background:rgba(255,255,255,.2)}
.card.dark .table th{color:#8B7355;background:rgba(255,255,255,.03);border-bottom-color:rgba(255,255,255,.06)}
.table td{padding:13px 18px;border-bottom:1px solid var(--border-light);color:var(--text)}
.card.dark .table td{color:#D4C4A8;border-bottom-color:rgba(255,255,255,.04)}
.table tr:last-child td{border-bottom:none}
.table tr:hover{background:var(--primary-glow)}
.card.dark .table tr:hover{background:rgba(232,168,23,.06)}
.badge{display:inline-block;padding:4px 12px;border-radius:var(--radius-pill);font-size:11px;font-weight:700;letter-spacing:.2px}
.badge-success{background:var(--success-light);color:#2E7D32}
.badge-warning{background:var(--warning-light);color:#E65100}
.badge-danger{background:var(--danger-light);color:#C62828}
.badge-info{background:var(--info-light);color:#1565C0}
.badge-primary{background:var(--primary-glow);color:#B8860B}
.badge-muted{background:rgba(0,0,0,.06);color:var(--text-muted)}
.form-group{margin-bottom:20px}
.form-label{display:block;font-size:13px;font-weight:700;margin-bottom:7px;color:var(--text)}
.form-input{width:100%;padding:11px 16px;border:1px solid var(--border);border-radius:var(--radius-sm);font-size:13px;font-family:var(--font);background:var(--glass);backdrop-filter:blur(8px);color:var(--text);transition:all .2s}
.form-input:focus{outline:none;border-color:var(--primary);box-shadow:0 0 0 3px var(--primary-glow);background:rgba(255,255,255,.7)}
.form-input.mono{font-family:var(--mono);font-size:12px}
.form-hint{font-size:12px;color:var(--text-light);margin-top:6px}
.code-block{background:var(--dark-card);color:#E8D5A3;padding:18px;border-radius:var(--radius-sm);font-family:var(--mono);font-size:12px;overflow-x:auto;line-height:1.7;white-space:pre-wrap;word-break:break-all;margin-bottom:8px}
.code-block .accent{color:#F5C842}.code-block .key{color:#4CAF50}
.wizard{max-width:640px}
.wizard-step{background:var(--glass);backdrop-filter:blur(16px);border:1px solid var(--glass-border);border-radius:var(--radius-lg);padding:30px;margin-bottom:22px;box-shadow:var(--glass-shadow)}
.wizard-step h3{font-size:18px;font-weight:800;margin-bottom:8px;letter-spacing:-.2px}
.wizard-step p{font-size:14px;color:var(--text-muted);line-height:1.7;margin-bottom:18px}
.wizard-step-num{display:inline-flex;align-items:center;justify-content:center;width:36px;height:36px;border-radius:50%;background:linear-gradient(135deg,#F5C842,#E8A817);color:#2D1F0E;font-size:14px;font-weight:800;margin-bottom:16px;box-shadow:0 4px 12px rgba(232,168,23,.25)}
code{font-family:var(--mono);font-size:12px;background:var(--primary-glow);color:#B8860B;padding:2px 7px;border-radius:6px}
@media(max-width:900px){.main{margin-left:0}.sidebar{display:none}}
</style></head>
"##;

/// CSS and HTML head for the marketing landing page at `/`.
const LANDING_CSS: &str = r##"<!DOCTYPE html>
<html lang="en"><head><meta charset="utf-8"/><meta name="viewport" content="width=device-width,initial-scale=1"/>
<title>ZVault &mdash; Secrets Management</title>
<link rel="preconnect" href="https://fonts.googleapis.com"/>
<link rel="preconnect" href="https://fonts.gstatic.com" crossorigin/>
<link href="https://fonts.googleapis.com/css2?family=Plus+Jakarta+Sans:wght@400;500;600;700;800&display=swap" rel="stylesheet"/>
<style>
*,*::before,*::after{box-sizing:border-box;margin:0;padding:0}
:root{
  --bg:#1E1610;
  --surface:rgba(255,253,247,.06);
  --glass:rgba(255,255,255,.04);
  --glass-border:rgba(255,255,255,.08);
  --text:#F5E6B8;
  --text-muted:#A69274;
  --primary:#F5C842;
  --primary-hover:#E8B830;
  --accent:#D4A843;
  --mono:'JetBrains Mono','SF Mono',Monaco,Consolas,monospace;
  --font:'Plus Jakarta Sans',-apple-system,BlinkMacSystemFont,'Segoe UI',sans-serif
}
body{font-family:var(--font);background:var(--bg);color:var(--text);line-height:1.6;-webkit-font-smoothing:antialiased;overflow-x:hidden}
a{color:inherit;text-decoration:none}
.nav{display:flex;align-items:center;justify-content:space-between;max-width:1100px;margin:0 auto;padding:24px}
.nav-logo{display:flex;align-items:center;gap:12px;font-size:20px;font-weight:800;letter-spacing:-.3px;color:#F5E6B8}
.nav-logo svg{width:32px;height:32px}
.nav-links{display:flex;align-items:center;gap:8px}
.nav-links a{color:#A69274;transition:all .2s;font-size:14px;font-weight:600;padding:8px 16px;border-radius:50px}
.nav-links a:hover{color:#F5E6B8;background:rgba(255,255,255,.05)}
.nav-links .nav-pill{background:rgba(245,200,66,.12);color:#F5C842;border:1px solid rgba(245,200,66,.2)}
.nav-links .nav-pill:hover{background:rgba(245,200,66,.2);border-color:rgba(245,200,66,.35)}
.btn{display:inline-flex;align-items:center;justify-content:center;gap:6px;padding:12px 28px;border-radius:50px;font-size:14px;font-weight:700;font-family:var(--font);border:none;cursor:pointer;transition:all .25s}
.btn-primary{background:linear-gradient(135deg,#F5C842,#E8A817);color:#2D1F0E;box-shadow:0 4px 20px rgba(245,200,66,.2)}.btn-primary:hover{box-shadow:0 8px 32px rgba(245,200,66,.35);transform:translateY(-2px)}
.btn-outline{background:transparent;color:#F5E6B8;border:1.5px solid rgba(245,230,184,.2);border-radius:50px}.btn-outline:hover{border-color:rgba(245,230,184,.4);background:rgba(245,230,184,.05)}
.btn-sm{padding:9px 18px;font-size:13px}
.hero{text-align:center;max-width:800px;margin:0 auto;padding:120px 24px 80px;position:relative}
.hero::before{content:'';position:absolute;top:50%;left:50%;transform:translate(-50%,-50%);width:600px;height:600px;background:radial-gradient(circle,rgba(245,200,66,.08) 0%,transparent 70%);pointer-events:none}
.hero h1{font-size:60px;font-weight:800;line-height:1.06;letter-spacing:-2.5px;margin-bottom:24px;color:#FFFDF7;position:relative}
.hero h1 span{background:linear-gradient(135deg,#F5C842,#F5E6B8,#D4A843);-webkit-background-clip:text;-webkit-text-fill-color:transparent;background-clip:text}
.hero p{font-size:18px;color:#A69274;max-width:520px;margin:0 auto 40px;line-height:1.75;position:relative}
.hero-actions{display:flex;gap:14px;justify-content:center;position:relative}
.features{max-width:1100px;margin:0 auto;padding:40px 24px 80px;display:grid;grid-template-columns:repeat(3,1fr);gap:18px}
.feature{background:var(--glass);border:1px solid var(--glass-border);border-radius:20px;padding:32px;transition:all .25s;backdrop-filter:blur(8px)}
.feature:hover{background:rgba(255,255,255,.07);border-color:rgba(245,200,66,.15);box-shadow:0 8px 32px rgba(245,200,66,.06);transform:translateY(-2px)}
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
    <a href="/app">Dashboard</a>
    <a href="/docs">Docs</a>
    <a href="https://github.com/zvault/zvault">GitHub</a>
    <a href="/app/init" class="nav-pill">Get Started</a>
  </div>
</nav>

<section class="hero">
  <h1>Your secrets deserve<br/>a <span>proper vault</span></h1>
  <p>A secure, high-performance secrets manager built entirely in Rust. AES-256-GCM encryption, Shamir unseal, zero unsafe crypto. Your treasure, locked tight.</p>
  <div class="hero-actions">
    <a href="/app/init" class="btn btn-primary">Initialize Vault</a>
    <a href="https://github.com/zvault/zvault" class="btn btn-outline">View Source</a>
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
  <span>ZVault v0.1.0 &mdash; MIT / Apache-2.0</span>
  <span>Built with Rust, Axum &amp; RustCrypto</span>
</footer>
</body></html>
"##;

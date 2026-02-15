//! Built-in documentation site for `ZVault`.
//!
//! Serves a multi-page documentation site at `/docs/*` with the same warm
//! amber Crextio-inspired glassmorphism theme as the dashboard. Covers
//! getting started, architecture, API reference, CLI reference, security
//! model, and configuration.

use axum::Router;
use axum::response::Html;
use axum::routing::get;
use std::sync::Arc;

use crate::state::AppState;

/// Build the docs router.
pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/docs", get(docs_index))
        .route("/docs/getting-started", get(getting_started))
        .route("/docs/architecture", get(architecture))
        .route("/docs/api", get(api_reference))
        .route("/docs/cli", get(cli_reference))
        .route("/docs/security", get(security_model))
        .route("/docs/configuration", get(configuration))
        .route("/docs/engines", get(engines))
        .route("/docs/policies", get(policies))
}

async fn docs_index() -> Html<String> {
    Html(docs_shell("Documentation", "index", DOCS_INDEX))
}

async fn getting_started() -> Html<String> {
    Html(docs_shell(
        "Getting Started",
        "getting-started",
        GETTING_STARTED,
    ))
}

async fn architecture() -> Html<String> {
    Html(docs_shell("Architecture", "architecture", ARCHITECTURE))
}

async fn api_reference() -> Html<String> {
    Html(docs_shell("API Reference", "api", API_REFERENCE))
}

async fn cli_reference() -> Html<String> {
    Html(docs_shell("CLI Reference", "cli", CLI_REFERENCE))
}

async fn security_model() -> Html<String> {
    Html(docs_shell("Security Model", "security", SECURITY_MODEL))
}

async fn configuration() -> Html<String> {
    Html(docs_shell("Configuration", "configuration", CONFIGURATION))
}

async fn engines() -> Html<String> {
    Html(docs_shell("Secrets Engines", "engines", ENGINES))
}

async fn policies() -> Html<String> {
    Html(docs_shell("Policies & Auth", "policies", POLICIES))
}

/// Render the docs shell with sidebar navigation and page content.
fn docs_shell(title: &str, active: &str, content: &str) -> String {
    let nav_item = |href: &str, id: &str, label: &str| -> String {
        let class = if active == id {
            "docs-nav-link active"
        } else {
            "docs-nav-link"
        };
        let mut s = String::with_capacity(128);
        s.push_str("<a href=\"");
        s.push_str(href);
        s.push_str("\" class=\"");
        s.push_str(class);
        s.push_str("\">");
        s.push_str(label);
        s.push_str("</a>");
        s
    };

    let mut html = String::with_capacity(32768);
    html.push_str(DOCS_CSS);
    html.push_str("<body>\n");

    // Top nav
    html.push_str(r##"<header class="docs-header"><div class="docs-header-inner"><a href="/" class="docs-logo"><svg viewBox="0 0 32 32" fill="none"><defs><linearGradient id="zg" x1="0" y1="0" x2="32" y2="32"><stop offset="0%" stop-color="#F5C842"/><stop offset="100%" stop-color="#E8A817"/></linearGradient></defs><rect width="32" height="32" rx="8" fill="url(#zg)"/><path d="M9 11h14l-14 10h14" stroke="#2D1F0E" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round"/></svg>ZVault</a><nav class="docs-header-nav"><a href="/app">Dashboard</a><a href="/docs">Docs</a><a href="https://github.com/zvault/zvault">GitHub</a></nav></div></header>"##);

    // Sidebar
    html.push_str(
        r#"<div class="docs-layout"><aside class="docs-sidebar"><nav class="docs-sidebar-nav">"#,
    );
    html.push_str(
        r#"<div class="docs-sidebar-section"><div class="docs-sidebar-label">Overview</div>"#,
    );
    html.push_str(&nav_item("/docs", "index", "Introduction"));
    html.push_str(&nav_item(
        "/docs/getting-started",
        "getting-started",
        "Getting Started",
    ));
    html.push_str(&nav_item(
        "/docs/architecture",
        "architecture",
        "Architecture",
    ));
    html.push_str("</div>");
    html.push_str(
        r#"<div class="docs-sidebar-section"><div class="docs-sidebar-label">Reference</div>"#,
    );
    html.push_str(&nav_item("/docs/api", "api", "API Reference"));
    html.push_str(&nav_item("/docs/cli", "cli", "CLI Reference"));
    html.push_str(&nav_item(
        "/docs/configuration",
        "configuration",
        "Configuration",
    ));
    html.push_str("</div>");
    html.push_str(
        r#"<div class="docs-sidebar-section"><div class="docs-sidebar-label">Concepts</div>"#,
    );
    html.push_str(&nav_item("/docs/engines", "engines", "Secrets Engines"));
    html.push_str(&nav_item("/docs/policies", "policies", "Policies & Auth"));
    html.push_str(&nav_item("/docs/security", "security", "Security Model"));
    html.push_str("</div>");
    html.push_str("</nav></aside>");

    // Main content
    html.push_str(r#"<main class="docs-main"><div class="docs-content"><h1 class="docs-title">"#);
    html.push_str(title);
    html.push_str("</h1>");
    html.push_str(content);
    html.push_str("</div></main></div>\n</body>\n</html>");
    html
}

/// CSS for the documentation site — Crextio-inspired warm amber glassmorphism.
const DOCS_CSS: &str = r#"<!DOCTYPE html>
<html lang="en"><head><meta charset="utf-8"/><meta name="viewport" content="width=device-width,initial-scale=1"/>
<title>ZVault Docs</title>
<link rel="preconnect" href="https://fonts.googleapis.com"/>
<link rel="preconnect" href="https://fonts.gstatic.com" crossorigin/>
<link href="https://fonts.googleapis.com/css2?family=Plus+Jakarta+Sans:wght@400;500;600;700;800&display=swap" rel="stylesheet"/>
<style>
*,*::before,*::after{box-sizing:border-box;margin:0;padding:0}
:root{
  --bg:linear-gradient(145deg,#FEF3D0 0%,#FAEAB5 30%,#F5DFA0 60%,#EDD48C 100%);
  --bg-flat:#FBF0C8;
  --surface:rgba(255,255,253,.72);
  --surface-warm:rgba(255,248,231,.8);
  --glass:rgba(255,255,255,.45);
  --glass-border:rgba(255,255,255,.6);
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
  --warning:#F5A623;
  --danger:#E74C3C;
  --accent:#D4A843;
  --radius:16px;
  --radius-sm:10px;
  --shadow:0 4px 16px rgba(45,31,14,.08);
  --mono:'JetBrains Mono','SF Mono',Monaco,Consolas,monospace;
  --font:'Plus Jakarta Sans',-apple-system,BlinkMacSystemFont,'Segoe UI',sans-serif
}
body{font-family:var(--font);background:var(--bg);background-attachment:fixed;color:var(--text);line-height:1.7;-webkit-font-smoothing:antialiased}
a{color:var(--accent);text-decoration:none}a:hover{text-decoration:underline}
.docs-header{background:var(--sidebar-bg);border-bottom:1px solid rgba(245,230,184,.08);position:sticky;top:0;z-index:100}
.docs-header-inner{max-width:1200px;margin:0 auto;display:flex;align-items:center;justify-content:space-between;padding:14px 24px}
.docs-logo{display:flex;align-items:center;gap:12px;font-size:18px;font-weight:800;color:#F5E6B8;text-decoration:none}
.docs-logo svg{width:28px;height:28px}
.docs-header-nav{display:flex;gap:8px;font-size:14px;font-weight:600}
.docs-header-nav a{color:#A69274;text-decoration:none;transition:all .2s;padding:6px 14px;border-radius:50px}
.docs-header-nav a:hover{color:#F5E6B8;background:rgba(255,255,255,.05)}
.docs-layout{display:flex;max-width:1200px;margin:0 auto;min-height:calc(100vh - 56px)}
.docs-sidebar{width:240px;padding:28px 16px;border-right:1px solid var(--border-light);position:sticky;top:56px;height:calc(100vh - 56px);overflow-y:auto;flex-shrink:0}
.docs-sidebar-nav{display:flex;flex-direction:column}
.docs-sidebar-section{margin-bottom:24px}
.docs-sidebar-label{font-size:10px;font-weight:700;text-transform:uppercase;letter-spacing:1.2px;color:var(--text-light);padding:0 12px;margin-bottom:8px}
.docs-nav-link{display:block;padding:8px 14px;border-radius:var(--radius-sm);color:var(--text-muted);font-size:14px;font-weight:500;text-decoration:none;transition:all .2s}
.docs-nav-link:hover{color:var(--text);background:var(--primary-glow);text-decoration:none}
.docs-nav-link.active{color:#B8860B;background:var(--primary-glow);font-weight:700}
.docs-main{flex:1;padding:36px 48px;max-width:820px}
.docs-title{font-size:34px;font-weight:800;letter-spacing:-.5px;margin-bottom:8px;color:var(--text)}
.docs-content h2{font-size:22px;font-weight:800;margin:36px 0 12px;letter-spacing:-.3px;color:var(--text);padding-bottom:8px;border-bottom:1px solid var(--border-light)}
.docs-content h3{font-size:17px;font-weight:700;margin:28px 0 10px;color:var(--text)}
.docs-content p{margin-bottom:16px;color:var(--text);font-size:15px;line-height:1.75}
.docs-content ul,.docs-content ol{margin-bottom:16px;padding-left:24px}
.docs-content li{margin-bottom:6px;font-size:15px;line-height:1.7}
.docs-content code{font-family:var(--mono);font-size:13px;background:var(--primary-glow);color:#B8860B;padding:2px 7px;border-radius:6px}
.docs-content pre{background:#1E1610;color:#E8D5A3;padding:20px 22px;border-radius:var(--radius-sm);font-family:var(--mono);font-size:13px;overflow-x:auto;line-height:1.7;margin-bottom:20px;white-space:pre;border:1px solid rgba(245,230,184,.08)}
.docs-content pre code{background:none;color:inherit;padding:0;font-size:13px}
.docs-content table{width:100%;border-collapse:collapse;margin-bottom:20px;font-size:14px}
.docs-content th{text-align:left;font-weight:700;color:var(--text-muted);font-size:12px;text-transform:uppercase;letter-spacing:.5px;padding:10px 14px;border-bottom:2px solid var(--border);background:var(--surface-warm)}
.docs-content td{padding:10px 14px;border-bottom:1px solid var(--border-light)}
.docs-content tr:hover{background:var(--primary-glow)}
.docs-content blockquote{border-left:3px solid var(--primary);padding:12px 20px;margin:16px 0;background:var(--primary-glow);border-radius:0 var(--radius-sm) var(--radius-sm) 0;font-size:14px;color:var(--text-muted)}
.docs-content .callout{padding:16px 20px;border-radius:var(--radius-sm);margin:16px 0;font-size:14px}
.docs-content .callout-warn{background:rgba(245,166,35,.1);border:1px solid rgba(245,166,35,.2);color:#E65100}
.docs-content .callout-danger{background:rgba(231,76,60,.08);border:1px solid rgba(231,76,60,.15);color:#C62828}
.docs-content .callout-info{background:rgba(91,155,213,.08);border:1px solid rgba(91,155,213,.15);color:#1565C0}
.docs-content .callout-success{background:rgba(76,175,80,.08);border:1px solid rgba(76,175,80,.15);color:#2E7D32}
.docs-content .endpoint{display:flex;align-items:center;gap:10px;padding:10px 14px;background:var(--glass);backdrop-filter:blur(8px);border:1px solid var(--glass-border);border-radius:var(--radius-sm);margin-bottom:8px;font-family:var(--mono);font-size:13px}
.docs-content .method{font-weight:800;padding:3px 10px;border-radius:50px;font-size:11px;letter-spacing:.5px}
.docs-content .method-get{background:rgba(76,175,80,.1);color:#2E7D32}
.docs-content .method-post{background:rgba(232,168,23,.12);color:#B8860B}
.docs-content .method-delete{background:rgba(231,76,60,.08);color:#C62828}
.docs-content .method-put{background:rgba(91,155,213,.1);color:#1565C0}
@media(max-width:900px){.docs-sidebar{display:none}.docs-main{padding:24px 16px}}
</style></head>
"#;

/// Documentation index / introduction page.
const DOCS_INDEX: &str = r#"
<p>Welcome to the ZVault documentation. ZVault is a secrets management platform
written entirely in Rust, designed to keep your sensitive data encrypted at rest
and tightly controlled at runtime.</p>

<h2>Why ZVault?</h2>
<ul>
  <li><strong>Single binary</strong> — no external databases, no Consul, no etcd. Embedded storage ships with the server.</li>
  <li><strong>Encryption barrier</strong> — every byte in storage is AES-256-GCM encrypted. The storage backend never sees plaintext.</li>
  <li><strong>Shamir unseal</strong> — the root key is split into shares. No single operator can unseal alone.</li>
  <li><strong>Pure Rust crypto</strong> — RustCrypto ecosystem only. No OpenSSL, no C dependencies for crypto.</li>
  <li><strong>Pluggable engines</strong> — KV, Transit, Database, PKI. Mount engines at any path.</li>
  <li><strong>Built-in web UI</strong> — manage secrets, policies, and leases from the browser.</li>
</ul>

<h2>Quick Links</h2>
<table>
  <thead><tr><th>Topic</th><th>Description</th></tr></thead>
  <tbody>
    <tr><td><a href="/docs/getting-started">Getting Started</a></td><td>Install, initialize, and unseal your first vault</td></tr>
    <tr><td><a href="/docs/architecture">Architecture</a></td><td>Barrier pattern, key hierarchy, crate structure</td></tr>
    <tr><td><a href="/docs/api">API Reference</a></td><td>Complete HTTP API documentation</td></tr>
    <tr><td><a href="/docs/cli">CLI Reference</a></td><td>Command-line tool usage</td></tr>
    <tr><td><a href="/docs/engines">Secrets Engines</a></td><td>KV v2, Transit, Database, PKI</td></tr>
    <tr><td><a href="/docs/policies">Policies & Auth</a></td><td>Access control and authentication methods</td></tr>
    <tr><td><a href="/docs/security">Security Model</a></td><td>Threat model, crypto choices, hardening</td></tr>
    <tr><td><a href="/docs/configuration">Configuration</a></td><td>Environment variables and deployment options</td></tr>
  </tbody>
</table>

<h2>Lifecycle Overview</h2>
<pre><code>1. Initialize  →  Generate root key, split into Shamir shares
2. Unseal      →  Submit threshold shares to decrypt root key
3. Operate     →  Read/write secrets, manage leases, audit everything
4. Seal        →  Zeroize all keys from memory, reject all operations
</code></pre>
"#;

/// Getting started guide.
const GETTING_STARTED: &str = r#"
<p>This guide walks you through building, running, and initializing ZVault for the first time.</p>

<h2>Prerequisites</h2>
<ul>
  <li>Rust 1.85+ (2024 edition)</li>
  <li>For RocksDB backend: <code>clang</code> and <code>libclang-dev</code></li>
</ul>

<h2>Build</h2>
<pre><code># Build the server (with RocksDB support)
cargo build --release --package zvault-server

# Build the CLI
cargo build --release --package zvault-cli</code></pre>

<h2>Run the Server</h2>
<pre><code># In-memory storage (development)
./target/release/zvault-server

# RocksDB storage (production)
ZVAULT_STORAGE=rocksdb ZVAULT_STORAGE_PATH=/var/lib/zvault \
  ./target/release/zvault-server</code></pre>

<p>The server starts on <code>http://127.0.0.1:8200</code> by default. Open the web UI or use the CLI.</p>

<h2>Step 1: Initialize</h2>
<p>Initialization generates the root encryption key and splits the unseal key into Shamir shares.
This can only be done once per storage backend.</p>

<pre><code># Via CLI
zvault-cli init --shares 5 --threshold 3

# Via API
curl -X POST http://127.0.0.1:8200/v1/sys/init \
  -H "Content-Type: application/json" \
  -d '{"shares": 5, "threshold": 3}'</code></pre>

<div class="callout callout-danger">
  <strong>Save your unseal shares securely.</strong> They are shown once and never stored by ZVault.
  Distribute them to trusted operators. You need the threshold number to unseal.
</div>

<h2>Step 2: Unseal</h2>
<p>Submit unseal shares one at a time until the threshold is reached.</p>

<pre><code># Submit each share
zvault-cli unseal &lt;share-1&gt;
zvault-cli unseal &lt;share-2&gt;
zvault-cli unseal &lt;share-3&gt;  # Threshold reached → vault unseals</code></pre>

<h2>Step 3: Authenticate</h2>
<p>Use the root token from initialization to authenticate. Then create scoped tokens for applications.</p>

<pre><code># Set the token for CLI
export VAULT_TOKEN="hvs.your-root-token"

# Check status
zvault-cli status</code></pre>

<h2>Step 4: Write Your First Secret</h2>
<pre><code># Write
zvault-cli kv put secret/myapp/db username=admin password=s3cret

# Read
zvault-cli kv get secret/myapp/db</code></pre>

<div class="callout callout-warn">
  <strong>Revoke the root token</strong> after creating scoped tokens for your applications.
  Root tokens have unrestricted access.
</div>
"#;

/// Architecture documentation.
const ARCHITECTURE: &str = r"
<p>ZVault follows a layered architecture where security boundaries are enforced at each level.</p>

<h2>The Barrier Pattern</h2>
<p>This is the most important architectural invariant. Every byte that reaches the storage backend
is encrypted. The barrier sits between the application layer and storage:</p>

<pre><code>Application layer (plaintext)
        │
        ▼
   ┌─────────┐
   │ Barrier  │  ← AES-256-GCM encrypt on write, decrypt on read
   └────┬────┘
        │
        ▼
Storage layer (ciphertext only)</code></pre>

<p>If the vault is sealed, the barrier returns an error for all operations. The storage backend
<strong>never</strong> sees plaintext data.</p>

<h2>Key Hierarchy</h2>
<pre><code>Unseal Key (256-bit)
  │  Split into N shares via Shamir's Secret Sharing
  │  Reconstructed from T shares at unseal time
  │
  └──► Encrypts: Root Key
        │
        Root Key (256-bit, held in memory only)
        │
        ├──► Encrypts all KV secret values
        ├──► Encrypts engine configuration
        ├──► Encrypts transit key material
        │
        └──► Per-Engine Keys (derived via HKDF-SHA256)
              Each engine gets its own derived key</code></pre>

<h2>Crate Structure</h2>
<table>
  <thead><tr><th>Crate</th><th>Responsibility</th></tr></thead>
  <tbody>
    <tr><td><code>zvault-storage</code></td><td>StorageBackend trait + Memory, RocksDB, redb implementations</td></tr>
    <tr><td><code>zvault-core</code></td><td>Barrier, seal/unseal, tokens, policies, audit, KV engine, transit engine, lease manager</td></tr>
    <tr><td><code>zvault-server</code></td><td>HTTP server (Axum), routes, middleware, web UI, docs site, config</td></tr>
    <tr><td><code>zvault-cli</code></td><td>Standalone CLI client — talks to the server via HTTP only</td></tr>
  </tbody>
</table>

<h2>Dependency Direction</h2>
<pre><code>zvault-server
  ├── zvault-core
  │     └── zvault-storage
  └── (HTTP layer, config, UI)

zvault-cli (standalone, HTTP client only — no internal deps)</code></pre>

<h2>Storage Key Namespacing</h2>
<p>All keys in the storage backend are namespaced by prefix:</p>
<pre><code>sys/config              → vault configuration (encrypted root key, salt)
sys/mounts              → engine mount table
sys/policies/&lt;name&gt;     → policy definitions
sys/tokens/&lt;hash&gt;       → token metadata
sys/leases/&lt;id&gt;         → lease data
kv/&lt;mount&gt;/data/&lt;path&gt;  → KV secret data
transit/&lt;mount&gt;/keys/   → transit key material</code></pre>

<h2>Graceful Shutdown</h2>
<p>On <code>SIGTERM</code> or <code>SIGINT</code>:</p>
<ol>
  <li>Stop accepting new connections</li>
  <li>Broadcast shutdown to all background workers (lease expiry, etc.)</li>
  <li>Wait up to 10 seconds for in-flight requests to complete</li>
  <li>Zeroize all key material from memory</li>
  <li>Close storage backend</li>
  <li>Exit</li>
</ol>
";

/// API reference documentation.
const API_REFERENCE: &str = r#"
<p>All API endpoints are prefixed with <code>/v1</code>. Authenticated endpoints require
an <code>X-Vault-Token</code> header.</p>

<h2>System</h2>
<p>System endpoints manage vault lifecycle. Init and health do not require authentication.</p>

<div class="endpoint"><span class="method method-post">POST</span> <code>/v1/sys/init</code></div>
<p>Initialize the vault. Generates root key and unseal shares.</p>
<pre><code>Request:  {"shares": 5, "threshold": 3}
Response: {"unseal_shares": ["...", ...], "root_token": "hvs.xxx"}</code></pre>

<div class="endpoint"><span class="method method-post">POST</span> <code>/v1/sys/unseal</code></div>
<p>Submit an unseal key share. Returns progress until threshold is reached.</p>
<pre><code>Request:  {"share": "base64-encoded-share"}
Response: {"sealed": true, "progress": 2, "threshold": 3}
          {"sealed": false}  // when threshold reached</code></pre>

<div class="endpoint"><span class="method method-post">POST</span> <code>/v1/sys/seal</code></div>
<p>Seal the vault. Requires authentication. Zeroizes all keys from memory.</p>

<div class="endpoint"><span class="method method-get">GET</span> <code>/v1/sys/seal-status</code></div>
<p>Get current seal status. No authentication required.</p>
<pre><code>Response: {"initialized": true, "sealed": false, "threshold": 3, "shares": 5}</code></pre>

<div class="endpoint"><span class="method method-get">GET</span> <code>/v1/sys/health</code></div>
<p>Health check. Returns 200 if unsealed, 503 if sealed, 501 if not initialized.</p>

<h2>Secrets (KV v2)</h2>
<p>Read and write versioned key-value secrets. All endpoints require authentication.</p>

<div class="endpoint"><span class="method method-get">GET</span> <code>/v1/secret/data/:path</code></div>
<p>Read the latest version of a secret.</p>
<pre><code>Response: {"data": {"key": "value"}, "metadata": {"version": 3, "created_time": "..."}}</code></pre>

<div class="endpoint"><span class="method method-post">POST</span> <code>/v1/secret/data/:path</code></div>
<p>Write a new version of a secret.</p>
<pre><code>Request:  {"data": {"username": "admin", "password": "s3cret"}}
Response: {"version": 4, "created_time": "..."}</code></pre>

<div class="endpoint"><span class="method method-delete">DELETE</span> <code>/v1/secret/data/:path</code></div>
<p>Soft-delete the latest version (recoverable).</p>

<div class="endpoint"><span class="method method-post">POST</span> <code>/v1/secret/destroy/:path</code></div>
<p>Permanently destroy specific versions.</p>
<pre><code>Request: {"versions": [1, 2]}</code></pre>

<div class="endpoint"><span class="method method-get">GET</span> <code>/v1/secret/metadata/:path</code></div>
<p>Read version history and metadata for a secret.</p>

<div class="endpoint"><span class="method method-get">GET</span> <code>/v1/secret/list/:prefix</code></div>
<p>List secret keys under a prefix.</p>

<h2>Transit (Encryption as a Service)</h2>

<div class="endpoint"><span class="method method-post">POST</span> <code>/v1/transit/keys/:name</code></div>
<p>Create a named encryption key.</p>
<pre><code>Request: {"type": "aes256-gcm"}  // or "ed25519", "ecdsa-p256"</code></pre>

<div class="endpoint"><span class="method method-get">GET</span> <code>/v1/transit/keys/:name</code></div>
<p>Read key metadata (type, versions, creation time). Key material is never returned.</p>

<div class="endpoint"><span class="method method-post">POST</span> <code>/v1/transit/encrypt/:name</code></div>
<p>Encrypt plaintext with a named key.</p>
<pre><code>Request:  {"plaintext": "base64-encoded-data"}
Response: {"ciphertext": "vault:v1:base64-ciphertext"}</code></pre>

<div class="endpoint"><span class="method method-post">POST</span> <code>/v1/transit/decrypt/:name</code></div>
<p>Decrypt ciphertext.</p>
<pre><code>Request:  {"ciphertext": "vault:v1:base64-ciphertext"}
Response: {"plaintext": "base64-encoded-data"}</code></pre>

<div class="endpoint"><span class="method method-post">POST</span> <code>/v1/transit/sign/:name</code></div>
<p>Sign data with a named key (Ed25519 or ECDSA).</p>

<div class="endpoint"><span class="method method-post">POST</span> <code>/v1/transit/verify/:name</code></div>
<p>Verify a signature.</p>

<div class="endpoint"><span class="method method-post">POST</span> <code>/v1/transit/hash</code></div>
<p>Compute SHA-256 hash of input data.</p>

<div class="endpoint"><span class="method method-post">POST</span> <code>/v1/transit/random/:bytes</code></div>
<p>Generate cryptographically random bytes.</p>

<h2>Auth Tokens</h2>

<div class="endpoint"><span class="method method-post">POST</span> <code>/v1/auth/token/create</code></div>
<p>Create a new token with specified policies and TTL.</p>
<pre><code>Request:  {"policies": ["app-readonly"], "ttl": "1h"}
Response: {"token": "hvs.xxx", "policies": ["app-readonly"], "ttl": 3600}</code></pre>

<div class="endpoint"><span class="method method-get">GET</span> <code>/v1/auth/token/lookup</code></div>
<p>Look up metadata for the current token.</p>

<div class="endpoint"><span class="method method-post">POST</span> <code>/v1/auth/token/renew</code></div>
<p>Renew the current token's TTL.</p>

<div class="endpoint"><span class="method method-post">POST</span> <code>/v1/auth/token/revoke</code></div>
<p>Revoke a token and all its child tokens and leases.</p>

<h2>Policies</h2>

<div class="endpoint"><span class="method method-get">GET</span> <code>/v1/sys/policies</code></div>
<p>List all policy names.</p>

<div class="endpoint"><span class="method method-get">GET</span> <code>/v1/sys/policies/:name</code></div>
<p>Read a policy definition.</p>

<div class="endpoint"><span class="method method-post">POST</span> <code>/v1/sys/policies/:name</code></div>
<p>Create or update a policy.</p>

<div class="endpoint"><span class="method method-delete">DELETE</span> <code>/v1/sys/policies/:name</code></div>
<p>Delete a policy.</p>

<h2>Mounts</h2>

<div class="endpoint"><span class="method method-get">GET</span> <code>/v1/sys/mounts</code></div>
<p>List all engine mounts.</p>

<div class="endpoint"><span class="method method-post">POST</span> <code>/v1/sys/mounts/:path</code></div>
<p>Mount a new secrets engine at the given path.</p>

<div class="endpoint"><span class="method method-delete">DELETE</span> <code>/v1/sys/mounts/:path</code></div>
<p>Unmount an engine and revoke all its leases.</p>

<h2>Leases</h2>

<div class="endpoint"><span class="method method-get">GET</span> <code>/v1/sys/leases</code></div>
<p>List active leases.</p>

<div class="endpoint"><span class="method method-post">POST</span> <code>/v1/sys/leases/renew</code></div>
<p>Renew a lease by ID.</p>

<div class="endpoint"><span class="method method-post">POST</span> <code>/v1/sys/leases/revoke</code></div>
<p>Revoke a lease by ID.</p>
"#;

/// CLI reference documentation.
const CLI_REFERENCE: &str = r#"
<p>The <code>zvault-cli</code> is a standalone binary that communicates with the ZVault server
over HTTP. It has no internal dependencies on the server crates.</p>

<h2>Global Options</h2>
<table>
  <thead><tr><th>Flag</th><th>Env Var</th><th>Description</th></tr></thead>
  <tbody>
    <tr><td><code>--addr</code></td><td><code>VAULT_ADDR</code></td><td>Server address (default: <code>http://127.0.0.1:8200</code>)</td></tr>
    <tr><td><code>--token</code></td><td><code>VAULT_TOKEN</code></td><td>Authentication token</td></tr>
  </tbody>
</table>

<h2>System Commands</h2>

<h3><code>zvault-cli status</code></h3>
<p>Show vault seal status, initialization state, and server version.</p>

<h3><code>zvault-cli init</code></h3>
<p>Initialize a new vault instance.</p>
<pre><code>zvault-cli init --shares 5 --threshold 3</code></pre>
<table>
  <thead><tr><th>Flag</th><th>Default</th><th>Description</th></tr></thead>
  <tbody>
    <tr><td><code>--shares</code></td><td>5</td><td>Number of unseal key shares (2-10)</td></tr>
    <tr><td><code>--threshold</code></td><td>3</td><td>Shares required to unseal (2 to shares)</td></tr>
  </tbody>
</table>

<h3><code>zvault-cli unseal &lt;share&gt;</code></h3>
<p>Submit an unseal key share. Repeat until threshold is reached.</p>

<h3><code>zvault-cli seal</code></h3>
<p>Seal the vault. Requires authentication.</p>

<h2>KV Commands</h2>

<h3><code>zvault-cli kv get &lt;path&gt;</code></h3>
<p>Read a secret at the given path.</p>
<pre><code>zvault-cli kv get secret/myapp/db</code></pre>

<h3><code>zvault-cli kv put &lt;path&gt; [key=value ...]</code></h3>
<p>Write key-value pairs to a secret path.</p>
<pre><code>zvault-cli kv put secret/myapp/db username=admin password=s3cret</code></pre>

<h3><code>zvault-cli kv delete &lt;path&gt;</code></h3>
<p>Soft-delete a secret (recoverable).</p>

<h3><code>zvault-cli kv list &lt;prefix&gt;</code></h3>
<p>List secrets under a prefix.</p>
<pre><code>zvault-cli kv list secret/myapp/</code></pre>

<h2>Transit Commands</h2>

<h3><code>zvault-cli transit create-key &lt;name&gt;</code></h3>
<p>Create a named encryption key.</p>
<pre><code>zvault-cli transit create-key my-app-key --type aes256-gcm</code></pre>

<h3><code>zvault-cli transit encrypt &lt;key&gt;</code></h3>
<p>Encrypt data with a named key. Reads from stdin or <code>--plaintext</code> flag.</p>
<pre><code>echo "sensitive data" | zvault-cli transit encrypt my-app-key</code></pre>

<h3><code>zvault-cli transit decrypt &lt;key&gt;</code></h3>
<p>Decrypt ciphertext with a named key.</p>

<h2>Token Commands</h2>

<h3><code>zvault-cli token create</code></h3>
<p>Create a new token.</p>
<pre><code>zvault-cli token create --policies app-readonly --ttl 1h</code></pre>

<h3><code>zvault-cli token lookup</code></h3>
<p>Look up the current token's metadata.</p>

<h3><code>zvault-cli token revoke &lt;token&gt;</code></h3>
<p>Revoke a token and all its children.</p>

<h2>Policy Commands</h2>

<h3><code>zvault-cli policy list</code></h3>
<p>List all policies.</p>

<h3><code>zvault-cli policy read &lt;name&gt;</code></h3>
<p>Read a policy definition.</p>

<h3><code>zvault-cli policy write &lt;name&gt; &lt;file&gt;</code></h3>
<p>Create or update a policy from a JSON file.</p>
<pre><code>zvault-cli policy write app-readonly policy.json</code></pre>

<h3><code>zvault-cli policy delete &lt;name&gt;</code></h3>
<p>Delete a policy.</p>
"#;

/// Security model documentation.
const SECURITY_MODEL: &str = r#"
<p>ZVault is a secrets management platform. Security is the product, not a feature.
This page documents the threat model, cryptographic choices, and hardening measures.</p>

<h2>Cryptographic Primitives</h2>
<table>
  <thead><tr><th>Purpose</th><th>Algorithm</th><th>Library</th></tr></thead>
  <tbody>
    <tr><td>Data encryption</td><td>AES-256-GCM</td><td><code>aes-gcm</code> (RustCrypto)</td></tr>
    <tr><td>Key derivation</td><td>HKDF-SHA256</td><td><code>hkdf</code> (RustCrypto)</td></tr>
    <tr><td>Token hashing</td><td>SHA-256</td><td><code>sha2</code> (RustCrypto)</td></tr>
    <tr><td>Secret sharing</td><td>Shamir's SSS (GF(256))</td><td>Custom implementation</td></tr>
    <tr><td>Random generation</td><td>OS CSPRNG</td><td><code>rand::OsRng</code></td></tr>
    <tr><td>Constant-time comparison</td><td>—</td><td><code>subtle::ConstantTimeEq</code></td></tr>
    <tr><td>Key zeroization</td><td>—</td><td><code>zeroize</code></td></tr>
  </tbody>
</table>

<div class="callout callout-info">
  All cryptographic crates come from the RustCrypto ecosystem. No OpenSSL, no ring, no C-backed crypto.
</div>

<h2>Encryption Barrier</h2>
<p>The barrier is the core security boundary. It enforces that:</p>
<ul>
  <li>All data written to storage passes through <code>encrypt()</code></li>
  <li>All data read from storage passes through <code>decrypt()</code></li>
  <li>Every encryption uses a fresh 96-bit nonce from <code>OsRng</code></li>
  <li>Ciphertext format: <code>nonce (12 bytes) || ciphertext || tag (16 bytes)</code></li>
  <li>If the vault is sealed, the barrier rejects all operations</li>
</ul>

<h2>Key Material Protection</h2>
<ul>
  <li>All key types implement <code>Zeroize</code> and <code>ZeroizeOnDrop</code></li>
  <li>Memory is locked with <code>mlockall(MCL_CURRENT | MCL_FUTURE)</code> to prevent swapping</li>
  <li>Core dumps are disabled via <code>RLIMIT_CORE = 0</code></li>
  <li>Key material never appears in logs, error messages, or API responses</li>
  <li>Debug trait is manually implemented to redact sensitive fields</li>
</ul>

<h2>Token Security</h2>
<ul>
  <li>Tokens are 128-bit UUIDv4 values from the OS CSPRNG</li>
  <li>Tokens are hashed with SHA-256 before storage — plaintext tokens are never persisted</li>
  <li>Token comparison uses <code>subtle::ConstantTimeEq</code> to prevent timing attacks</li>
  <li>Failed auth attempts take the same time as successful ones</li>
</ul>

<h2>Audit System</h2>
<ul>
  <li>Every API request generates an audit entry <strong>before</strong> the response is sent</li>
  <li>If all audit backends fail, the request is denied (fail-closed)</li>
  <li>Sensitive fields are HMAC'd with a per-backend key</li>
  <li>Audit log is append-only — no update or delete operations</li>
</ul>

<h2>Input Validation</h2>
<ul>
  <li>Secret paths validated against <code>^[a-zA-Z0-9_\-/]+$</code></li>
  <li>Maximum secret value size: 1MB</li>
  <li>Maximum path depth: 10 segments</li>
  <li>Shamir shares: 2-10, threshold: 2 to share count</li>
</ul>

<h2>HTTP Security Headers</h2>
<pre><code>X-Content-Type-Options: nosniff
X-Frame-Options: DENY
Cache-Control: no-store</code></pre>

<h2>Production Hardening Checklist</h2>
<ul>
  <li>Use RocksDB or redb storage (not in-memory)</li>
  <li>Keep <code>ZVAULT_DISABLE_MLOCK=false</code> (enable memory locking)</li>
  <li>Enable audit logging to a persistent file</li>
  <li>Use scoped tokens — revoke the root token after initial setup</li>
  <li>Run as a non-root user with <code>CAP_IPC_LOCK</code> capability</li>
  <li>Restrict network access — bind to <code>127.0.0.1</code> and use a reverse proxy</li>
</ul>
"#;

/// Configuration documentation.
const CONFIGURATION: &str = r"
<p>ZVault is configured entirely through environment variables. No config files required.</p>

<h2>Environment Variables</h2>
<table>
  <thead><tr><th>Variable</th><th>Default</th><th>Description</th></tr></thead>
  <tbody>
    <tr>
      <td><code>PORT</code></td>
      <td>—</td>
      <td>Bind port (Railway/Render convention). When set, binds to <code>0.0.0.0:$PORT</code>.</td>
    </tr>
    <tr>
      <td><code>ZVAULT_BIND_ADDR</code></td>
      <td><code>127.0.0.1:8200</code></td>
      <td>Full bind address. Overrides <code>PORT</code> if both are set.</td>
    </tr>
    <tr>
      <td><code>ZVAULT_STORAGE</code></td>
      <td><code>memory</code></td>
      <td>Storage backend: <code>memory</code>, <code>rocksdb</code>, or <code>redb</code>.</td>
    </tr>
    <tr>
      <td><code>ZVAULT_STORAGE_PATH</code></td>
      <td><code>./data</code></td>
      <td>Filesystem path for persistent storage backends.</td>
    </tr>
    <tr>
      <td><code>ZVAULT_LOG_LEVEL</code></td>
      <td><code>info</code></td>
      <td>Log level filter: <code>debug</code>, <code>info</code>, <code>warn</code>, <code>error</code>.</td>
    </tr>
    <tr>
      <td><code>ZVAULT_AUDIT_FILE</code></td>
      <td>—</td>
      <td>Path to audit log file. If set, enables file audit backend.</td>
    </tr>
    <tr>
      <td><code>ZVAULT_ENABLE_TRANSIT</code></td>
      <td><code>true</code></td>
      <td>Mount the default transit encryption engine at <code>transit/</code>.</td>
    </tr>
    <tr>
      <td><code>ZVAULT_LEASE_SCAN_INTERVAL</code></td>
      <td><code>60</code></td>
      <td>Seconds between lease expiry scans.</td>
    </tr>
    <tr>
      <td><code>ZVAULT_DISABLE_MLOCK</code></td>
      <td><code>false</code></td>
      <td>Skip <code>mlockall</code>. Set to <code>true</code> in containers without <code>CAP_IPC_LOCK</code>.</td>
    </tr>
  </tbody>
</table>

<h2>Priority Order</h2>
<p>For the bind address, the priority is:</p>
<ol>
  <li><code>ZVAULT_BIND_ADDR</code> (explicit full address)</li>
  <li><code>PORT</code> (platform convention, binds to <code>0.0.0.0</code>)</li>
  <li>Default: <code>127.0.0.1:8200</code></li>
</ol>

<h2>Storage Backends</h2>

<h3>Memory (development)</h3>
<p>Data is stored in a <code>HashMap</code> in memory. All data is lost on restart.
Use for development and testing only.</p>
<pre><code>ZVAULT_STORAGE=memory</code></pre>

<h3>RocksDB (production, default feature)</h3>
<p>Embedded LSM-tree storage. Battle-tested at Facebook, TiKV, CockroachDB scale.
Requires <code>clang</code> and <code>libclang-dev</code> at build time.</p>
<pre><code>ZVAULT_STORAGE=rocksdb
ZVAULT_STORAGE_PATH=/var/lib/zvault/data</code></pre>

<h3>redb (pure Rust alternative)</h3>
<p>Embedded B-tree storage. Pure Rust, no C dependencies. Good for environments
where cross-compilation matters. Enable with the <code>redb-backend</code> feature flag.</p>
<pre><code>ZVAULT_STORAGE=redb
ZVAULT_STORAGE_PATH=/var/lib/zvault/data</code></pre>

<h2>Deployment Examples</h2>

<h3>Railway</h3>
<p>The project includes <code>railway.toml</code> and <code>railpack.toml</code>. Set environment
variables in the Railway dashboard:</p>
<pre><code>ZVAULT_STORAGE=rocksdb
ZVAULT_STORAGE_PATH=/data
ZVAULT_DISABLE_MLOCK=true
ZVAULT_LOG_LEVEL=info</code></pre>

<h3>Docker</h3>
<pre><code>docker build -t zvault .
docker run -p 8200:8200 \
  -e ZVAULT_STORAGE=rocksdb \
  -e ZVAULT_STORAGE_PATH=/data \
  -e ZVAULT_DISABLE_MLOCK=true \
  -v zvault-data:/data \
  zvault</code></pre>

<h3>Systemd</h3>
<pre><code>[Unit]
Description=ZVault Secrets Manager
After=network.target

[Service]
Type=simple
User=zvault
Group=zvault
ExecStart=/usr/local/bin/zvault-server
Environment=ZVAULT_STORAGE=rocksdb
Environment=ZVAULT_STORAGE_PATH=/var/lib/zvault/data
Environment=ZVAULT_AUDIT_FILE=/var/log/zvault/audit.log
AmbientCapabilities=CAP_IPC_LOCK
NoNewPrivileges=true
ProtectSystem=strict
ReadWritePaths=/var/lib/zvault /var/log/zvault

[Install]
WantedBy=multi-user.target</code></pre>
";

/// Secrets engines documentation.
const ENGINES: &str = r#"
<p>Secrets engines are pluggable components that handle different types of secret data.
Each engine is mounted at a path and handles all requests under that path.</p>

<h2>KV v2 (Key-Value)</h2>
<p>The default secrets engine. Stores versioned key-value pairs with full history.</p>

<h3>Features</h3>
<ul>
  <li>Version history — read any previous version of a secret</li>
  <li>Soft delete — mark versions as deleted (recoverable)</li>
  <li>Hard destroy — permanently remove version data</li>
  <li>Metadata — custom key-value pairs per secret</li>
  <li>Check-and-set (CAS) — prevent race conditions on writes</li>
  <li>Configurable max versions per secret</li>
</ul>

<h3>Usage</h3>
<pre><code># Write a secret
curl -X POST http://127.0.0.1:8200/v1/secret/data/myapp/db \
  -H "X-Vault-Token: $TOKEN" \
  -d '{"data": {"username": "admin", "password": "s3cret"}}'

# Read latest version
curl http://127.0.0.1:8200/v1/secret/data/myapp/db \
  -H "X-Vault-Token: $TOKEN"

# List secrets
curl http://127.0.0.1:8200/v1/secret/list/myapp/ \
  -H "X-Vault-Token: $TOKEN"</code></pre>

<h2>Transit (Encryption as a Service)</h2>
<p>Provides encryption, decryption, signing, and verification without exposing key material.
Applications send data to ZVault and get encrypted/signed results back.</p>

<h3>Key Types</h3>
<table>
  <thead><tr><th>Type</th><th>Algorithm</th><th>Operations</th></tr></thead>
  <tbody>
    <tr><td><code>aes256-gcm</code></td><td>AES-256-GCM</td><td>Encrypt, Decrypt</td></tr>
    <tr><td><code>ed25519</code></td><td>Ed25519</td><td>Sign, Verify</td></tr>
    <tr><td><code>ecdsa-p256</code></td><td>ECDSA P-256</td><td>Sign, Verify</td></tr>
  </tbody>
</table>

<h3>Key Versioning</h3>
<p>Each named key supports multiple versions. Encryption always uses the latest version.
Decryption tries all versions (ciphertext includes a version prefix). Old versions can
be disabled or destroyed for key rotation.</p>

<h3>Usage</h3>
<pre><code># Create a key
curl -X POST http://127.0.0.1:8200/v1/transit/keys/my-app-key \
  -H "X-Vault-Token: $TOKEN" \
  -d '{"type": "aes256-gcm"}'

# Encrypt
curl -X POST http://127.0.0.1:8200/v1/transit/encrypt/my-app-key \
  -H "X-Vault-Token: $TOKEN" \
  -d '{"plaintext": "aGVsbG8gd29ybGQ="}'

# Decrypt
curl -X POST http://127.0.0.1:8200/v1/transit/decrypt/my-app-key \
  -H "X-Vault-Token: $TOKEN" \
  -d '{"ciphertext": "vault:v1:..."}'</code></pre>

<h2>Database (Dynamic Credentials)</h2>
<div class="callout callout-warn">Planned — not yet implemented.</div>
<p>Generates short-lived database credentials on demand. Connects to a target database,
creates temporary users with a TTL, and revokes them on lease expiry.</p>

<h2>PKI (Certificate Authority)</h2>
<div class="callout callout-warn">Planned — not yet implemented.</div>
<p>Acts as an internal certificate authority. Generates X.509 certificates on demand
with configurable SANs, TTL, and key usage.</p>
"#;

/// Policies and auth documentation.
const POLICIES: &str = r#"
<p>ZVault uses path-based policies to control access. Every token is bound to one or more
policies that define what paths it can access and what operations it can perform.</p>

<h2>Policy Structure</h2>
<p>Policies are JSON documents with a list of path rules:</p>
<pre><code>{
  "name": "app-readonly",
  "rules": [
    {
      "path": "secret/data/production/*",
      "capabilities": ["read", "list"]
    },
    {
      "path": "transit/encrypt/app-key",
      "capabilities": ["update"]
    }
  ]
}</code></pre>

<h2>Capabilities</h2>
<table>
  <thead><tr><th>Capability</th><th>Description</th></tr></thead>
  <tbody>
    <tr><td><code>read</code></td><td>Read data at a path</td></tr>
    <tr><td><code>list</code></td><td>List keys under a prefix</td></tr>
    <tr><td><code>create</code></td><td>Create new data at a path</td></tr>
    <tr><td><code>update</code></td><td>Update existing data at a path</td></tr>
    <tr><td><code>delete</code></td><td>Delete data at a path</td></tr>
    <tr><td><code>sudo</code></td><td>Access system/admin endpoints</td></tr>
    <tr><td><code>deny</code></td><td>Explicitly deny access (always wins)</td></tr>
  </tbody>
</table>

<h2>Path Matching</h2>
<ul>
  <li><strong>Exact:</strong> <code>secret/data/production/db-password</code></li>
  <li><strong>Glob:</strong> <code>secret/data/production/*</code> (one level)</li>
  <li><strong>Recursive:</strong> <code>secret/data/production/**</code> (all descendants)</li>
</ul>
<p><code>deny</code> always takes precedence over other capabilities, regardless of other policies.</p>

<h2>Built-in Policies</h2>
<table>
  <thead><tr><th>Policy</th><th>Description</th></tr></thead>
  <tbody>
    <tr><td><code>root</code></td><td>Unrestricted access to everything. Assigned to the root token.</td></tr>
    <tr><td><code>default</code></td><td>Minimal access — token self-lookup and renewal.</td></tr>
  </tbody>
</table>

<h2>Managing Policies</h2>
<pre><code># Create a policy
zvault-cli policy write app-readonly policy.json

# List policies
zvault-cli policy list

# Read a policy
zvault-cli policy read app-readonly

# Delete a policy
zvault-cli policy delete app-readonly</code></pre>

<h2>Authentication Methods</h2>

<h3>Token Auth (built-in)</h3>
<p>The default auth method. Tokens are created with specific policies and TTLs.
Tokens form a hierarchy — revoking a parent revokes all children.</p>
<pre><code># Create a scoped token
zvault-cli token create --policies app-readonly --ttl 1h</code></pre>

<h3>AppRole (planned)</h3>
<p>Machine-to-machine authentication using a role ID (public) and secret ID (private, single-use).
Designed for CI/CD pipelines and automated systems.</p>

<h3>OIDC (planned)</h3>
<p>Authenticate via an external OpenID Connect provider (Okta, Auth0, Keycloak, Spring).
Maps OIDC claims to ZVault policies.</p>

<h3>Kubernetes (planned)</h3>
<p>Validate Kubernetes service account JWTs. Maps service accounts and namespaces to policies.
Essential for the K8s operator integration.</p>
"#;

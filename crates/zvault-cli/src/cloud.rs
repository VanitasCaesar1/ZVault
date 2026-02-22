//! CLI Cloud Mode — manage secrets on `ZVault` Cloud.
//!
//! Authenticates via browser OAuth (Clerk) or service token (`ZVAULT_TOKEN`).
//! All commands talk to the cloud API at `ZVAULT_CLOUD_URL` (default: `https://api.zvault.cloud`).

use std::fmt::Write as _;

use anyhow::{Context, Result, bail};
use serde_json::Value;

use super::{
    BOLD, CYAN, DIM, GREEN, MAGENTA, RED, RESET, YELLOW,
    header, kv_line, success, warning, parse_env_file, detect_project_name,
};

// ── Cloud config (.zvault.toml [cloud] section) ─────────────────────

const CLOUD_CONFIG_FILE: &str = ".zvault.toml";

/// Parsed `[cloud]` section from `.zvault.toml`.
#[derive(Debug, Default)]
pub struct CloudConfig {
    pub org: String,
    pub project: String,
    pub default_env: String,
}

/// Read the `[cloud]` section from `.zvault.toml`.
pub fn load_cloud_config() -> Result<CloudConfig> {
    let path = std::path::Path::new(CLOUD_CONFIG_FILE);
    if !path.exists() {
        bail!(
            "no .zvault.toml found — run `zvault cloud init` to link this directory to a cloud project"
        );
    }

    let content = std::fs::read_to_string(path)
        .with_context(|| format!("failed to read {CLOUD_CONFIG_FILE}"))?;

    // Minimal TOML parsing — we only need [cloud] org/project/default_env.
    let mut cfg = CloudConfig {
        default_env: "development".to_owned(),
        ..Default::default()
    };
    let mut in_cloud_section = false;

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('[') {
            in_cloud_section = trimmed == "[cloud]";
            continue;
        }
        if !in_cloud_section {
            continue;
        }
        if let Some((key, val)) = trimmed.split_once('=') {
            let key = key.trim();
            let val = val.trim().trim_matches('"');
            match key {
                "org" => val.clone_into(&mut cfg.org),
                "project" => val.clone_into(&mut cfg.project),
                "default_env" => val.clone_into(&mut cfg.default_env),
                _ => {}
            }
        }
    }

    if cfg.org.is_empty() || cfg.project.is_empty() {
        bail!(".zvault.toml is missing [cloud] org or project — run `zvault cloud init`");
    }

    Ok(cfg)
}

// ── Cloud HTTP client ────────────────────────────────────────────────

pub struct CloudClient {
    http: reqwest::Client,
    base_url: String,
    token: String,
}

impl CloudClient {
    pub fn new(base_url: String, token: String) -> Self {
        Self {
            http: reqwest::Client::new(),
            base_url,
            token,
        }
    }

    fn url(&self, path: &str) -> String {
        format!("{}{path}", self.base_url)
    }

    async fn get(&self, path: &str) -> Result<Value> {
        let resp = self
            .http
            .get(self.url(path))
            .header("Authorization", format!("Bearer {}", self.token))
            .send()
            .await
            .context("cloud request failed")?;
        handle_cloud_response(resp).await
    }

    async fn post(&self, path: &str, body: &Value) -> Result<Value> {
        let resp = self
            .http
            .post(self.url(path))
            .header("Authorization", format!("Bearer {}", self.token))
            .header("Content-Type", "application/json")
            .json(body)
            .send()
            .await
            .context("cloud request failed")?;
        handle_cloud_response(resp).await
    }

    async fn put(&self, path: &str, body: &Value) -> Result<Value> {
        let resp = self
            .http
            .put(self.url(path))
            .header("Authorization", format!("Bearer {}", self.token))
            .header("Content-Type", "application/json")
            .json(body)
            .send()
            .await
            .context("cloud request failed")?;
        handle_cloud_response(resp).await
    }

    async fn delete(&self, path: &str) -> Result<Value> {
        let resp = self
            .http
            .delete(self.url(path))
            .header("Authorization", format!("Bearer {}", self.token))
            .send()
            .await
            .context("cloud request failed")?;
        handle_cloud_response(resp).await
    }
}

async fn handle_cloud_response(resp: reqwest::Response) -> Result<Value> {
    let status = resp.status();
    if status == reqwest::StatusCode::NO_CONTENT {
        return Ok(Value::Null);
    }
    let body = resp.text().await.context("failed to read cloud response")?;
    if !status.is_success() {
        // Try to extract error message from JSON response.
        if let Ok(json) = serde_json::from_str::<Value>(&body) {
            if let Some(msg) = json.get("error").and_then(Value::as_str) {
                bail!("{msg}");
            }
        }
        bail!("cloud API returned {status}: {body}");
    }
    if body.is_empty() {
        return Ok(Value::Null);
    }
    serde_json::from_str(&body).context("failed to parse cloud response JSON")
}

// ── Token management ─────────────────────────────────────────────────

/// Save the cloud session token to `~/.zvault/cloud-token`.
fn save_cloud_token(token: &str) -> Result<std::path::PathBuf> {
    let home = home_dir()?;
    let dir = home.join(".zvault");
    if !dir.exists() {
        std::fs::create_dir_all(&dir)
            .with_context(|| format!("failed to create {}", dir.display()))?;
    }
    let path = dir.join("cloud-token");
    std::fs::write(&path, token)
        .with_context(|| format!("failed to write {}", path.display()))?;

    // Restrict permissions on Unix.
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let perms = std::fs::Permissions::from_mode(0o600);
        let _ = std::fs::set_permissions(&path, perms);
    }

    Ok(path)
}

/// Load the cloud session token from `~/.zvault/cloud-token`.
fn load_cloud_token() -> Result<Option<String>> {
    let home = home_dir()?;
    let path = home.join(".zvault").join("cloud-token");
    if !path.exists() {
        return Ok(None);
    }
    let token = std::fs::read_to_string(&path)
        .with_context(|| format!("failed to read {}", path.display()))?;
    let token = token.trim().to_owned();
    if token.is_empty() {
        return Ok(None);
    }
    Ok(Some(token))
}

/// Remove the cloud session token.
fn remove_cloud_token() -> Result<()> {
    let home = home_dir()?;
    let path = home.join(".zvault").join("cloud-token");
    if path.exists() {
        std::fs::remove_file(&path)
            .with_context(|| format!("failed to remove {}", path.display()))?;
    }
    Ok(())
}

fn home_dir() -> Result<std::path::PathBuf> {
    #[cfg(unix)]
    {
        std::env::var("HOME")
            .map(std::path::PathBuf::from)
            .context("HOME not set")
    }
    #[cfg(windows)]
    {
        std::env::var("USERPROFILE")
            .map(std::path::PathBuf::from)
            .context("USERPROFILE not set")
    }
}

/// Resolve the cloud token: `ZVAULT_TOKEN` env > saved token > error.
fn resolve_token() -> Result<String> {
    // 1. Service token from env (CI/CD use case).
    if let Ok(token) = std::env::var("ZVAULT_TOKEN") {
        if !token.is_empty() {
            return Ok(token);
        }
    }

    // 2. Saved session token from `zvault login`.
    if let Some(token) = load_cloud_token()? {
        return Ok(token);
    }

    bail!("not authenticated — run `zvault login` or set ZVAULT_TOKEN");
}

/// Resolve the cloud API base URL.
fn resolve_cloud_url() -> String {
    std::env::var("ZVAULT_CLOUD_URL")
        .unwrap_or_else(|_| "https://api.zvault.cloud".to_owned())
}

/// Build an authenticated `CloudClient` from resolved token + URL.
fn build_client() -> Result<CloudClient> {
    let token = resolve_token()?;
    let url = resolve_cloud_url();
    Ok(CloudClient::new(url, token))
}

// ── Command handlers ─────────────────────────────────────────────────

/// `zvault login` — authenticate CLI with cloud account via browser OAuth.
pub async fn cmd_cloud_login() -> Result<()> {
    let base_url = resolve_cloud_url();

    println!();
    header("🔐", "ZVault Cloud Login");
    println!();

    // Start a tiny local HTTP server to receive the callback token.
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .context("failed to bind local callback server")?;
    let port = listener
        .local_addr()
        .context("failed to get local address")?
        .port();

    let login_url = format!("{base_url}/cli/auth?port={port}");

    println!("  {DIM}Opening browser for authentication...{RESET}");
    println!();
    println!("  {CYAN}{login_url}{RESET}");
    println!();
    println!("  {DIM}If the browser doesn't open, copy the URL above.{RESET}");
    println!();

    // Try to open the browser (best-effort).
    let _ = open_browser(&login_url);

    // Wait for the callback with a timeout.
    let token = tokio::time::timeout(std::time::Duration::from_secs(120), async {
        let (mut stream, _) = listener
            .accept()
            .await
            .context("failed to accept callback connection")?;

        let mut buf = vec![0u8; 4096];
        let n = tokio::io::AsyncReadExt::read(&mut stream, &mut buf)
            .await
            .context("failed to read callback request")?;

        let request = String::from_utf8_lossy(&buf[..n]);

        // Extract token from GET /callback?token=xxx
        let token = extract_query_param(&request, "token")
            .ok_or_else(|| anyhow::anyhow!("no token received in callback"))?;

        // Send a simple HTML response.
        let response = "HTTP/1.1 200 OK\r\nContent-Type: text/html\r\n\r\n\
            <html><body style=\"font-family:system-ui;text-align:center;padding:60px\">\
            <h2>✓ Authenticated</h2><p>You can close this tab and return to the terminal.</p>\
            </body></html>";
        let _ = tokio::io::AsyncWriteExt::write_all(&mut stream, response.as_bytes()).await;

        Ok::<String, anyhow::Error>(token)
    })
    .await
    .context("login timed out after 120 seconds")??;

    let path = save_cloud_token(&token)?;
    println!();
    success(&format!(
        "Logged in to ZVault Cloud. Token saved to {DIM}{}{RESET}",
        path.display()
    ));
    println!();

    Ok(())
}

/// `zvault logout` — remove saved cloud token.
pub async fn cmd_cloud_logout() -> Result<()> {
    remove_cloud_token()?;
    println!();
    success("Logged out of ZVault Cloud.");
    println!();
    Ok(())
}

/// `zvault cloud init` — link current directory to a cloud project.
pub async fn cmd_cloud_init(org: Option<&str>, project: Option<&str>) -> Result<()> {
    let client = build_client()?;

    println!();
    header("☁️", "Link to ZVault Cloud Project");
    println!();

    // If org/project not provided, list available ones and prompt.
    let org_slug = if let Some(o) = org {
        o.to_owned()
    } else {
        let resp = client.get("/v1/cloud/orgs").await?;
        let orgs = resp
            .get("organizations")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();

        if orgs.is_empty() {
            bail!("no organizations found — create one at app.zvault.cloud");
        }

        println!("  {BOLD}Your organizations:{RESET}");
        for (i, o) in orgs.iter().enumerate() {
            let name = o.get("name").and_then(Value::as_str).unwrap_or("?");
            let slug = o.get("slug").and_then(Value::as_str).unwrap_or("?");
            let idx = i.saturating_add(1);
            println!("  {DIM}{idx}.{RESET} {name} {DIM}({slug}){RESET}");
        }
        println!();

        // Use the first org if only one exists.
        if orgs.len() == 1 {
            let slug = orgs[0]
                .get("slug")
                .and_then(Value::as_str)
                .unwrap_or("default");
            println!("  {DIM}Using org:{RESET} {BOLD}{slug}{RESET}");
            slug.to_owned()
        } else {
            bail!(
                "multiple orgs found — specify with: zvault cloud init --org <slug> --project <name>"
            );
        }
    };

    let project_name = match project {
        Some(p) => p.to_owned(),
        None => detect_project_name()?,
    };

    // Check if project exists, create if not.
    let projects_resp = client
        .get(&format!("/v1/cloud/orgs/{org_slug}/projects"))
        .await?;
    let projects = projects_resp
        .get("projects")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();

    let project_exists = projects.iter().any(|p| {
        p.get("name")
            .and_then(Value::as_str)
            .is_some_and(|n| n == project_name)
    });

    if !project_exists {
        println!(
            "  {DIM}Project '{project_name}' not found — creating...{RESET}"
        );
        let body = serde_json::json!({ "name": project_name });
        client
            .post(&format!("/v1/cloud/orgs/{org_slug}/projects"), &body)
            .await?;
        success(&format!("Created project {BOLD}{project_name}{RESET}"));
    }

    // Write .zvault.toml.
    let toml_content = format!(
        "[cloud]\norg = \"{org_slug}\"\nproject = \"{project_name}\"\ndefault_env = \"development\"\n"
    );
    std::fs::write(CLOUD_CONFIG_FILE, &toml_content)
        .with_context(|| format!("failed to write {CLOUD_CONFIG_FILE}"))?;

    println!();
    success(&format!(
        "Linked to {BOLD}{org_slug}/{project_name}{RESET}"
    ));
    kv_line("Config", CLOUD_CONFIG_FILE);
    kv_line("Default env", "development");
    println!();

    Ok(())
}

/// `zvault cloud push` — push local .env secrets to cloud project.
pub async fn cmd_cloud_push(env_file: Option<&str>, env: Option<&str>) -> Result<()> {
    let cfg = load_cloud_config()?;
    let client = build_client()?;
    let environment = env.unwrap_or(&cfg.default_env);

    // Read local .env file.
    let file_path = env_file.unwrap_or(".env");
    let content = std::fs::read_to_string(file_path)
        .with_context(|| format!("failed to read {file_path}"))?;
    let entries = parse_env_file(&content);

    if entries.is_empty() {
        bail!("no secrets found in {file_path}");
    }

    println!();
    header(
        "⬆️",
        &format!("Pushing secrets to {}/{}", cfg.org, cfg.project),
    );
    println!();
    kv_line("Environment", environment);
    kv_line("Source", file_path);
    kv_line("Secrets", &entries.len().to_string());
    println!();

    let mut pushed = 0u32;
    let mut failed = 0u32;

    for (key, value) in &entries {
        let body = serde_json::json!({
            "key": key,
            "value": value,
            "environment": environment,
        });

        let path = format!(
            "/v1/cloud/orgs/{}/projects/{}/secrets",
            cfg.org, cfg.project
        );

        match client.put(&path, &body).await {
            Ok(_) => {
                println!("  {GREEN}✓{RESET} {key}");
                pushed = pushed.saturating_add(1);
            }
            Err(e) => {
                println!("  {RED}✗{RESET} {key} — {RED}{e}{RESET}");
                failed = failed.saturating_add(1);
            }
        }
    }

    println!();
    if failed == 0 {
        success(&format!("Pushed {pushed} secrets to {environment}"));
    } else {
        warning(&format!(
            "Pushed {pushed} secrets, {failed} failed"
        ));
    }
    println!();

    Ok(())
}

/// `zvault cloud pull` — pull secrets from cloud to local .env file.
pub async fn cmd_cloud_pull(env: Option<&str>, output: Option<&str>, format: &str) -> Result<()> {
    let cfg = load_cloud_config()?;
    let client = build_client()?;
    let environment = env.unwrap_or(&cfg.default_env);

    println!();
    header(
        "⬇️",
        &format!("Pulling secrets from {}/{}", cfg.org, cfg.project),
    );
    println!();
    kv_line("Environment", environment);
    kv_line("Format", format);

    let path = format!(
        "/v1/cloud/orgs/{}/projects/{}/secrets?environment={environment}",
        cfg.org, cfg.project
    );
    let resp = client.get(&path).await?;

    let secrets = resp
        .get("secrets")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();

    if secrets.is_empty() {
        println!();
        warning(&format!("No secrets found in {environment}"));
        println!();
        return Ok(());
    }

    let content = match format {
        "json" => format_as_json(&cfg, environment, &secrets),
        "yaml" | "yml" => format_as_yaml(&cfg, environment, &secrets),
        _ => format_as_env(&cfg, environment, &secrets),
    };

    let default_ext = match format {
        "json" => ".env.json",
        "yaml" | "yml" => ".env.yaml",
        _ => ".env",
    };
    let out_path = output.unwrap_or(default_ext);
    std::fs::write(out_path, &content)
        .with_context(|| format!("failed to write {out_path}"))?;

    println!();
    success(&format!(
        "Pulled {} secrets to {BOLD}{out_path}{RESET} ({format})",
        secrets.len()
    ));
    println!();

    Ok(())
}

/// Format secrets as a `.env` file.
fn format_as_env(cfg: &CloudConfig, environment: &str, secrets: &[Value]) -> String {
    let mut content = format!(
        "# Pulled from ZVault Cloud — {}/{} ({environment})\n",
        cfg.org, cfg.project
    );
    let _ = writeln!(content, "# Do NOT commit this file\n");

    for secret in secrets {
        let key = secret.get("key").and_then(Value::as_str).unwrap_or("");
        let value = secret.get("value").and_then(Value::as_str).unwrap_or("");
        // Quote values that contain spaces, #, or newlines.
        if value.contains(' ') || value.contains('#') || value.contains('\n') || value.contains('"')
        {
            let escaped = value.replace('\\', "\\\\").replace('"', "\\\"");
            let _ = writeln!(content, "{key}=\"{escaped}\"");
        } else {
            let _ = writeln!(content, "{key}={value}");
        }
    }

    content
}

/// Format secrets as JSON.
fn format_as_json(cfg: &CloudConfig, environment: &str, secrets: &[Value]) -> String {
    let mut map = serde_json::Map::new();
    map.insert(
        "_meta".to_owned(),
        serde_json::json!({
            "org": cfg.org,
            "project": cfg.project,
            "environment": environment,
            "warning": "Do NOT commit this file"
        }),
    );

    for secret in secrets {
        let key = secret.get("key").and_then(Value::as_str).unwrap_or("");
        let value = secret.get("value").and_then(Value::as_str).unwrap_or("");
        if !key.is_empty() {
            map.insert(key.to_owned(), Value::String(value.to_owned()));
        }
    }

    // Pretty-print with 2-space indent.
    serde_json::to_string_pretty(&Value::Object(map)).unwrap_or_default()
}

/// Format secrets as YAML.
fn format_as_yaml(cfg: &CloudConfig, environment: &str, secrets: &[Value]) -> String {
    let mut content = format!(
        "# Pulled from ZVault Cloud — {}/{} ({environment})\n",
        cfg.org, cfg.project
    );
    let _ = writeln!(content, "# Do NOT commit this file\n");

    for secret in secrets {
        let key = secret.get("key").and_then(Value::as_str).unwrap_or("");
        let value = secret.get("value").and_then(Value::as_str).unwrap_or("");
        // YAML quoting: quote if value contains special chars.
        if value.is_empty()
            || value.contains(':')
            || value.contains('#')
            || value.contains('{')
            || value.contains('}')
            || value.contains('[')
            || value.contains(']')
            || value.contains('\'')
            || value.contains('"')
            || value.contains('\n')
            || value.starts_with(' ')
            || value.ends_with(' ')
            || value == "true"
            || value == "false"
            || value == "null"
            || value.parse::<f64>().is_ok()
        {
            let escaped = value.replace('\'', "''");
            let _ = writeln!(content, "{key}: '{escaped}'");
        } else {
            let _ = writeln!(content, "{key}: {value}");
        }
    }

    content
}

/// `zvault cloud status` — show linked project, current env, token status.
pub async fn cmd_cloud_status() -> Result<()> {
    println!();
    header("☁️", "ZVault Cloud Status");
    println!();

    // Check cloud config.
    match load_cloud_config() {
        Ok(cfg) => {
            kv_line("Organization", &cfg.org);
            kv_line("Project", &cfg.project);
            kv_line("Default Env", &cfg.default_env);
        }
        Err(_) => {
            kv_line("Project", &format!("{DIM}not linked{RESET}"));
        }
    }

    // Check token status.
    match resolve_token() {
        Ok(token) => {
            if token.starts_with("zvt_") {
                kv_line("Auth", &format!("{GREEN}service token{RESET}"));
            } else {
                kv_line("Auth", &format!("{GREEN}logged in{RESET}"));
            }
        }
        Err(_) => {
            kv_line("Auth", &format!("{RED}not authenticated{RESET}"));
        }
    }

    kv_line("Cloud URL", &resolve_cloud_url());
    println!();

    Ok(())
}

/// `zvault cloud envs` — list environments for current project.
pub async fn cmd_cloud_envs() -> Result<()> {
    let cfg = load_cloud_config()?;
    let client = build_client()?;

    println!();
    header("🌍", &format!("Environments — {}/{}", cfg.org, cfg.project));
    println!();

    let path = format!(
        "/v1/cloud/orgs/{}/projects/{}/environments",
        cfg.org, cfg.project
    );
    let resp = client.get(&path).await?;

    let envs = resp
        .get("environments")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();

    if envs.is_empty() {
        println!("  {DIM}(no environments){RESET}");
    } else {
        for env in &envs {
            let name = env.get("name").and_then(Value::as_str).unwrap_or("?");
            let count = env.get("secret_count").and_then(Value::as_u64).unwrap_or(0);
            let marker = if name == cfg.default_env {
                format!(" {GREEN}← default{RESET}")
            } else {
                String::new()
            };
            println!("  {CYAN}●{RESET} {name} {DIM}({count} secrets){RESET}{marker}");
        }
    }

    println!();
    Ok(())
}

/// `zvault cloud secrets` — list secret keys for an environment.
pub async fn cmd_cloud_secrets(env: Option<&str>) -> Result<()> {
    let cfg = load_cloud_config()?;
    let client = build_client()?;
    let environment = env.unwrap_or(&cfg.default_env);

    println!();
    header(
        "🔑",
        &format!("{}/{} — {environment}", cfg.org, cfg.project),
    );
    println!();

    let path = format!(
        "/v1/cloud/orgs/{}/projects/{}/secrets?environment={environment}",
        cfg.org, cfg.project
    );
    let resp = client.get(&path).await?;

    let secrets = resp
        .get("secrets")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();

    if secrets.is_empty() {
        println!("  {DIM}(no secrets in {environment}){RESET}");
    } else {
        for secret in &secrets {
            let key = secret.get("key").and_then(Value::as_str).unwrap_or("?");
            let updated = secret
                .get("updated_at")
                .and_then(Value::as_str)
                .unwrap_or("");
            println!("  {CYAN}├─{RESET} {key} {DIM}{updated}{RESET}");
        }
        println!();
        println!("  {DIM}{} secrets in {environment}{RESET}", secrets.len());
    }

    println!();
    Ok(())
}

/// `zvault cloud token create` — create a service token scoped to project + env.
pub async fn cmd_cloud_token_create(
    name: &str,
    env: Option<&str>,
    ttl: Option<&str>,
) -> Result<()> {
    let cfg = load_cloud_config()?;
    let client = build_client()?;
    let environment = env.unwrap_or(&cfg.default_env);

    println!();
    header("🪙", "Create Service Token");
    println!();

    let mut body = serde_json::json!({
        "name": name,
        "environment": environment,
    });

    if let Some(ttl_val) = ttl {
        if let Some(obj) = body.as_object_mut() {
            obj.insert("ttl".to_owned(), serde_json::json!(ttl_val));
        }
    }

    let path = format!(
        "/v1/cloud/orgs/{}/projects/{}/tokens",
        cfg.org, cfg.project
    );
    let resp = client.post(&path, &body).await?;

    let token = resp
        .get("token")
        .and_then(Value::as_str)
        .unwrap_or("(unknown)");

    kv_line("Name", name);
    kv_line("Environment", environment);
    kv_line("Project", &format!("{}/{}", cfg.org, cfg.project));
    println!();
    println!("  {YELLOW}{BOLD}⚠  Save this token — it will NOT be shown again.{RESET}");
    println!();
    println!("  {GREEN}{BOLD}{token}{RESET}");
    println!();
    println!("  {DIM}Usage:{RESET}");
    println!("    ZVAULT_TOKEN={token} zvault run -- npm start");
    println!();

    Ok(())
}

/// `zvault cloud token revoke` — revoke a service token.
pub async fn cmd_cloud_token_revoke(token_id: &str) -> Result<()> {
    let cfg = load_cloud_config()?;
    let client = build_client()?;

    let path = format!(
        "/v1/cloud/orgs/{}/projects/{}/tokens/{token_id}",
        cfg.org, cfg.project
    );
    client.delete(&path).await?;

    println!();
    success(&format!("Token {BOLD}{token_id}{RESET} revoked."));
    println!();

    Ok(())
}

/// `zvault cloud token list` — list service tokens for current project.
pub async fn cmd_cloud_token_list() -> Result<()> {
    let cfg = load_cloud_config()?;
    let client = build_client()?;

    println!();
    header("🪙", &format!("Service Tokens — {}/{}", cfg.org, cfg.project));
    println!();

    let path = format!(
        "/v1/cloud/orgs/{}/projects/{}/tokens",
        cfg.org, cfg.project
    );
    let resp = client.get(&path).await?;

    let tokens = resp
        .get("tokens")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();

    if tokens.is_empty() {
        println!("  {DIM}(no service tokens){RESET}");
    } else {
        for t in &tokens {
            let name = t.get("name").and_then(Value::as_str).unwrap_or("?");
            let env_name = t.get("environment").and_then(Value::as_str).unwrap_or("?");
            let created = t.get("created_at").and_then(Value::as_str).unwrap_or("");
            let id = t.get("id").and_then(Value::as_str).unwrap_or("?");
            println!(
                "  {MAGENTA}⚷{RESET}  {name} {DIM}({env_name}) — {id} — {created}{RESET}"
            );
        }
    }

    println!();
    Ok(())
}

/// `zvault run --env <env>` — resolve secrets from cloud, inject as env vars, run command.
pub async fn cmd_cloud_run(env: &str, command: &[String]) -> Result<()> {
    if command.is_empty() {
        bail!("no command specified — usage: zvault run --env staging -- npm start");
    }

    let cfg = load_cloud_config()?;
    let client = build_client()?;

    println!();
    header(
        "🔑",
        &format!("Resolving secrets from cloud ({env})"),
    );
    println!();

    let path = format!(
        "/v1/cloud/orgs/{}/projects/{}/secrets?environment={env}",
        cfg.org, cfg.project
    );
    let resp = client.get(&path).await?;

    let secrets = resp
        .get("secrets")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();

    if secrets.is_empty() {
        bail!("no secrets found in {env} — push secrets first with `zvault cloud push`");
    }

    let mut env_vars: Vec<(String, String)> = Vec::with_capacity(secrets.len());

    for secret in &secrets {
        let key = secret.get("key").and_then(Value::as_str).unwrap_or("");
        let value = secret.get("value").and_then(Value::as_str).unwrap_or("");
        println!("  {GREEN}✓{RESET} {key}");
        env_vars.push((key.to_owned(), value.to_owned()));
    }

    println!();
    println!("  {DIM}Resolved {} secrets from {env}{RESET}", env_vars.len());
    println!();

    // Execute the child process with injected environment.
    let program = &command[0];
    let args = &command[1..];

    println!("  {CYAN}{BOLD}▶{RESET} {BOLD}{}{RESET}", command.join(" "));
    println!();

    let status = std::process::Command::new(program)
        .args(args)
        .envs(env_vars)
        .status()
        .with_context(|| format!("failed to execute: {program}"))?;

    if !status.success() {
        let code = status.code().unwrap_or(1);
        bail!("command exited with code {code}");
    }

    Ok(())
}

// ── Helpers ──────────────────────────────────────────────────────────

/// Try to open a URL in the default browser (best-effort).
fn open_browser(url: &str) -> Result<()> {
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg(url)
            .spawn()
            .context("failed to open browser")?;
    }
    #[cfg(target_os = "linux")]
    {
        std::process::Command::new("xdg-open")
            .arg(url)
            .spawn()
            .context("failed to open browser")?;
    }
    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("cmd")
            .args(["/C", "start", url])
            .spawn()
            .context("failed to open browser")?;
    }
    Ok(())
}

/// Extract a query parameter value from a raw HTTP request string.
fn extract_query_param(request: &str, param: &str) -> Option<String> {
    // Parse "GET /callback?token=xxx&foo=bar HTTP/1.1\r\n..."
    let first_line = request.lines().next()?;
    let path = first_line.split_whitespace().nth(1)?;
    let query = path.split_once('?').map(|(_, q)| q)?;

    for pair in query.split('&') {
        if let Some((key, value)) = pair.split_once('=') {
            if key == param {
                return Some(value.to_owned());
            }
        }
    }

    None
}

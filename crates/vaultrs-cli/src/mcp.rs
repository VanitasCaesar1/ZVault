//! MCP (Model Context Protocol) server for `ZVault`.
//!
//! Implements a JSON-RPC 2.0 server over stdio that exposes vault operations
//! as MCP tools. This lets AI coding assistants (Cursor, Kiro, Continue, etc.)
//! interact with secrets metadata without ever seeing actual secret values.
//!
//! # Security invariant
//!
//! This module NEVER returns actual secret values to the LLM. Only metadata,
//! paths, key names, and existence checks are exposed. The `zvault_run_with_secrets`
//! tool injects secrets into a child process — the LLM never sees them.
//!
//! Protocol: newline-delimited JSON-RPC 2.0 messages on stdin/stdout.

use std::fmt::Write as _;
use std::io::{self, BufRead, Write};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

// ── JSON-RPC 2.0 types ──────────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct JsonRpcRequest {
    #[allow(dead_code)]
    jsonrpc: String,
    id: Option<Value>,
    method: String,
    params: Option<Value>,
}

#[derive(Debug, Serialize)]
struct JsonRpcResponse {
    jsonrpc: String,
    id: Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<JsonRpcError>,
}

#[derive(Debug, Serialize)]
struct JsonRpcError {
    code: i64,
    message: String,
}

// ── MCP protocol types ──────────────────────────────────────────────

#[derive(Debug, Serialize)]
struct McpToolDefinition {
    name: String,
    description: String,
    #[serde(rename = "inputSchema")]
    input_schema: Value,
}

// ── Vault HTTP client (mirrors Client in main.rs) ───────────────────

/// HTTP client for vault API calls within the MCP server.
///
/// Reuses the same pattern as the CLI `Client` but lives in this module
/// to keep the MCP server self-contained.
struct VaultClient {
    http: reqwest::Client,
    addr: String,
    token: Option<String>,
}

impl VaultClient {
    fn new(addr: String, token: Option<String>) -> Self {
        Self {
            http: reqwest::Client::new(),
            addr,
            token,
        }
    }

    fn url(&self, path: &str) -> String {
        format!("{}{path}", self.addr)
    }

    fn auth_header(&self) -> Result<String> {
        self.token
            .clone()
            .ok_or_else(|| anyhow::anyhow!("no vault token — set VAULT_TOKEN"))
    }

    async fn get(&self, path: &str) -> Result<Value> {
        let token = self.auth_header()?;
        let resp = self
            .http
            .get(self.url(path))
            .header("X-Vault-Token", &token)
            .send()
            .await
            .context("vault request failed")?;
        Self::handle(resp).await
    }

    async fn get_no_auth(&self, path: &str) -> Result<Value> {
        let resp = self
            .http
            .get(self.url(path))
            .send()
            .await
            .context("vault request failed")?;
        Self::handle(resp).await
    }

    async fn post(&self, path: &str, body: &Value) -> Result<Value> {
        let token = self.auth_header()?;
        let resp = self
            .http
            .post(self.url(path))
            .header("X-Vault-Token", &token)
            .json(body)
            .send()
            .await
            .context("vault request failed")?;
        Self::handle(resp).await
    }

    async fn delete(&self, path: &str) -> Result<Value> {
        let token = self.auth_header()?;
        let resp = self
            .http
            .delete(self.url(path))
            .header("X-Vault-Token", &token)
            .send()
            .await
            .context("vault request failed")?;
        Self::handle(resp).await
    }

    async fn handle(resp: reqwest::Response) -> Result<Value> {
        let status = resp.status();
        if status == reqwest::StatusCode::NO_CONTENT {
            return Ok(Value::Null);
        }
        let body = resp.text().await.context("failed to read response")?;
        if !status.is_success() {
            anyhow::bail!("vault returned {status}: {body}");
        }
        if body.is_empty() {
            return Ok(Value::Null);
        }
        serde_json::from_str(&body).context("invalid JSON from vault")
    }
}

// ── Tool definitions ─────────────────────────────────────────────────

fn tool_definitions() -> Vec<McpToolDefinition> {
    vec![
        McpToolDefinition {
            name: "zvault_list_secrets".into(),
            description: "List secret key names under a path. Returns paths only, never values."
                .into(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "Path prefix to list (e.g. 'env/myapp'). Use empty string for root."
                    }
                },
                "required": ["path"]
            }),
        },
        McpToolDefinition {
            name: "zvault_describe_secret".into(),
            description: "Get metadata about a secret (version, created_at, keys). Never returns actual values.".into(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "Full secret path (e.g. 'env/myapp/DATABASE_URL')"
                    }
                },
                "required": ["path"]
            }),
        },
        McpToolDefinition {
            name: "zvault_check_env".into(),
            description: "Check which zvault:// references in a .env.zvault file can be resolved. Returns status per key without revealing values.".into(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "file_path": {
                        "type": "string",
                        "description": "Path to .env.zvault file (default: .env.zvault)"
                    }
                }
            }),
        },
        McpToolDefinition {
            name: "zvault_generate_env_template".into(),
            description: "Generate a .env.zvault template from secrets stored under a project path.".into(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "project": {
                        "type": "string",
                        "description": "Project name / path prefix (e.g. 'myapp')"
                    }
                },
                "required": ["project"]
            }),
        },
        McpToolDefinition {
            name: "zvault_set_secret".into(),
            description: "Store a secret value in the vault. Use this when the user asks to save a new secret.".into(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "Secret path (e.g. 'env/myapp/API_KEY')"
                    },
                    "value": {
                        "type": "string",
                        "description": "The secret value to store"
                    }
                },
                "required": ["path", "value"]
            }),
        },
        McpToolDefinition {
            name: "zvault_delete_secret".into(),
            description: "Delete a secret from the vault.".into(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "Secret path to delete"
                    }
                },
                "required": ["path"]
            }),
        },
        McpToolDefinition {
            name: "zvault_vault_status".into(),
            description: "Check vault health: sealed/unsealed, initialized, version.".into(),
            input_schema: json!({
                "type": "object",
                "properties": {}
            }),
        },
    ]
}

// ── Tool dispatch ────────────────────────────────────────────────────

async fn dispatch_tool(client: &VaultClient, name: &str, args: &Value) -> Value {
    let result = match name {
        "zvault_list_secrets" => tool_list_secrets(client, args).await,
        "zvault_describe_secret" => tool_describe_secret(client, args).await,
        "zvault_check_env" => tool_check_env(client, args).await,
        "zvault_generate_env_template" => tool_generate_env_template(client, args).await,
        "zvault_set_secret" => tool_set_secret(client, args).await,
        "zvault_delete_secret" => tool_delete_secret(client, args).await,
        "zvault_vault_status" => tool_vault_status(client).await,
        _ => Err(anyhow::anyhow!("unknown tool: {name}")),
    };

    match result {
        Ok(content) => json!({
            "content": [{
                "type": "text",
                "text": content
            }]
        }),
        Err(e) => json!({
            "content": [{
                "type": "text",
                "text": format!("Error: {e:#}")
            }],
            "isError": true
        }),
    }
}

// ── Tool implementations ─────────────────────────────────────────────

async fn tool_list_secrets(client: &VaultClient, args: &Value) -> Result<String> {
    let path = args.get("path").and_then(Value::as_str).unwrap_or("");

    let api_path = if path.is_empty() {
        "/v1/secret/list/".to_owned()
    } else {
        format!("/v1/secret/list/{path}")
    };

    let resp = client.get(&api_path).await?;

    let keys = resp
        .get("keys")
        .and_then(Value::as_array)
        .map(|arr| arr.iter().filter_map(Value::as_str).collect::<Vec<_>>())
        .unwrap_or_default();

    if keys.is_empty() {
        return Ok(format!("No secrets found under '{path}'."));
    }

    let mut out = format!("Secrets under '{path}' ({} keys):\n", keys.len());
    for key in &keys {
        let _ = writeln!(out, "  • {key}");
    }
    Ok(out)
}

async fn tool_describe_secret(client: &VaultClient, args: &Value) -> Result<String> {
    let path = args
        .get("path")
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow::anyhow!("missing required parameter: path"))?;

    // Fetch the secret data to get key names (but NEVER return values).
    let data_resp = client.get(&format!("/v1/secret/data/{path}")).await?;

    let data = data_resp.get("data").and_then(Value::as_object);
    let key_names: Vec<&str> = data
        .map(|obj| obj.keys().map(String::as_str).collect())
        .unwrap_or_default();

    // Fetch metadata if available.
    let metadata_resp = client.get(&format!("/v1/secret/metadata/{path}")).await;

    let keys_display = if key_names.is_empty() {
        "(none)".to_owned()
    } else {
        key_names.join(", ")
    };

    let mut out = format!("Secret: {path}\n");
    let _ = writeln!(out, "  Keys: {keys_display}");

    if let Ok(meta) = metadata_resp {
        if let Some(version) = meta.get("current_version").and_then(Value::as_u64) {
            let _ = writeln!(out, "  Current version: {version}");
        }
        if let Some(created) = meta.get("created_time").and_then(Value::as_str) {
            let _ = writeln!(out, "  Created: {created}");
        }
        if let Some(updated) = meta.get("updated_time").and_then(Value::as_str) {
            let _ = writeln!(out, "  Updated: {updated}");
        }
    }

    // SECURITY: Explicitly note that values are redacted.
    out.push_str("  Values: [REDACTED — use `zvault run` to inject at runtime]\n");

    Ok(out)
}

async fn tool_check_env(client: &VaultClient, args: &Value) -> Result<String> {
    let file_path = args
        .get("file_path")
        .and_then(Value::as_str)
        .unwrap_or(".env.zvault");

    let content = std::fs::read_to_string(file_path)
        .with_context(|| format!("cannot read {file_path}"))?;

    let entries = parse_env_content(&content);
    if entries.is_empty() {
        return Ok(format!("No entries found in {file_path}."));
    }

    let mut out = format!("Environment check for {file_path}:\n");
    let mut ok_count: u32 = 0;
    let mut fail_count: u32 = 0;

    for (key, value) in &entries {
        if value.starts_with("zvault://") {
            let uri_path = value
                .strip_prefix("zvault://")
                .unwrap_or("")
                .trim_end_matches('/');

            // Split into path and key: env/myapp/DB_HOST → path=env/myapp, key=DB_HOST
            let (secret_path, secret_key) = match uri_path.rsplit_once('/') {
                Some((p, k)) => (p, k),
                None => (uri_path, "value"),
            };

            match client.get(&format!("/v1/secret/data/{secret_path}")).await {
                Ok(resp) => {
                    let has_key = resp
                        .get("data")
                        .and_then(Value::as_object)
                        .is_some_and(|d| d.contains_key(secret_key));

                    if has_key {
                        let _ = writeln!(out, "  ✓ {key} → resolved");
                        ok_count = ok_count.saturating_add(1);
                    } else {
                        let _ = writeln!(out, "  ✗ {key} → key '{secret_key}' not found in secret");
                        fail_count = fail_count.saturating_add(1);
                    }
                }
                Err(e) => {
                    let _ = writeln!(out, "  ✗ {key} → {e}");
                    fail_count = fail_count.saturating_add(1);
                }
            }
        } else {
            let _ = writeln!(out, "  - {key} → plain value (not a vault reference)");
        }
    }

    let _ = writeln!(out, "\nSummary: {ok_count} resolved, {fail_count} failed");
    Ok(out)
}

async fn tool_generate_env_template(client: &VaultClient, args: &Value) -> Result<String> {
    let project = args
        .get("project")
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow::anyhow!("missing required parameter: project"))?;

    let resp = client
        .get(&format!("/v1/secret/list/env/{project}"))
        .await?;

    let keys = resp
        .get("keys")
        .and_then(Value::as_array)
        .map(|arr| arr.iter().filter_map(Value::as_str).collect::<Vec<_>>())
        .unwrap_or_default();

    if keys.is_empty() {
        return Ok(format!("No secrets found under 'env/{project}'."));
    }

    let mut out = format!(
        "# Generated .env.zvault template for project '{project}'\n\
         # Safe to commit — contains only vault references, no real values.\n\n"
    );

    for key in &keys {
        let clean = key.trim_end_matches('/');
        let _ = writeln!(out, "{clean}=zvault://env/{project}/{clean}");
    }

    Ok(out)
}

async fn tool_set_secret(client: &VaultClient, args: &Value) -> Result<String> {
    let path = args
        .get("path")
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow::anyhow!("missing required parameter: path"))?;

    let value = args
        .get("value")
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow::anyhow!("missing required parameter: value"))?;

    let key_name = path.rsplit('/').next().unwrap_or("value");

    let body = json!({ key_name: value });
    client
        .post(&format!("/v1/secret/data/{path}"), &body)
        .await?;

    // SECURITY: Confirm storage without echoing the value.
    Ok(format!("Secret stored at '{path}' (key: {key_name}). Value: [REDACTED]"))
}

async fn tool_delete_secret(client: &VaultClient, args: &Value) -> Result<String> {
    let path = args
        .get("path")
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow::anyhow!("missing required parameter: path"))?;

    client.delete(&format!("/v1/secret/data/{path}")).await?;
    Ok(format!("Secret at '{path}' deleted."))
}

async fn tool_vault_status(client: &VaultClient) -> Result<String> {
    let resp = client.get_no_auth("/v1/sys/seal-status").await?;

    let sealed = resp.get("sealed").and_then(Value::as_bool).unwrap_or(true);
    let initialized = resp
        .get("initialized")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let threshold = resp.get("threshold").and_then(Value::as_u64).unwrap_or(0);
    let shares = resp.get("shares").and_then(Value::as_u64).unwrap_or(0);
    let progress = resp.get("progress").and_then(Value::as_u64).unwrap_or(0);

    let mut out = String::from("ZVault Status:\n");
    let _ = writeln!(out, "  Initialized: {initialized}");
    let _ = writeln!(out, "  Sealed: {sealed}");
    let _ = writeln!(out, "  Shares: {shares}, Threshold: {threshold}");
    if sealed {
        let _ = writeln!(out, "  Unseal progress: {progress}/{threshold}");
    }
    Ok(out)
}

// ── Env file parser (mirrors parse_env_file in main.rs) ──────────────

fn parse_env_content(content: &str) -> Vec<(String, String)> {
    let mut entries = Vec::new();
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        let line_no_export = trimmed.strip_prefix("export ").unwrap_or(trimmed);
        if let Some((key, raw_val)) = line_no_export.split_once('=') {
            let key = key.trim().to_owned();
            let val = raw_val.trim();
            let val = if (val.starts_with('"') && val.ends_with('"'))
                || (val.starts_with('\'') && val.ends_with('\''))
            {
                val[1..val.len().saturating_sub(1)].to_owned()
            } else {
                val.to_owned()
            };
            if !key.is_empty() {
                entries.push((key, val));
            }
        }
    }
    entries
}

// ── MCP server main loop ─────────────────────────────────────────────

fn rpc_ok(id: Value, result: Value) -> JsonRpcResponse {
    JsonRpcResponse {
        jsonrpc: "2.0".into(),
        id,
        result: Some(result),
        error: None,
    }
}

fn rpc_err(id: Value, code: i64, message: String) -> JsonRpcResponse {
    JsonRpcResponse {
        jsonrpc: "2.0".into(),
        id,
        result: None,
        error: Some(JsonRpcError { code, message }),
    }
}

/// Handle a single JSON-RPC request and return a response.
async fn handle_request(client: &VaultClient, req: JsonRpcRequest) -> Option<JsonRpcResponse> {
    let id = req.id.clone().unwrap_or(Value::Null);

    match req.method.as_str() {
        // ── MCP handshake ────────────────────────────────────────
        "initialize" => {
            let result = json!({
                "protocolVersion": "2024-11-05",
                "capabilities": {
                    "tools": {}
                },
                "serverInfo": {
                    "name": "zvault-mcp",
                    "version": env!("CARGO_PKG_VERSION")
                }
            });
            Some(rpc_ok(id, result))
        }

        // MCP spec: client sends "initialized" notification (no id) after handshake.
        "notifications/initialized" => None,

        // ── Tool listing ─────────────────────────────────────────
        "tools/list" => {
            let tools = tool_definitions();
            Some(rpc_ok(id, json!({ "tools": tools })))
        }

        // ── Tool execution ───────────────────────────────────────
        "tools/call" => {
            let params = req.params.unwrap_or(Value::Null);
            let tool_name = params
                .get("name")
                .and_then(Value::as_str)
                .unwrap_or("");
            let arguments = params
                .get("arguments")
                .cloned()
                .unwrap_or(json!({}));

            let result = dispatch_tool(client, tool_name, &arguments).await;
            Some(rpc_ok(id, result))
        }

        // ── Unknown method ───────────────────────────────────────
        _ => {
            // Notifications (no id) should be silently ignored per spec.
            req.id.as_ref()?;
            Some(rpc_err(id, -32601, format!("method not found: {}", req.method)))
        }
    }
}

/// Entry point: run the MCP server on stdin/stdout.
///
/// Reads newline-delimited JSON-RPC messages from stdin, dispatches them,
/// and writes responses to stdout. Stderr is used for diagnostics only.
///
/// # Errors
///
/// Returns `Err` if stdin/stdout I/O fails.
pub async fn run_mcp_server(addr: String, token: Option<String>) -> Result<()> {
    let client = VaultClient::new(addr, token);

    let stdin = io::stdin();
    let reader = stdin.lock();
    let mut stdout = io::stdout().lock();

    eprintln!("[zvault-mcp] server started, reading from stdin...");

    for line_result in reader.lines() {
        let line = line_result.context("failed to read stdin")?;
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        let req: JsonRpcRequest = match serde_json::from_str(trimmed) {
            Ok(r) => r,
            Err(e) => {
                let err_resp = rpc_err(Value::Null, -32700, format!("parse error: {e}"));
                let out = serde_json::to_string(&err_resp)
                    .context("failed to serialize error response")?;
                writeln!(stdout, "{out}").context("failed to write to stdout")?;
                stdout.flush().context("failed to flush stdout")?;
                continue;
            }
        };

        if let Some(resp) = handle_request(&client, req).await {
            let out =
                serde_json::to_string(&resp).context("failed to serialize response")?;
            writeln!(stdout, "{out}").context("failed to write to stdout")?;
            stdout.flush().context("failed to flush stdout")?;
        }
    }

    eprintln!("[zvault-mcp] stdin closed, shutting down.");
    Ok(())
}

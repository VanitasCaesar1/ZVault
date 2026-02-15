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

#[allow(clippy::too_many_lines)]
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
        McpToolDefinition {
            name: "zvault_query_database".into(),
            description: "Execute a SQL query against a database whose connection string is stored in the vault. The AI never sees the credentials — ZVault resolves the secret, connects, runs the query, and returns results. READ-ONLY by default (SELECT, EXPLAIN, SHOW). Set allow_write=true for INSERT/UPDATE/DELETE.".into(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "secret_path": {
                        "type": "string",
                        "description": "Vault path to the database connection string secret (e.g. 'env/Z-vault/POSTGRES_URL')"
                    },
                    "query": {
                        "type": "string",
                        "description": "SQL query to execute"
                    },
                    "allow_write": {
                        "type": "boolean",
                        "description": "Allow write operations (INSERT/UPDATE/DELETE/CREATE/DROP/ALTER). Default: false"
                    },
                    "max_rows": {
                        "type": "integer",
                        "description": "Maximum rows to return (default: 50, max: 500)"
                    }
                },
                "required": ["secret_path", "query"]
            }),
        },
        McpToolDefinition {
            name: "zvault_http_request".into(),
            description: "Make an HTTP request using credentials stored in the vault. The AI never sees the secret values — ZVault resolves zvault:// references in headers/URL and returns the response.".into(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "method": {
                        "type": "string",
                        "description": "HTTP method (GET, POST, PUT, DELETE)",
                        "enum": ["GET", "POST", "PUT", "DELETE"]
                    },
                    "url": {
                        "type": "string",
                        "description": "Request URL (can contain zvault:// references that will be resolved)"
                    },
                    "headers": {
                        "type": "object",
                        "description": "Request headers (values can be zvault:// references)",
                        "additionalProperties": { "type": "string" }
                    },
                    "body": {
                        "type": "string",
                        "description": "Request body (for POST/PUT)"
                    },
                    "secret_path": {
                        "type": "string",
                        "description": "Optional: vault path to a secret to use as Bearer token in Authorization header"
                    }
                },
                "required": ["method", "url"]
            }),
        },
        McpToolDefinition {
            name: "zvault_check_service".into(),
            description: "Health-check a service using credentials from the vault. Connects to the service (database, Redis, HTTP endpoint) and reports if it's reachable. Never exposes credentials.".into(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "secret_path": {
                        "type": "string",
                        "description": "Vault path to the connection string / URL secret"
                    },
                    "service_type": {
                        "type": "string",
                        "description": "Type of service to check",
                        "enum": ["postgres", "redis", "http"]
                    }
                },
                "required": ["secret_path", "service_type"]
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
        "zvault_query_database" => tool_query_database(client, args).await,
        "zvault_http_request" => tool_http_request(client, args).await,
        "zvault_check_service" => tool_check_service(client, args).await,
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

// ── Secret resolution helper ─────────────────────────────────────────

/// Resolve a vault secret path to its plaintext value.
/// This is used internally by proxy tools — the value is NEVER returned to the LLM.
async fn resolve_secret_value(client: &VaultClient, path: &str) -> Result<String> {
    let resp = client.get(&format!("/v1/secret/data/{path}")).await?;

    // The key name is the last segment of the path (e.g. "HEALTH_URL" from "env/test/HEALTH_URL").
    let key_name = path.rsplit('/').next().unwrap_or("value");

    // Walk through nested `data` envelopes (KV v2 response shape).
    let mut node = &resp;
    for _ in 0..4 {
        match node.get("data") {
            Some(inner) => node = inner,
            None => break,
        }
    }

    // Try the key name first (e.g. "HEALTH_URL"), then "value" as fallback.
    if let Some(val) = node.get(key_name).and_then(Value::as_str) {
        return Ok(val.to_owned());
    }
    if let Some(val) = node.get("value").and_then(Value::as_str) {
        return Ok(val.to_owned());
    }
    // If the node itself is a string (single-value secret).
    if let Some(val) = node.as_str() {
        return Ok(val.to_owned());
    }
    // If the node is an object with exactly one key, use that value.
    if let Some(obj) = node.as_object()
        && obj.len() == 1
        && let Some(val) = obj.values().next().and_then(Value::as_str)
    {
        return Ok(val.to_owned());
    }

    anyhow::bail!("no value found at secret path: {path}")
}

// ── Proxy tools (secure execution without exposing credentials) ──────

/// Execute a SQL query against a Postgres database using credentials from the vault.
/// The AI never sees the connection string.
async fn tool_query_database(client: &VaultClient, args: &Value) -> Result<String> {
    let secret_path = args.get("secret_path")
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow::anyhow!("missing required parameter: secret_path"))?;
    let query = args.get("query")
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow::anyhow!("missing required parameter: query"))?;
    let allow_write = args.get("allow_write")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let max_rows: usize = args.get("max_rows")
        .and_then(Value::as_u64)
        .map_or(50, |n| n.min(500) as usize);

    // Safety: block write operations unless explicitly allowed.
    let query_upper = query.trim().to_uppercase();
    let is_write = query_upper.starts_with("INSERT")
        || query_upper.starts_with("UPDATE")
        || query_upper.starts_with("DELETE")
        || query_upper.starts_with("DROP")
        || query_upper.starts_with("ALTER")
        || query_upper.starts_with("CREATE")
        || query_upper.starts_with("TRUNCATE");

    if is_write && !allow_write {
        anyhow::bail!(
            "Write operation blocked. The query appears to modify data. \
             Set allow_write=true to permit INSERT/UPDATE/DELETE/DDL operations."
        );
    }

    // Resolve the connection string from the vault (never exposed to AI).
    let conn_str = resolve_secret_value(client, secret_path).await
        .context("failed to resolve database connection string from vault")?;

    // Connect to Postgres.
    let tls_connector = native_tls::TlsConnector::builder()
        .danger_accept_invalid_certs(true)
        .build()
        .context("failed to build TLS connector")?;
    let pg_tls = postgres_native_tls::MakeTlsConnector::new(tls_connector);

    let (pg_client, connection) = tokio_postgres::connect(&conn_str, pg_tls)
        .await
        .context("failed to connect to database (credentials resolved from vault)")?;

    // Spawn the connection handler.
    tokio::spawn(async move {
        if let Err(e) = connection.await {
            eprintln!("postgres connection error: {e}");
        }
    });

    // Execute the query with a timeout.
    let rows = tokio::time::timeout(
        std::time::Duration::from_secs(30),
        pg_client.query(query, &[]),
    )
    .await
    .context("query timed out after 30 seconds")?
    .context("query execution failed")?;

    // Format results as a table.
    let mut output = String::new();

    if rows.is_empty() {
        if is_write {
            output.push_str("Query executed successfully. 0 rows affected.");
        } else {
            output.push_str("No rows returned.");
        }
        return Ok(output);
    }

    // Get column names.
    let columns = rows[0].columns();
    let col_names: Vec<&str> = columns.iter().map(tokio_postgres::Column::name).collect();

    // Header.
    let _ = writeln!(output, "{}", col_names.join(" | "));
    let _ = writeln!(output, "{}", col_names.iter().map(|c| "-".repeat(c.len().max(4))).collect::<Vec<_>>().join("-+-"));

    // Rows (capped at max_rows).
    let total = rows.len();
    for row in rows.iter().take(max_rows) {
        let vals: Vec<String> = columns.iter().enumerate().map(|(i, col)| {
            format_pg_value(row, i, col.type_())
        }).collect();
        let _ = writeln!(output, "{}", vals.join(" | "));
    }

    if total > max_rows {
        let _ = writeln!(output, "\n... ({total} total rows, showing first {max_rows})");
    } else {
        let _ = writeln!(output, "\n({total} rows)");
    }

    Ok(output)
}

/// Format a single Postgres column value to a display string.
fn format_pg_value(row: &tokio_postgres::Row, idx: usize, pg_type: &tokio_postgres::types::Type) -> String {
    use tokio_postgres::types::Type;

    match *pg_type {
        Type::BOOL => row.try_get::<_, bool>(idx).map_or_else(|_| "NULL".into(), |v| v.to_string()),
        Type::INT2 => row.try_get::<_, i16>(idx).map_or_else(|_| "NULL".into(), |v| v.to_string()),
        Type::INT4 => row.try_get::<_, i32>(idx).map_or_else(|_| "NULL".into(), |v| v.to_string()),
        Type::INT8 => row.try_get::<_, i64>(idx).map_or_else(|_| "NULL".into(), |v| v.to_string()),
        Type::FLOAT4 => row.try_get::<_, f32>(idx).map_or_else(|_| "NULL".into(), |v| v.to_string()),
        Type::FLOAT8 => row.try_get::<_, f64>(idx).map_or_else(|_| "NULL".into(), |v| v.to_string()),
        Type::TEXT | Type::VARCHAR | Type::NAME | Type::BPCHAR => {
            row.try_get::<_, String>(idx).unwrap_or_else(|_| "NULL".into())
        }
        Type::JSON | Type::JSONB => {
            row.try_get::<_, serde_json::Value>(idx)
                .map_or_else(|_| "NULL".into(), |v| v.to_string())
        }
        _ => {
            // Fallback: try as string, then show type name.
            row.try_get::<_, String>(idx)
                .unwrap_or_else(|_| format!("<{pg_type}>"))
        }
    }
}

/// Make an HTTP request with credentials resolved from the vault.
/// The AI provides the URL/headers with `zvault://` references; `ZVault` resolves them.
async fn tool_http_request(client: &VaultClient, args: &Value) -> Result<String> {
    let method = args.get("method")
        .and_then(Value::as_str)
        .unwrap_or("GET");
    let url = args.get("url")
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow::anyhow!("missing required parameter: url"))?;
    let body = args.get("body").and_then(Value::as_str);
    let headers = args.get("headers").and_then(Value::as_object);
    let secret_path = args.get("secret_path").and_then(Value::as_str);

    // Resolve zvault:// references in the URL.
    let resolved_url = if url.contains("zvault://") {
        resolve_zvault_refs_in_string(client, url).await?
    } else {
        url.to_owned()
    };

    // Build the request.
    let http = reqwest::Client::new();
    let mut req = match method.to_uppercase().as_str() {
        "GET" => http.get(&resolved_url),
        "POST" => http.post(&resolved_url),
        "PUT" => http.put(&resolved_url),
        "DELETE" => http.delete(&resolved_url),
        other => anyhow::bail!("unsupported HTTP method: {other}"),
    };

    // Resolve and set headers.
    if let Some(hdrs) = headers {
        for (key, val) in hdrs {
            if let Some(val_str) = val.as_str() {
                let resolved = if val_str.starts_with("zvault://") {
                    let path = val_str.strip_prefix("zvault://").unwrap_or(val_str);
                    resolve_secret_value(client, path).await
                        .with_context(|| format!("failed to resolve header {key}"))?
                } else {
                    val_str.to_owned()
                };
                req = req.header(key.as_str(), &resolved);
            }
        }
    }

    // If a secret_path is provided, use it as Bearer token.
    if let Some(sp) = secret_path {
        let token = resolve_secret_value(client, sp).await
            .context("failed to resolve auth token from vault")?;
        req = req.header("Authorization", format!("Bearer {token}"));
    }

    // Set body if provided.
    if let Some(b) = body {
        req = req.header("Content-Type", "application/json").body(b.to_owned());
    }

    // Execute with timeout.
    let resp = tokio::time::timeout(
        std::time::Duration::from_secs(30),
        req.send(),
    )
    .await
    .context("HTTP request timed out after 30 seconds")?
    .context("HTTP request failed")?;

    let status = resp.status();
    let resp_headers = format!("{:?}", resp.headers());
    let resp_body = resp.text().await.unwrap_or_default();

    // Truncate large responses.
    let body_display = if resp_body.len() > 10_000 {
        format!("{}... (truncated, {} bytes total)", &resp_body[..10_000], resp_body.len())
    } else {
        resp_body
    };

    let mut output = String::new();
    let _ = writeln!(output, "Status: {status}");
    let _ = writeln!(output, "Headers: {resp_headers}");
    let _ = writeln!(output, "\n{body_display}");

    // Scrub: make sure the resolved URL (which may contain secrets) is NOT in the output.
    // Replace it with the original URL pattern.
    let output = output.replace(&resolved_url, url);

    Ok(output)
}

/// Resolve all zvault:// references in a string.
async fn resolve_zvault_refs_in_string(client: &VaultClient, input: &str) -> Result<String> {
    let mut result = input.to_owned();
    while let Some(start) = result.find("zvault://") {
        // Find the end of the reference (next whitespace, quote, or end of string).
        let rest = &result[start..];
        let end = rest.find(|c: char| c.is_whitespace() || c == '"' || c == '\'' || c == ',' || c == '}')
            .unwrap_or(rest.len());
        let reference = &result[start..start + end];
        let path = reference.strip_prefix("zvault://").unwrap_or(reference);
        let value = resolve_secret_value(client, path).await
            .with_context(|| format!("failed to resolve {reference}"))?;
        result = format!("{}{}{}", &result[..start], value, &result[start + end..]);
    }
    Ok(result)
}

/// Health-check a service using credentials from the vault.
async fn tool_check_service(client: &VaultClient, args: &Value) -> Result<String> {
    let secret_path = args.get("secret_path")
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow::anyhow!("missing required parameter: secret_path"))?;
    let service_type = args.get("service_type")
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow::anyhow!("missing required parameter: service_type"))?;

    let conn_str = resolve_secret_value(client, secret_path).await
        .context("failed to resolve service credentials from vault")?;

    let start = std::time::Instant::now();

    match service_type {
        "postgres" => {
            let tls_connector = native_tls::TlsConnector::builder()
                .danger_accept_invalid_certs(true)
                .build()
                .context("failed to build TLS connector")?;
            let pg_tls = postgres_native_tls::MakeTlsConnector::new(tls_connector);

            let result = tokio::time::timeout(
                std::time::Duration::from_secs(10),
                tokio_postgres::connect(&conn_str, pg_tls),
            ).await;

            match result {
                Ok(Ok((pg_client, connection))) => {
                    tokio::spawn(async move { let _ = connection.await; });
                    let version = pg_client.query_one("SELECT version()", &[]).await
                        .map_or_else(|_| "unknown".into(), |row| row.get::<_, String>(0));
                    let elapsed = start.elapsed();
                    Ok(format!(
                        "✓ PostgreSQL is reachable\n  Version: {version}\n  Latency: {elapsed:.0?}\n  Secret: {secret_path} (credentials not shown)"
                    ))
                }
                Ok(Err(e)) => Ok(format!("✗ PostgreSQL connection failed\n  Error: {e}\n  Secret: {secret_path}")),
                Err(_) => Ok("✗ PostgreSQL connection timed out (10s)".into()),
            }
        }
        "redis" => {
            // Simple TCP connect check for Redis.
            let host_port = extract_redis_host_port(&conn_str);
            let result = tokio::time::timeout(
                std::time::Duration::from_secs(5),
                tokio::net::TcpStream::connect(&host_port),
            ).await;

            let elapsed = start.elapsed();
            match result {
                Ok(Ok(_)) => Ok(format!(
                    "✓ Redis is reachable\n  Latency: {elapsed:.0?}\n  Secret: {secret_path} (credentials not shown)"
                )),
                Ok(Err(e)) => Ok(format!("✗ Redis connection failed\n  Error: {e}\n  Secret: {secret_path}")),
                Err(_) => Ok("✗ Redis connection timed out (5s)".into()),
            }
        }
        "http" => {
            let http = reqwest::Client::new();
            let result = tokio::time::timeout(
                std::time::Duration::from_secs(10),
                http.get(&conn_str).send(),
            ).await;

            let elapsed = start.elapsed();
            match result {
                Ok(Ok(resp)) => Ok(format!(
                    "✓ HTTP endpoint is reachable\n  Status: {}\n  Latency: {elapsed:.0?}\n  Secret: {secret_path} (URL not shown)",
                    resp.status()
                )),
                Ok(Err(e)) => Ok(format!("✗ HTTP request failed\n  Error: {e}\n  Secret: {secret_path}")),
                Err(_) => Ok("✗ HTTP request timed out (10s)".into()),
            }
        }
        other => anyhow::bail!("unsupported service type: {other}. Use: postgres, redis, http"),
    }
}

/// Extract host:port from a Redis URL for TCP health check.
fn extract_redis_host_port(url: &str) -> String {
    // redis://[:password@]host:port[/db]
    let stripped = url.strip_prefix("redis://").or_else(|| url.strip_prefix("rediss://")).unwrap_or(url);
    let after_auth = if let Some(at_pos) = stripped.rfind('@') {
        &stripped[at_pos + 1..]
    } else {
        stripped
    };
    let host_port = after_auth.split('/').next().unwrap_or(after_auth);
    if host_port.contains(':') {
        host_port.to_owned()
    } else {
        format!("{host_port}:6379")
    }
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

    eprintln!("[zvault-mcp] server started, reading from stdin...");

    // Read stdin on a blocking thread so async vault HTTP calls can proceed.
    let (tx, mut rx) = tokio::sync::mpsc::channel::<String>(32);

    tokio::task::spawn_blocking(move || {
        let stdin = io::stdin();
        let reader = stdin.lock();
        for line_result in reader.lines() {
            match line_result {
                Ok(line) => {
                    if tx.blocking_send(line).is_err() {
                        break;
                    }
                }
                Err(_) => break,
            }
        }
    });

    let mut stdout = io::stdout().lock();

    while let Some(line) = rx.recv().await {
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

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
        // ── Tier 2: Tools 11–20 ─────────────────────────────────────
        McpToolDefinition {
            name: "zvault_query_redis".into(),
            description: "Execute Redis commands using credentials stored in the vault. Supports GET, SET, DEL, KEYS, HGETALL, INFO, DBSIZE, TTL, TYPE, EXISTS, MGET, LRANGE, SMEMBERS, SCARD, PING. The AI never sees the Redis URL.".into(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "secret_path": {
                        "type": "string",
                        "description": "Vault path to the Redis connection URL (e.g. 'env/myapp/REDIS_URL')"
                    },
                    "command": {
                        "type": "string",
                        "description": "Redis command to execute (e.g. 'GET mykey', 'KEYS user:*', 'INFO server')"
                    }
                },
                "required": ["secret_path", "command"]
            }),
        },
        McpToolDefinition {
            name: "zvault_query_mysql".into(),
            description: "Execute a SQL query against a MySQL/MariaDB database using credentials stored in the vault. READ-ONLY by default. Set allow_write=true for writes.".into(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "secret_path": {
                        "type": "string",
                        "description": "Vault path to the MySQL connection string (e.g. 'env/myapp/MYSQL_URL')"
                    },
                    "query": {
                        "type": "string",
                        "description": "SQL query to execute"
                    },
                    "allow_write": {
                        "type": "boolean",
                        "description": "Allow write operations (INSERT/UPDATE/DELETE/DDL). Default: false"
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
            name: "zvault_query_mongodb".into(),
            description: "Execute MongoDB operations (find, count, aggregate, listCollections) using credentials stored in the vault. READ-ONLY by default.".into(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "secret_path": {
                        "type": "string",
                        "description": "Vault path to the MongoDB connection string"
                    },
                    "database": {
                        "type": "string",
                        "description": "Database name"
                    },
                    "collection": {
                        "type": "string",
                        "description": "Collection name (not required for listCollections)"
                    },
                    "operation": {
                        "type": "string",
                        "description": "Operation to perform",
                        "enum": ["find", "count", "aggregate", "listCollections"]
                    },
                    "filter": {
                        "type": "object",
                        "description": "Query filter document (for find/count)"
                    },
                    "pipeline": {
                        "type": "array",
                        "description": "Aggregation pipeline stages (for aggregate)"
                    },
                    "limit": {
                        "type": "integer",
                        "description": "Maximum documents to return (default: 50, max: 500)"
                    }
                },
                "required": ["secret_path", "database", "operation"]
            }),
        },
        McpToolDefinition {
            name: "zvault_run_command".into(),
            description: "Execute a shell command with vault secrets injected as environment variables. The AI specifies which secrets to inject; ZVault resolves them and sets them as env vars for the child process. The AI never sees the secret values.".into(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "command": {
                        "type": "string",
                        "description": "Shell command to execute (e.g. 'npm run migrate', 'curl $API_URL/health')"
                    },
                    "secrets": {
                        "type": "object",
                        "description": "Map of ENV_VAR_NAME → vault secret path (e.g. {\"DATABASE_URL\": \"env/myapp/DATABASE_URL\"})",
                        "additionalProperties": { "type": "string" }
                    },
                    "timeout_secs": {
                        "type": "integer",
                        "description": "Timeout in seconds (default: 30, max: 120)"
                    }
                },
                "required": ["command", "secrets"]
            }),
        },
        McpToolDefinition {
            name: "zvault_s3_list".into(),
            description: "List objects in an S3/R2 bucket using credentials stored in the vault. Returns object keys, sizes, and last modified dates.".into(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "access_key_path": {
                        "type": "string",
                        "description": "Vault path to the S3 access key ID"
                    },
                    "secret_key_path": {
                        "type": "string",
                        "description": "Vault path to the S3 secret access key"
                    },
                    "endpoint": {
                        "type": "string",
                        "description": "S3 endpoint URL (for R2, MinIO, etc.). Omit for AWS S3."
                    },
                    "region": {
                        "type": "string",
                        "description": "AWS region (default: us-east-1)"
                    },
                    "bucket": {
                        "type": "string",
                        "description": "Bucket name"
                    },
                    "prefix": {
                        "type": "string",
                        "description": "Key prefix to filter by (optional)"
                    },
                    "max_keys": {
                        "type": "integer",
                        "description": "Maximum keys to return (default: 100, max: 1000)"
                    }
                },
                "required": ["access_key_path", "secret_key_path", "bucket"]
            }),
        },
        McpToolDefinition {
            name: "zvault_s3_read".into(),
            description: "Read an object from S3/R2 using credentials stored in the vault. Returns the object content (text files, configs, logs). Max 1MB.".into(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "access_key_path": {
                        "type": "string",
                        "description": "Vault path to the S3 access key ID"
                    },
                    "secret_key_path": {
                        "type": "string",
                        "description": "Vault path to the S3 secret access key"
                    },
                    "endpoint": {
                        "type": "string",
                        "description": "S3 endpoint URL (for R2, MinIO, etc.)"
                    },
                    "region": {
                        "type": "string",
                        "description": "AWS region (default: us-east-1)"
                    },
                    "bucket": {
                        "type": "string",
                        "description": "Bucket name"
                    },
                    "key": {
                        "type": "string",
                        "description": "Object key to read"
                    }
                },
                "required": ["access_key_path", "secret_key_path", "bucket", "key"]
            }),
        },
        McpToolDefinition {
            name: "zvault_s3_write".into(),
            description: "Write/upload an object to S3/R2 using credentials stored in the vault.".into(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "access_key_path": {
                        "type": "string",
                        "description": "Vault path to the S3 access key ID"
                    },
                    "secret_key_path": {
                        "type": "string",
                        "description": "Vault path to the S3 secret access key"
                    },
                    "endpoint": {
                        "type": "string",
                        "description": "S3 endpoint URL (for R2, MinIO, etc.)"
                    },
                    "region": {
                        "type": "string",
                        "description": "AWS region (default: us-east-1)"
                    },
                    "bucket": {
                        "type": "string",
                        "description": "Bucket name"
                    },
                    "key": {
                        "type": "string",
                        "description": "Object key to write"
                    },
                    "content": {
                        "type": "string",
                        "description": "Content to upload"
                    },
                    "content_type": {
                        "type": "string",
                        "description": "MIME type (default: application/octet-stream)"
                    }
                },
                "required": ["access_key_path", "secret_key_path", "bucket", "key", "content"]
            }),
        },
        McpToolDefinition {
            name: "zvault_query_clickhouse".into(),
            description: "Execute a SQL query against ClickHouse using credentials stored in the vault. READ-ONLY by default.".into(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "secret_path": {
                        "type": "string",
                        "description": "Vault path to the ClickHouse URL (e.g. 'env/myapp/CLICKHOUSE_URL')"
                    },
                    "query": {
                        "type": "string",
                        "description": "SQL query to execute"
                    },
                    "allow_write": {
                        "type": "boolean",
                        "description": "Allow write operations. Default: false"
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
            name: "zvault_search_meilisearch".into(),
            description: "Search a MeiliSearch index using an API key stored in the vault. Returns matching documents.".into(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "host_path": {
                        "type": "string",
                        "description": "Vault path to the MeiliSearch host URL"
                    },
                    "api_key_path": {
                        "type": "string",
                        "description": "Vault path to the MeiliSearch API key"
                    },
                    "index": {
                        "type": "string",
                        "description": "Index name to search"
                    },
                    "query": {
                        "type": "string",
                        "description": "Search query string"
                    },
                    "limit": {
                        "type": "integer",
                        "description": "Maximum results (default: 20, max: 100)"
                    },
                    "filter": {
                        "type": "string",
                        "description": "MeiliSearch filter expression (optional)"
                    }
                },
                "required": ["host_path", "api_key_path", "index", "query"]
            }),
        },
        McpToolDefinition {
            name: "zvault_rabbitmq_status".into(),
            description: "Check RabbitMQ status via the management API using credentials stored in the vault. Shows queues, consumers, message counts, and node health.".into(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "secret_path": {
                        "type": "string",
                        "description": "Vault path to the RabbitMQ management URL (e.g. http://user:pass@host:15672)"
                    },
                    "resource": {
                        "type": "string",
                        "description": "Resource to query",
                        "enum": ["overview", "queues", "connections", "channels", "nodes"]
                    }
                },
                "required": ["secret_path"]
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
        "zvault_query_redis" => tool_query_redis(client, args).await,
        "zvault_query_mysql" => tool_query_mysql(client, args).await,
        "zvault_query_mongodb" => tool_query_mongodb(client, args).await,
        "zvault_run_command" => tool_run_command(client, args).await,
        "zvault_s3_list" => tool_s3_list(client, args).await,
        "zvault_s3_read" => tool_s3_read(client, args).await,
        "zvault_s3_write" => tool_s3_write(client, args).await,
        "zvault_query_clickhouse" => tool_query_clickhouse(client, args).await,
        "zvault_search_meilisearch" => tool_search_meilisearch(client, args).await,
        "zvault_rabbitmq_status" => tool_rabbitmq_status(client, args).await,
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

    let content =
        std::fs::read_to_string(file_path).with_context(|| format!("cannot read {file_path}"))?;

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
    Ok(format!(
        "Secret stored at '{path}' (key: {key_name}). Value: [REDACTED]"
    ))
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
    let secret_path = args
        .get("secret_path")
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow::anyhow!("missing required parameter: secret_path"))?;
    let query = args
        .get("query")
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow::anyhow!("missing required parameter: query"))?;
    let allow_write = args
        .get("allow_write")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let max_rows: usize = args
        .get("max_rows")
        .and_then(Value::as_u64)
        .map_or(50, |n| n.min(500) as usize);

    // Safety: block write operations unless explicitly allowed.
    let is_write = is_write_query(query);

    if is_write && !allow_write {
        anyhow::bail!(
            "Write operation blocked. The query appears to modify data. \
             Set allow_write=true to permit INSERT/UPDATE/DELETE/DDL operations."
        );
    }

    // Resolve the connection string from the vault (never exposed to AI).
    let conn_str = resolve_secret_value(client, secret_path)
        .await
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
    let _ = writeln!(
        output,
        "{}",
        col_names
            .iter()
            .map(|c| "-".repeat(c.len().max(4)))
            .collect::<Vec<_>>()
            .join("-+-")
    );

    // Rows (capped at max_rows).
    let total = rows.len();
    for row in rows.iter().take(max_rows) {
        let vals: Vec<String> = columns
            .iter()
            .enumerate()
            .map(|(i, col)| format_pg_value(row, i, col.type_()))
            .collect();
        let _ = writeln!(output, "{}", vals.join(" | "));
    }

    if total > max_rows {
        let _ = writeln!(
            output,
            "\n... ({total} total rows, showing first {max_rows})"
        );
    } else {
        let _ = writeln!(output, "\n({total} rows)");
    }

    Ok(output)
}

/// Format a single Postgres column value to a display string.
fn format_pg_value(
    row: &tokio_postgres::Row,
    idx: usize,
    pg_type: &tokio_postgres::types::Type,
) -> String {
    use tokio_postgres::types::Type;

    match *pg_type {
        Type::BOOL => row
            .try_get::<_, bool>(idx)
            .map_or_else(|_| "NULL".into(), |v| v.to_string()),
        Type::INT2 => row
            .try_get::<_, i16>(idx)
            .map_or_else(|_| "NULL".into(), |v| v.to_string()),
        Type::INT4 => row
            .try_get::<_, i32>(idx)
            .map_or_else(|_| "NULL".into(), |v| v.to_string()),
        Type::INT8 => row
            .try_get::<_, i64>(idx)
            .map_or_else(|_| "NULL".into(), |v| v.to_string()),
        Type::FLOAT4 => row
            .try_get::<_, f32>(idx)
            .map_or_else(|_| "NULL".into(), |v| v.to_string()),
        Type::FLOAT8 => row
            .try_get::<_, f64>(idx)
            .map_or_else(|_| "NULL".into(), |v| v.to_string()),
        Type::TEXT | Type::VARCHAR | Type::NAME | Type::BPCHAR => row
            .try_get::<_, String>(idx)
            .unwrap_or_else(|_| "NULL".into()),
        Type::JSON | Type::JSONB => row
            .try_get::<_, serde_json::Value>(idx)
            .map_or_else(|_| "NULL".into(), |v| v.to_string()),
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
    let method = args.get("method").and_then(Value::as_str).unwrap_or("GET");
    let url = args
        .get("url")
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
                    resolve_secret_value(client, path)
                        .await
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
        let token = resolve_secret_value(client, sp)
            .await
            .context("failed to resolve auth token from vault")?;
        req = req.header("Authorization", format!("Bearer {token}"));
    }

    // Set body if provided.
    if let Some(b) = body {
        req = req
            .header("Content-Type", "application/json")
            .body(b.to_owned());
    }

    // Execute with timeout.
    let resp = tokio::time::timeout(std::time::Duration::from_secs(30), req.send())
        .await
        .context("HTTP request timed out after 30 seconds")?
        .context("HTTP request failed")?;

    let status = resp.status();
    let resp_headers = format!("{:?}", resp.headers());
    let resp_body = resp.text().await.unwrap_or_default();

    // Truncate large responses.
    let body_display = if resp_body.len() > 10_000 {
        format!(
            "{}... (truncated, {} bytes total)",
            &resp_body[..10_000],
            resp_body.len()
        )
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
        let end = rest
            .find(|c: char| c.is_whitespace() || c == '"' || c == '\'' || c == ',' || c == '}')
            .unwrap_or(rest.len());
        let reference = &result[start..start + end];
        let path = reference.strip_prefix("zvault://").unwrap_or(reference);
        let value = resolve_secret_value(client, path)
            .await
            .with_context(|| format!("failed to resolve {reference}"))?;
        result = format!("{}{}{}", &result[..start], value, &result[start + end..]);
    }
    Ok(result)
}

/// Health-check a service using credentials from the vault.
async fn tool_check_service(client: &VaultClient, args: &Value) -> Result<String> {
    let secret_path = args
        .get("secret_path")
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow::anyhow!("missing required parameter: secret_path"))?;
    let service_type = args
        .get("service_type")
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow::anyhow!("missing required parameter: service_type"))?;

    let conn_str = resolve_secret_value(client, secret_path)
        .await
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
            )
            .await;

            match result {
                Ok(Ok((pg_client, connection))) => {
                    tokio::spawn(async move {
                        let _ = connection.await;
                    });
                    let version = pg_client
                        .query_one("SELECT version()", &[])
                        .await
                        .map_or_else(|_| "unknown".into(), |row| row.get::<_, String>(0));
                    let elapsed = start.elapsed();
                    Ok(format!(
                        "✓ PostgreSQL is reachable\n  Version: {version}\n  Latency: {elapsed:.0?}\n  Secret: {secret_path} (credentials not shown)"
                    ))
                }
                Ok(Err(e)) => Ok(format!(
                    "✗ PostgreSQL connection failed\n  Error: {e}\n  Secret: {secret_path}"
                )),
                Err(_) => Ok("✗ PostgreSQL connection timed out (10s)".into()),
            }
        }
        "redis" => {
            // Simple TCP connect check for Redis.
            let host_port = extract_redis_host_port(&conn_str);
            let result = tokio::time::timeout(
                std::time::Duration::from_secs(5),
                tokio::net::TcpStream::connect(&host_port),
            )
            .await;

            let elapsed = start.elapsed();
            match result {
                Ok(Ok(_)) => Ok(format!(
                    "✓ Redis is reachable\n  Latency: {elapsed:.0?}\n  Secret: {secret_path} (credentials not shown)"
                )),
                Ok(Err(e)) => Ok(format!(
                    "✗ Redis connection failed\n  Error: {e}\n  Secret: {secret_path}"
                )),
                Err(_) => Ok("✗ Redis connection timed out (5s)".into()),
            }
        }
        "http" => {
            let http = reqwest::Client::new();
            let result = tokio::time::timeout(
                std::time::Duration::from_secs(10),
                http.get(&conn_str).send(),
            )
            .await;

            let elapsed = start.elapsed();
            match result {
                Ok(Ok(resp)) => Ok(format!(
                    "✓ HTTP endpoint is reachable\n  Status: {}\n  Latency: {elapsed:.0?}\n  Secret: {secret_path} (URL not shown)",
                    resp.status()
                )),
                Ok(Err(e)) => Ok(format!(
                    "✗ HTTP request failed\n  Error: {e}\n  Secret: {secret_path}"
                )),
                Err(_) => Ok("✗ HTTP request timed out (10s)".into()),
            }
        }
        other => anyhow::bail!("unsupported service type: {other}. Use: postgres, redis, http"),
    }
}

/// Extract host:port from a Redis URL for TCP health check.
fn extract_redis_host_port(url: &str) -> String {
    // redis://[:password@]host:port[/db]
    let stripped = url
        .strip_prefix("redis://")
        .or_else(|| url.strip_prefix("rediss://"))
        .unwrap_or(url);
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

// ── Tier 2: Tool implementations (11–20) ─────────────────────────────

/// Execute Redis commands using credentials from the vault.
async fn tool_query_redis(client: &VaultClient, args: &Value) -> Result<String> {
    let secret_path = args
        .get("secret_path")
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow::anyhow!("missing required parameter: secret_path"))?;
    let command_str = args
        .get("command")
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow::anyhow!("missing required parameter: command"))?;

    let redis_url = resolve_secret_value(client, secret_path)
        .await
        .context("failed to resolve Redis URL from vault")?;

    let redis_client = redis::Client::open(redis_url.as_str()).context("invalid Redis URL")?;

    let mut conn = tokio::time::timeout(
        std::time::Duration::from_secs(10),
        redis_client.get_multiplexed_async_connection(),
    )
    .await
    .context("Redis connection timed out (10s)")?
    .context("failed to connect to Redis")?;

    let parts: Vec<&str> = command_str.split_whitespace().collect();
    if parts.is_empty() {
        anyhow::bail!("empty command");
    }

    let cmd_name = parts[0].to_uppercase();
    let allowed = [
        "GET",
        "SET",
        "DEL",
        "KEYS",
        "HGETALL",
        "HGET",
        "HKEYS",
        "HLEN",
        "INFO",
        "DBSIZE",
        "TTL",
        "PTTL",
        "TYPE",
        "EXISTS",
        "MGET",
        "LRANGE",
        "LLEN",
        "SMEMBERS",
        "SCARD",
        "SISMEMBER",
        "PING",
        "STRLEN",
        "SCAN",
        "HSCAN",
    ];
    if !allowed.contains(&cmd_name.as_str()) {
        anyhow::bail!(
            "command '{cmd_name}' is not allowed. Allowed: {}",
            allowed.join(", ")
        );
    }

    let mut cmd = redis::cmd(&cmd_name);
    for arg in &parts[1..] {
        cmd.arg(*arg);
    }

    let result: redis::Value = tokio::time::timeout(
        std::time::Duration::from_secs(30),
        cmd.query_async(&mut conn),
    )
    .await
    .context("Redis command timed out (30s)")?
    .context("Redis command failed")?;

    Ok(format_redis_value(&result, 0))
}

/// Format a Redis value for display.
fn format_redis_value(val: &redis::Value, depth: usize) -> String {
    let indent = "  ".repeat(depth);
    match val {
        redis::Value::Nil => format!("{indent}(nil)"),
        redis::Value::Int(i) => format!("{indent}(integer) {i}"),
        redis::Value::BulkString(bytes) => match String::from_utf8(bytes.clone()) {
            Ok(s) => {
                if s.len() > 2000 {
                    format!(
                        "{indent}\"{}...\" (truncated, {} bytes)",
                        &s[..2000],
                        s.len()
                    )
                } else {
                    format!("{indent}\"{s}\"")
                }
            }
            Err(_) => format!("{indent}(binary, {} bytes)", bytes.len()),
        },
        redis::Value::Array(arr) => {
            if arr.is_empty() {
                return format!("{indent}(empty array)");
            }
            let mut out = String::new();
            for (i, item) in arr.iter().enumerate() {
                let _ = writeln!(
                    out,
                    "{indent}{}) {}",
                    i.saturating_add(1),
                    format_redis_value(item, 0)
                );
            }
            out
        }
        redis::Value::SimpleString(s) => format!("{indent}{s}"),
        redis::Value::Okay => format!("{indent}OK"),
        redis::Value::ServerError(e) => format!("{indent}(error) {e:?}"),
        _ => format!("{indent}(unknown redis type)"),
    }
}

/// Execute a SQL query against `MySQL` using credentials from the vault.
async fn tool_query_mysql(client: &VaultClient, args: &Value) -> Result<String> {
    use mysql_async::prelude::*;

    let secret_path = args
        .get("secret_path")
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow::anyhow!("missing required parameter: secret_path"))?;
    let query = args
        .get("query")
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow::anyhow!("missing required parameter: query"))?;
    let allow_write = args
        .get("allow_write")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let max_rows: usize = args
        .get("max_rows")
        .and_then(Value::as_u64)
        .map_or(50, |n| n.min(500) as usize);

    if !allow_write && is_write_query(query) {
        anyhow::bail!("Write operation blocked. Set allow_write=true to permit.");
    }

    let conn_str = resolve_secret_value(client, secret_path)
        .await
        .context("failed to resolve MySQL connection string from vault")?;

    let pool = mysql_async::Pool::new(conn_str.as_str());
    let mut conn = tokio::time::timeout(std::time::Duration::from_secs(10), pool.get_conn())
        .await
        .context("MySQL connection timed out (10s)")?
        .context("failed to connect to MySQL")?;

    let rows: Vec<mysql_async::Row> =
        tokio::time::timeout(std::time::Duration::from_secs(30), conn.query(query))
            .await
            .context("query timed out after 30 seconds")?
            .context("query execution failed")?;

    if rows.is_empty() {
        pool.disconnect().await.ok();
        return Ok(if is_write_query(query) {
            "Query executed successfully.".into()
        } else {
            "No rows returned.".into()
        });
    }

    let mut output = String::new();
    let columns: Vec<String> = rows[0]
        .columns_ref()
        .iter()
        .map(|c| c.name_str().to_string())
        .collect();

    let _ = writeln!(output, "{}", columns.join(" | "));
    let _ = writeln!(
        output,
        "{}",
        columns
            .iter()
            .map(|c| "-".repeat(c.len().max(4)))
            .collect::<Vec<_>>()
            .join("-+-")
    );

    let total = rows.len();
    for row in rows.iter().take(max_rows) {
        let vals: Vec<String> = (0..columns.len())
            .map(|i| {
                row.as_ref(i)
                    .map_or_else(|| "NULL".into(), |v| format!("{v:?}"))
            })
            .collect();
        let _ = writeln!(output, "{}", vals.join(" | "));
    }

    if total > max_rows {
        let _ = writeln!(
            output,
            "\n... ({total} total rows, showing first {max_rows})"
        );
    } else {
        let _ = writeln!(output, "\n({total} rows)");
    }

    pool.disconnect().await.ok();
    Ok(output)
}

/// Execute `MongoDB` operations using credentials from the vault.
async fn tool_query_mongodb(client: &VaultClient, args: &Value) -> Result<String> {
    let secret_path = args
        .get("secret_path")
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow::anyhow!("missing required parameter: secret_path"))?;
    let database = args
        .get("database")
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow::anyhow!("missing required parameter: database"))?;
    let operation = args
        .get("operation")
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow::anyhow!("missing required parameter: operation"))?;
    let collection_name = args.get("collection").and_then(Value::as_str);
    let limit: u64 = args
        .get("limit")
        .and_then(Value::as_u64)
        .map_or(50, |n| n.min(500));

    let (data_api_url, api_key) = resolve_mongodb_credentials(client, secret_path).await?;

    let base_url = data_api_url.trim_end_matches('/');

    let action = match operation {
        "find" => "find",
        "count" | "aggregate" | "listCollections" => "aggregate",
        other => anyhow::bail!(
            "unsupported operation: {other}. Use: find, count, aggregate, listCollections"
        ),
    };

    let url = format!("{base_url}/action/{action}");
    let body = build_mongodb_request_body(operation, database, collection_name, limit, args)?;

    let http = reqwest::Client::new();
    let resp = tokio::time::timeout(
        std::time::Duration::from_secs(30),
        http.post(&url)
            .header("api-key", &api_key)
            .header("Content-Type", "application/json")
            .json(&body)
            .send(),
    )
    .await
    .context("MongoDB request timed out (30s)")?
    .context("MongoDB request failed")?;

    let status = resp.status();
    let resp_body: Value = resp
        .json()
        .await
        .context("failed to parse MongoDB response")?;

    if !status.is_success() {
        let msg = resp_body
            .get("error")
            .and_then(Value::as_str)
            .unwrap_or("unknown error");
        anyhow::bail!("MongoDB Data API error ({status}): {msg}");
    }

    Ok(format_mongodb_response(&resp_body, database))
}

/// Resolve `MongoDB` Atlas Data API credentials from the vault.
///
/// Accepts either `"url|api_key"` pipe-delimited format or a JSON object
/// with `url` and `api_key` fields.
async fn resolve_mongodb_credentials(
    client: &VaultClient,
    secret_path: &str,
) -> Result<(String, String)> {
    let secret_val = resolve_secret_value(client, secret_path)
        .await
        .context("failed to resolve MongoDB credentials from vault")?;

    if let Some((url, key)) = secret_val.split_once('|') {
        return Ok((url.to_owned(), key.to_owned()));
    }

    if let Ok(obj) = serde_json::from_str::<serde_json::Map<String, Value>>(&secret_val) {
        let url = obj
            .get("url")
            .and_then(Value::as_str)
            .ok_or_else(|| anyhow::anyhow!("MongoDB secret must have 'url' field"))?;
        let key = obj
            .get("api_key")
            .and_then(Value::as_str)
            .ok_or_else(|| anyhow::anyhow!("MongoDB secret must have 'api_key' field"))?;
        return Ok((url.to_owned(), key.to_owned()));
    }

    anyhow::bail!(
        "MongoDB secret must be either 'url|api_key' or JSON {{\"url\": \"...\", \"api_key\": \"...\"}}"
    )
}

/// Build the JSON request body for a `MongoDB` Atlas Data API call.
fn build_mongodb_request_body(
    operation: &str,
    database: &str,
    collection_name: Option<&str>,
    limit: u64,
    args: &Value,
) -> Result<Value> {
    let body = match operation {
        "find" => {
            let coll = collection_name
                .ok_or_else(|| anyhow::anyhow!("'collection' is required for find"))?;
            let filter = args.get("filter").cloned().unwrap_or(json!({}));
            json!({
                "dataSource": "Cluster0",
                "database": database,
                "collection": coll,
                "filter": filter,
                "limit": limit,
            })
        }
        "count" => {
            let coll = collection_name
                .ok_or_else(|| anyhow::anyhow!("'collection' is required for count"))?;
            let filter = args.get("filter").cloned().unwrap_or(json!({}));
            json!({
                "dataSource": "Cluster0",
                "database": database,
                "collection": coll,
                "pipeline": [{"$match": filter}, {"$count": "count"}],
            })
        }
        "aggregate" => {
            let coll = collection_name
                .ok_or_else(|| anyhow::anyhow!("'collection' is required for aggregate"))?;
            let pipeline = args
                .get("pipeline")
                .cloned()
                .ok_or_else(|| anyhow::anyhow!("'pipeline' is required for aggregate"))?;
            json!({
                "dataSource": "Cluster0",
                "database": database,
                "collection": coll,
                "pipeline": pipeline,
            })
        }
        _ => json!({
            "dataSource": "Cluster0",
            "database": database,
            "collection": "$cmd",
            "pipeline": [{"$listCollections": {}}],
        }),
    };
    Ok(body)
}

/// Format a `MongoDB` Data API response into a human-readable string.
fn format_mongodb_response(resp_body: &Value, database: &str) -> String {
    let documents = resp_body.get("documents").and_then(Value::as_array);
    let Some(docs) = documents else {
        return serde_json::to_string_pretty(resp_body)
            .unwrap_or_else(|_| "No documents returned.".into());
    };

    if docs.is_empty() {
        return format!("No documents found in '{database}'.");
    }

    let mut out = format!("{} document(s):\n\n", docs.len());
    for doc in docs {
        let _ = writeln!(
            out,
            "{}",
            serde_json::to_string_pretty(doc).unwrap_or_else(|_| format!("{doc}"))
        );
        out.push('\n');
    }
    out
}

/// Execute a shell command with vault secrets injected as environment variables.
async fn tool_run_command(client: &VaultClient, args: &Value) -> Result<String> {
    let command = args
        .get("command")
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow::anyhow!("missing required parameter: command"))?;
    let secrets = args
        .get("secrets")
        .and_then(Value::as_object)
        .ok_or_else(|| anyhow::anyhow!("missing required parameter: secrets"))?;
    let timeout_secs: u64 = args
        .get("timeout_secs")
        .and_then(Value::as_u64)
        .map_or(30, |n| n.min(120));

    // Resolve all secrets from the vault.
    let mut env_vars: Vec<(String, String)> = Vec::with_capacity(secrets.len());
    for (env_name, vault_path_val) in secrets {
        let vault_path = vault_path_val
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("secret value for '{env_name}' must be a string"))?;
        let value = resolve_secret_value(client, vault_path)
            .await
            .with_context(|| format!("failed to resolve secret for {env_name}"))?;
        env_vars.push((env_name.clone(), value));
    }

    // Execute the command with secrets as env vars.
    let child = tokio::process::Command::new("sh")
        .arg("-c")
        .arg(command)
        .envs(env_vars.iter().map(|(k, v)| (k.as_str(), v.as_str())))
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .context("failed to spawn command")?;

    let output = tokio::time::timeout(
        std::time::Duration::from_secs(timeout_secs),
        child.wait_with_output(),
    )
    .await
    .context(format!("command timed out after {timeout_secs}s"))?
    .context("failed to wait for command")?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    let mut result = String::new();
    let _ = writeln!(result, "Exit code: {}", output.status.code().unwrap_or(-1));
    let _ = writeln!(result, "Secrets injected: {} env var(s)", secrets.len());

    if !stdout.is_empty() {
        let display = if stdout.len() > 10_000 {
            format!("{}... (truncated)", &stdout[..10_000])
        } else {
            stdout.to_string()
        };
        let _ = writeln!(result, "\n--- stdout ---\n{display}");
    }
    if !stderr.is_empty() {
        let display = if stderr.len() > 5_000 {
            format!("{}... (truncated)", &stderr[..5_000])
        } else {
            stderr.to_string()
        };
        let _ = writeln!(result, "\n--- stderr ---\n{display}");
    }

    // Scrub: ensure no secret values leaked into the output.
    // We replace any resolved secret value that appears in stdout/stderr.
    let mut scrubbed = result;
    for (env_name, value) in &env_vars {
        if value.len() >= 8 && scrubbed.contains(value.as_str()) {
            scrubbed = scrubbed.replace(value.as_str(), &format!("[{env_name}=REDACTED]"));
        }
    }

    Ok(scrubbed)
}

/// Build an S3 client from vault-stored credentials.
async fn build_s3_client(client: &VaultClient, args: &Value) -> Result<aws_sdk_s3::Client> {
    let access_key_path = args
        .get("access_key_path")
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow::anyhow!("missing required parameter: access_key_path"))?;
    let secret_key_path = args
        .get("secret_key_path")
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow::anyhow!("missing required parameter: secret_key_path"))?;
    let endpoint = args.get("endpoint").and_then(Value::as_str);
    let region = args
        .get("region")
        .and_then(Value::as_str)
        .unwrap_or("us-east-1");

    let access_key = resolve_secret_value(client, access_key_path)
        .await
        .context("failed to resolve S3 access key")?;
    let secret_key = resolve_secret_value(client, secret_key_path)
        .await
        .context("failed to resolve S3 secret key")?;

    let creds =
        aws_credential_types::Credentials::new(&access_key, &secret_key, None, None, "zvault");
    let creds_provider = aws_credential_types::provider::SharedCredentialsProvider::new(creds);

    let mut config_builder = aws_sdk_s3::Config::builder()
        .region(aws_sdk_s3::config::Region::new(region.to_owned()))
        .credentials_provider(creds_provider)
        .behavior_version_latest();

    if let Some(ep) = endpoint {
        config_builder = config_builder.endpoint_url(ep).force_path_style(true);
    }

    Ok(aws_sdk_s3::Client::from_conf(config_builder.build()))
}

/// List objects in an S3/R2 bucket.
async fn tool_s3_list(client: &VaultClient, args: &Value) -> Result<String> {
    let bucket = args
        .get("bucket")
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow::anyhow!("missing required parameter: bucket"))?;
    let prefix = args.get("prefix").and_then(Value::as_str);
    let max_keys: i32 = args
        .get("max_keys")
        .and_then(Value::as_i64)
        .and_then(|n| i32::try_from(n.clamp(1, 1000)).ok())
        .unwrap_or(100);

    let s3 = build_s3_client(client, args).await?;

    let mut req = s3.list_objects_v2().bucket(bucket).max_keys(max_keys);
    if let Some(p) = prefix {
        req = req.prefix(p);
    }

    let resp = tokio::time::timeout(std::time::Duration::from_secs(30), req.send())
        .await
        .context("S3 list timed out (30s)")?
        .context("S3 list failed")?;

    let objects = resp.contents();
    if objects.is_empty() {
        return Ok(format!(
            "No objects found in '{bucket}' (prefix: {}).",
            prefix.unwrap_or("none")
        ));
    }

    let mut out = format!("Objects in '{bucket}' ({} returned):\n\n", objects.len());
    let _ = writeln!(out, "{:<60} {:>10} Last Modified", "Key", "Size");
    let _ = writeln!(out, "{}", "-".repeat(90));

    for obj in objects {
        let key = obj.key().unwrap_or("?");
        let size = obj.size().unwrap_or(0);
        let modified = obj
            .last_modified()
            .map_or_else(|| "?".into(), std::string::ToString::to_string);
        let _ = writeln!(out, "{:<60} {:>10} {}", key, format_bytes(size), modified);
    }

    Ok(out)
}

/// Read an object from S3/R2.
async fn tool_s3_read(client: &VaultClient, args: &Value) -> Result<String> {
    let bucket = args
        .get("bucket")
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow::anyhow!("missing required parameter: bucket"))?;
    let key = args
        .get("key")
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow::anyhow!("missing required parameter: key"))?;

    let s3 = build_s3_client(client, args).await?;

    let resp = tokio::time::timeout(
        std::time::Duration::from_secs(30),
        s3.get_object().bucket(bucket).key(key).send(),
    )
    .await
    .context("S3 read timed out (30s)")?
    .context("S3 get_object failed")?;

    let content_length = resp.content_length().unwrap_or(0);
    if content_length > 1_048_576 {
        anyhow::bail!("Object is too large ({content_length} bytes). Max 1MB for read.");
    }

    let bytes = resp
        .body
        .collect()
        .await
        .context("failed to read S3 object body")?
        .into_bytes();

    let content = String::from_utf8(bytes.to_vec())
        .unwrap_or_else(|_| format!("(binary content, {content_length} bytes)"));

    let mut out = format!("Object: s3://{bucket}/{key}\n");
    let _ = writeln!(out, "Size: {}\n", format_bytes(content_length));
    out.push_str(&content);
    Ok(out)
}

/// Write an object to S3/R2.
async fn tool_s3_write(client: &VaultClient, args: &Value) -> Result<String> {
    let bucket = args
        .get("bucket")
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow::anyhow!("missing required parameter: bucket"))?;
    let key = args
        .get("key")
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow::anyhow!("missing required parameter: key"))?;
    let content = args
        .get("content")
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow::anyhow!("missing required parameter: content"))?;
    let content_type = args
        .get("content_type")
        .and_then(Value::as_str)
        .unwrap_or("application/octet-stream");

    let s3 = build_s3_client(client, args).await?;

    tokio::time::timeout(
        std::time::Duration::from_secs(30),
        s3.put_object()
            .bucket(bucket)
            .key(key)
            .body(aws_sdk_s3::primitives::ByteStream::from(
                content.as_bytes().to_vec(),
            ))
            .content_type(content_type)
            .send(),
    )
    .await
    .context("S3 write timed out (30s)")?
    .context("S3 put_object failed")?;

    Ok(format!(
        "✓ Uploaded to s3://{bucket}/{key} ({} bytes, {content_type})",
        content.len()
    ))
}

/// Format bytes into human-readable size using integer arithmetic.
fn format_bytes(bytes: i64) -> String {
    if bytes < 1024 {
        format!("{bytes} B")
    } else if bytes < 1_048_576 {
        let whole = bytes / 1024;
        let frac = (bytes % 1024) * 10 / 1024;
        format!("{whole}.{frac} KB")
    } else if bytes < 1_073_741_824 {
        let whole = bytes / 1_048_576;
        let frac = (bytes % 1_048_576) * 10 / 1_048_576;
        format!("{whole}.{frac} MB")
    } else {
        let whole = bytes / 1_073_741_824;
        let frac = (bytes % 1_073_741_824) * 10 / 1_073_741_824;
        format!("{whole}.{frac} GB")
    }
}

/// Execute a SQL query against `ClickHouse` using credentials from the vault.
async fn tool_query_clickhouse(client: &VaultClient, args: &Value) -> Result<String> {
    let secret_path = args
        .get("secret_path")
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow::anyhow!("missing required parameter: secret_path"))?;
    let query = args
        .get("query")
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow::anyhow!("missing required parameter: query"))?;
    let allow_write = args
        .get("allow_write")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let max_rows: usize = args
        .get("max_rows")
        .and_then(Value::as_u64)
        .map_or(50, |n| n.min(500) as usize);

    if !allow_write && is_write_query(query) {
        anyhow::bail!("Write operation blocked. Set allow_write=true to permit.");
    }

    let conn_str = resolve_secret_value(client, secret_path)
        .await
        .context("failed to resolve ClickHouse URL from vault")?;

    // ClickHouse URL format: http://user:pass@host:8123/database
    // Use reqwest to query the HTTP interface directly.
    let http = reqwest::Client::new();

    // Append FORMAT JSONEachRow if it's a SELECT and doesn't already have FORMAT.
    let query_upper = query.trim().to_uppercase();
    let final_query = if query_upper.starts_with("SELECT") && !query_upper.contains("FORMAT") {
        format!("{query} FORMAT JSONEachRow")
    } else {
        query.to_owned()
    };

    // Append LIMIT if not present for SELECT queries.
    let final_query = if query_upper.starts_with("SELECT") && !query_upper.contains("LIMIT") {
        format!("{final_query} LIMIT {max_rows}")
    } else {
        final_query
    };

    let resp = tokio::time::timeout(
        std::time::Duration::from_secs(30),
        http.post(&conn_str).body(final_query).send(),
    )
    .await
    .context("ClickHouse query timed out (30s)")?
    .context("ClickHouse request failed")?;

    let status = resp.status();
    let body = resp.text().await.unwrap_or_default();

    if !status.is_success() {
        anyhow::bail!("ClickHouse error ({status}): {body}");
    }

    if body.trim().is_empty() {
        return Ok(if is_write_query(query) {
            "Query executed successfully.".into()
        } else {
            "No rows returned.".into()
        });
    }

    Ok(format_clickhouse_rows(&body, max_rows))
}

/// Format `ClickHouse` `JSONEachRow` output into a readable table.
fn format_clickhouse_rows(body: &str, max_rows: usize) -> String {
    // Parse JSONEachRow: one JSON object per line.
    let lines: Vec<&str> = body.lines().filter(|l| !l.trim().is_empty()).collect();
    if lines.is_empty() {
        return "No rows returned.".into();
    }

    // Try to parse as JSON for nice formatting.
    let first: Result<serde_json::Map<String, Value>, _> = serde_json::from_str(lines[0]);
    if let Ok(first_row) = first {
        let columns: Vec<&str> = first_row.keys().map(String::as_str).collect();
        let mut output = String::new();
        let _ = writeln!(output, "{}", columns.join(" | "));
        let _ = writeln!(
            output,
            "{}",
            columns
                .iter()
                .map(|c| "-".repeat(c.len().max(4)))
                .collect::<Vec<_>>()
                .join("-+-")
        );

        let total = lines.len();
        for line in lines.iter().take(max_rows) {
            if let Ok(row) = serde_json::from_str::<serde_json::Map<String, Value>>(line) {
                let vals: Vec<String> = columns
                    .iter()
                    .map(|&col| {
                        row.get(col).map_or_else(
                            || "NULL".into(),
                            |v| match v {
                                Value::String(s) => s.clone(),
                                other => other.to_string(),
                            },
                        )
                    })
                    .collect();
                let _ = writeln!(output, "{}", vals.join(" | "));
            }
        }

        if total > max_rows {
            let _ = writeln!(
                output,
                "\n... ({total} total rows, showing first {max_rows})"
            );
        } else {
            let _ = writeln!(output, "\n({total} rows)");
        }
        output
    } else {
        // Fallback: return raw output.
        if body.len() > 10_000 {
            format!("{}... (truncated)", &body[..10_000])
        } else {
            body.to_owned()
        }
    }
}

/// Search a `MeiliSearch` index using credentials from the vault.
async fn tool_search_meilisearch(client: &VaultClient, args: &Value) -> Result<String> {
    let host_path = args
        .get("host_path")
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow::anyhow!("missing required parameter: host_path"))?;
    let api_key_path = args
        .get("api_key_path")
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow::anyhow!("missing required parameter: api_key_path"))?;
    let index = args
        .get("index")
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow::anyhow!("missing required parameter: index"))?;
    let query = args
        .get("query")
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow::anyhow!("missing required parameter: query"))?;
    let limit: u64 = args
        .get("limit")
        .and_then(Value::as_u64)
        .map_or(20, |n| n.min(100));
    let filter = args.get("filter").and_then(Value::as_str);

    let host = resolve_secret_value(client, host_path)
        .await
        .context("failed to resolve MeiliSearch host")?;
    let api_key = resolve_secret_value(client, api_key_path)
        .await
        .context("failed to resolve MeiliSearch API key")?;

    // Use the HTTP API directly (avoids meilisearch-sdk dependency).
    let http = reqwest::Client::new();
    let url = format!("{}/indexes/{index}/search", host.trim_end_matches('/'));

    let mut body = json!({
        "q": query,
        "limit": limit,
    });
    if let Some(f) = filter {
        body.as_object_mut()
            .map(|m| m.insert("filter".into(), json!(f)));
    }

    let resp = tokio::time::timeout(
        std::time::Duration::from_secs(15),
        http.post(&url)
            .header("Authorization", format!("Bearer {api_key}"))
            .json(&body)
            .send(),
    )
    .await
    .context("MeiliSearch request timed out (15s)")?
    .context("MeiliSearch request failed")?;

    let status = resp.status();
    let resp_body: Value = resp
        .json()
        .await
        .context("failed to parse MeiliSearch response")?;

    if !status.is_success() {
        let msg = resp_body
            .get("message")
            .and_then(Value::as_str)
            .unwrap_or("unknown error");
        anyhow::bail!("MeiliSearch error ({status}): {msg}");
    }

    let hits = resp_body.get("hits").and_then(Value::as_array);
    let total = resp_body
        .get("estimatedTotalHits")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let processing_time = resp_body
        .get("processingTimeMs")
        .and_then(Value::as_u64)
        .unwrap_or(0);

    let Some(hits) = hits else {
        return Ok("No results found.".into());
    };

    if hits.is_empty() {
        return Ok(format!("No results for '{query}' in index '{index}'."));
    }

    let mut out = format!(
        "Search results for '{query}' in '{index}' ({total} estimated total, {processing_time}ms):\n\n"
    );

    for (i, hit) in hits.iter().enumerate() {
        let _ = writeln!(out, "--- Result {} ---", i.saturating_add(1));
        let _ = writeln!(
            out,
            "{}",
            serde_json::to_string_pretty(hit).unwrap_or_else(|_| format!("{hit}"))
        );
        out.push('\n');
    }

    Ok(out)
}

/// Check `RabbitMQ` status via the management HTTP API.
async fn tool_rabbitmq_status(client: &VaultClient, args: &Value) -> Result<String> {
    let secret_path = args
        .get("secret_path")
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow::anyhow!("missing required parameter: secret_path"))?;
    let resource = args
        .get("resource")
        .and_then(Value::as_str)
        .unwrap_or("overview");

    let mgmt_url = resolve_secret_value(client, secret_path)
        .await
        .context("failed to resolve RabbitMQ management URL from vault")?;

    let api_path = match resource {
        "overview" => "/api/overview",
        "queues" => "/api/queues",
        "connections" => "/api/connections",
        "channels" => "/api/channels",
        "nodes" => "/api/nodes",
        other => anyhow::bail!(
            "unsupported resource: {other}. Use: overview, queues, connections, channels, nodes"
        ),
    };

    // Parse auth from URL: http://user:pass@host:15672
    let http = reqwest::Client::new();
    let full_url = format!("{}{api_path}", mgmt_url.trim_end_matches('/'));

    let resp = tokio::time::timeout(
        std::time::Duration::from_secs(10),
        http.get(&full_url).send(),
    )
    .await
    .context("RabbitMQ request timed out (10s)")?
    .context("RabbitMQ request failed")?;

    let status = resp.status();
    if !status.is_success() {
        let body = resp.text().await.unwrap_or_default();
        anyhow::bail!("RabbitMQ management API error ({status}): {body}");
    }

    let body: Value = resp
        .json()
        .await
        .context("failed to parse RabbitMQ response")?;

    match resource {
        "overview" => Ok(format_rabbitmq_overview(&body)),
        "queues" => Ok(format_rabbitmq_queues(&body)),
        _ => {
            // For connections, channels, nodes — just pretty-print the JSON.
            let display = serde_json::to_string_pretty(&body).unwrap_or_else(|_| format!("{body}"));
            let display = if display.len() > 10_000 {
                format!("{}... (truncated)", &display[..10_000])
            } else {
                display
            };
            Ok(format!("RabbitMQ {resource}:\n\n{display}"))
        }
    }
}

/// Format `RabbitMQ` overview response into a readable summary.
fn format_rabbitmq_overview(body: &Value) -> String {
    let mut out = String::from("RabbitMQ Overview:\n");
    if let Some(v) = body.get("rabbitmq_version").and_then(Value::as_str) {
        let _ = writeln!(out, "  Version: {v}");
    }
    if let Some(v) = body.get("erlang_version").and_then(Value::as_str) {
        let _ = writeln!(out, "  Erlang: {v}");
    }
    if let Some(q) = body.get("queue_totals").and_then(Value::as_object) {
        let messages = q.get("messages").and_then(Value::as_u64).unwrap_or(0);
        let ready = q.get("messages_ready").and_then(Value::as_u64).unwrap_or(0);
        let unacked = q
            .get("messages_unacknowledged")
            .and_then(Value::as_u64)
            .unwrap_or(0);
        let _ = writeln!(
            out,
            "  Messages: {messages} total, {ready} ready, {unacked} unacked"
        );
    }
    if let Some(o) = body.get("object_totals").and_then(Value::as_object) {
        let queues = o.get("queues").and_then(Value::as_u64).unwrap_or(0);
        let connections = o.get("connections").and_then(Value::as_u64).unwrap_or(0);
        let channels = o.get("channels").and_then(Value::as_u64).unwrap_or(0);
        let consumers = o.get("consumers").and_then(Value::as_u64).unwrap_or(0);
        let _ = writeln!(
            out,
            "  Queues: {queues}, Connections: {connections}, Channels: {channels}, Consumers: {consumers}"
        );
    }
    out
}

/// Format `RabbitMQ` queues response into a readable table.
fn format_rabbitmq_queues(body: &Value) -> String {
    let queues = body.as_array().map_or(Vec::new(), std::clone::Clone::clone);
    if queues.is_empty() {
        return "No queues found.".into();
    }
    let mut out = format!("Queues ({} total):\n\n", queues.len());
    let _ = writeln!(
        out,
        "{:<40} {:>8} {:>8} {:>8} {:>10}",
        "Name", "Ready", "Unacked", "Total", "Consumers"
    );
    let _ = writeln!(out, "{}", "-".repeat(80));
    for q in &queues {
        let name = q.get("name").and_then(Value::as_str).unwrap_or("?");
        let ready = q.get("messages_ready").and_then(Value::as_u64).unwrap_or(0);
        let unacked = q
            .get("messages_unacknowledged")
            .and_then(Value::as_u64)
            .unwrap_or(0);
        let total = q.get("messages").and_then(Value::as_u64).unwrap_or(0);
        let consumers = q.get("consumers").and_then(Value::as_u64).unwrap_or(0);
        let _ = writeln!(
            out,
            "{name:<40} {ready:>8} {unacked:>8} {total:>8} {consumers:>10}"
        );
    }
    out
}

/// Check if a SQL query is a write operation.
fn is_write_query(query: &str) -> bool {
    let upper = query.trim().to_uppercase();
    upper.starts_with("INSERT")
        || upper.starts_with("UPDATE")
        || upper.starts_with("DELETE")
        || upper.starts_with("DROP")
        || upper.starts_with("ALTER")
        || upper.starts_with("CREATE")
        || upper.starts_with("TRUNCATE")
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
            let tool_name = params.get("name").and_then(Value::as_str).unwrap_or("");
            let arguments = params.get("arguments").cloned().unwrap_or(json!({}));

            let result = dispatch_tool(client, tool_name, &arguments).await;
            Some(rpc_ok(id, result))
        }

        // ── Unknown method ───────────────────────────────────────
        _ => {
            // Notifications (no id) should be silently ignored per spec.
            req.id.as_ref()?;
            Some(rpc_err(
                id,
                -32601,
                format!("method not found: {}", req.method),
            ))
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
            let out = serde_json::to_string(&resp).context("failed to serialize response")?;
            writeln!(stdout, "{out}").context("failed to write to stdout")?;
            stdout.flush().context("failed to flush stdout")?;
        }
    }

    eprintln!("[zvault-mcp] stdin closed, shutting down.");
    Ok(())
}

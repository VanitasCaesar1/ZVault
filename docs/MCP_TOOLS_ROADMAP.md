# ZVault MCP Tools Roadmap

> 50 secure proxy tools for AI coding assistants.
> The AI sends queries/commands — ZVault resolves credentials from the vault, executes, and returns results. The AI never sees secrets.

---

## How It Works

```
AI Assistant (Cursor, Kiro, Copilot)
    │
    │  "SELECT * FROM users LIMIT 5"
    │  secret_path: "env/myapp/DATABASE_URL"
    ▼
┌──────────────────────────┐
│  ZVault MCP Server       │
│  ┌────────────────────┐  │
│  │ Resolve credential │  │  ← fetches from vault
│  │ Connect to service │  │  ← uses real credentials
│  │ Execute query      │  │
│  │ Return results     │  │  → AI sees rows, never the connection string
│  └────────────────────┘  │
└──────────────────────────┘
```

---

## Status Legend

| Icon | Meaning |
|------|---------|
| ✅ | Built and tested |
| 🔜 | Next up (high priority) |
| 📋 | Planned |
| 💡 | Nice to have |

---

## Tier 1: Built (1–10) ✅

Core vault operations + first proxy tools. Shipped in v0.1.0.

| # | Tool | Description | Tier |
|---|------|-------------|------|
| 1 | `zvault_list_secrets` | List secret key names under a path (never values) | Free |
| 2 | `zvault_describe_secret` | Metadata: version, created_at, key names (never values) | Free |
| 3 | `zvault_check_env` | Verify which `zvault://` refs in `.env.zvault` resolve | Free |
| 4 | `zvault_generate_env_template` | Generate `.env.zvault` from vault contents | Free |
| 5 | `zvault_set_secret` | Store a secret value in the vault | Free |
| 6 | `zvault_delete_secret` | Delete a secret from the vault | Free |
| 7 | `zvault_vault_status` | Check vault health (sealed/unsealed/initialized) | Free |
| 8 | `zvault_query_database` | SQL queries via vault-stored Postgres credentials | Pro |
| 9 | `zvault_http_request` | HTTP requests with `zvault://` refs in URL/headers | Pro |
| 10 | `zvault_check_service` | Health-check postgres/redis/http using vault creds | Pro |

---

## Tier 2: High Priority — Daily Dev Use (11–20) 🔜

These cover 95% of what developers need day-to-day. Build next.

| # | Tool | Description | Tier |
|---|------|-------------|------|
| 11 | `zvault_query_redis` | Execute Redis commands (GET, SET, KEYS, INFO, etc.) via vault-stored Redis URL | Pro |
| 12 | `zvault_query_mysql` | SQL queries via vault-stored MySQL/MariaDB credentials | Pro |
| 13 | `zvault_query_mongodb` | MongoDB queries (find, aggregate, count) via vault-stored connection string | Pro |
| 14 | `zvault_run_command` | Execute a shell command with vault secrets injected as env vars | Pro |
| 15 | `zvault_s3_list` | List objects in an S3/R2 bucket using vault-stored credentials | Pro |
| 16 | `zvault_s3_read` | Read an object from S3/R2 (text files, configs, logs) | Pro |
| 17 | `zvault_s3_write` | Write/upload an object to S3/R2 | Pro |
| 18 | `zvault_query_clickhouse` | SQL queries against ClickHouse via vault-stored credentials | Pro |
| 19 | `zvault_search_meilisearch` | Search a MeiliSearch index using vault-stored API key | Pro |
| 20 | `zvault_rabbitmq_status` | Check RabbitMQ queues, consumers, message counts via management API | Pro |

---

## Tier 3: Medium Priority — Weekly Use (21–30) 📋

Infrastructure and DevOps tools for more advanced workflows.

| # | Tool | Description | Tier |
|---|------|-------------|------|
| 21 | `zvault_ssh_exec` | Execute a command on a remote host via vault-stored SSH key | Team |
| 22 | `zvault_dns_lookup` | DNS queries using vault-stored DNS-over-HTTPS credentials | Pro |
| 23 | `zvault_smtp_send` | Send an email via vault-stored SMTP credentials | Team |
| 24 | `zvault_graphql_query` | Execute GraphQL queries with vault-stored auth tokens | Pro |
| 25 | `zvault_websocket_send` | Send a message to a WebSocket endpoint with vault auth | Pro |
| 26 | `zvault_tcp_check` | TCP connectivity check to any host:port (no creds needed) | Free |
| 27 | `zvault_tls_inspect` | Inspect TLS certificate of a remote host (expiry, issuer, chain) | Free |
| 28 | `zvault_docker_exec` | Execute a command in a Docker container via vault-stored Docker API creds | Team |
| 29 | `zvault_k8s_get` | Get Kubernetes resources via vault-stored kubeconfig | Enterprise |
| 30 | `zvault_k8s_logs` | Stream pod logs via vault-stored kubeconfig | Enterprise |

---

## Tier 4: Nice to Have — Power User (31–40) 💡

Database introspection, debugging, and vault management tools.

| # | Tool | Description | Tier |
|---|------|-------------|------|
| 31 | `zvault_pg_schema` | Introspect Postgres schema (tables, columns, indexes, constraints) | Pro |
| 32 | `zvault_pg_explain` | Run EXPLAIN ANALYZE on a query and return the plan | Pro |
| 33 | `zvault_pg_stats` | Database statistics (table sizes, row counts, index usage) | Pro |
| 34 | `zvault_redis_monitor` | Real-time Redis command stream (limited duration, sampled) | Pro |
| 35 | `zvault_grpc_call` | Make a gRPC call with vault-stored credentials | Team |
| 36 | `zvault_jwt_decode` | Decode a JWT token (header + payload, no verification) | Free |
| 37 | `zvault_rotate_secret` | Trigger rotation for a secret with a rotation policy | Pro |
| 38 | `zvault_copy_secret` | Copy a secret from one path to another within the vault | Pro |
| 39 | `zvault_diff_secrets` | Compare secrets between two paths (key names only, never values) | Pro |
| 40 | `zvault_bulk_import` | Import multiple secrets from a JSON/YAML file | Pro |

---

## Tier 5: Enterprise (41–50) 📋

Audit, compliance, and multi-tenant features.

| # | Tool | Description | Tier |
|---|------|-------------|------|
| 41 | `zvault_audit_query` | Query audit log entries with filters (actor, path, time range) | Team |
| 42 | `zvault_policy_check` | Check if a token has a specific capability on a path | Pro |
| 43 | `zvault_token_create` | Create a scoped token with specific policies and TTL | Team |
| 44 | `zvault_lease_list` | List active leases with expiry times | Pro |
| 45 | `zvault_lease_revoke` | Revoke a specific lease | Pro |
| 46 | `zvault_seal` | Seal the vault (emergency) | Enterprise |
| 47 | `zvault_snapshot` | Create a point-in-time backup of the vault | Enterprise |
| 48 | `zvault_compare_envs` | Compare two `.env.zvault` files (diff which keys exist) | Free |
| 49 | `zvault_secret_history` | Show version history of a secret (timestamps, not values) | Pro |
| 50 | `zvault_webhook_test` | Send a test webhook to a configured notification endpoint | Team |

---

## Implementation Notes

### Security Invariant

Every proxy tool follows the same pattern:

1. AI provides the tool name, parameters, and a `secret_path`
2. ZVault resolves the credential from the vault (AI never sees it)
3. ZVault connects to the service using the real credential
4. ZVault executes the operation
5. ZVault returns the result to the AI (scrubbed of any credential leaks)

The AI never sees connection strings, API keys, passwords, or tokens.

### Write Safety

All database tools default to read-only. Write operations require explicit `allow_write: true`. This prevents accidental data modification by AI assistants.

### Timeouts

Every proxy tool has a 30-second timeout. Long-running queries are killed.

### Response Limits

- SQL results: max 500 rows (configurable via `max_rows`)
- HTTP responses: truncated at 10KB
- S3 objects: max 1MB for read operations
- Redis: max 1000 keys for KEYS command

### Dependencies Per Tool

| Tool Group | Rust Crate |
|------------|------------|
| PostgreSQL | `tokio-postgres` + `postgres-native-tls` |
| MySQL | `mysql_async` |
| MongoDB | `mongodb` |
| Redis | `redis` (async) |
| S3/R2 | `aws-sdk-s3` or `rusoto_s3` |
| ClickHouse | `clickhouse-rs` |
| MeiliSearch | `meilisearch-sdk` |
| RabbitMQ | `lapin` or HTTP management API via `reqwest` |
| SSH | `russh` |
| gRPC | `tonic` |
| Docker | HTTP API via `reqwest` |
| K8s | `kube-rs` |

---

## Build Order

```
v0.1.0  ✅  Tools 1–10   (core + postgres/http/health)
v0.2.0  🔜  Tools 11–20  (redis, mysql, mongo, s3, clickhouse, meilisearch, rabbitmq)
v0.3.0  📋  Tools 31–36  (pg introspection, redis monitor, grpc, jwt)
v0.4.0  📋  Tools 21–30  (ssh, smtp, graphql, docker, k8s)
v0.5.0  📋  Tools 37–50  (vault management, audit, enterprise)
```

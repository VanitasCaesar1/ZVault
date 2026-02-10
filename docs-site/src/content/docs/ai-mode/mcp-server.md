---
title: MCP Server
description: The Model Context Protocol server that gives AI tools safe access to secret metadata.
sidebar:
  order: 2
---

ZVault ships an MCP server that AI coding assistants connect to via stdio JSON-RPC 2.0. It exposes vault operations as tools while enforcing a strict security invariant: actual secret values are never returned to the LLM.

## Starting the Server

```bash
zvault mcp-server
```

The server reads JSON-RPC messages from stdin and writes responses to stdout. It's designed to be launched by your IDE as a child process.

Requires a Pro license. Set `VAULT_ADDR` and `VAULT_TOKEN` in your environment.

## Available Tools

### zvault_list_secrets

List secret key names under a path. Returns paths only, never values.

```json
{
  "name": "zvault_list_secrets",
  "arguments": { "path": "env/myapp" }
}
```

Response:
```
Secrets under 'env/myapp' (3 keys):
  • STRIPE_KEY
  • DATABASE_URL
  • JWT_SECRET
```

### zvault_describe_secret

Get metadata about a secret — version, timestamps, key names. Never returns values.

```json
{
  "name": "zvault_describe_secret",
  "arguments": { "path": "env/myapp/STRIPE_KEY" }
}
```

Response:
```
Secret: env/myapp/STRIPE_KEY
  Keys: value
  Current version: 1
  Created: 2026-01-15T10:30:00Z
  Values: [REDACTED — use `zvault run` to inject at runtime]
```

### zvault_check_env

Check which `zvault://` references in a `.env.zvault` file can be resolved.

```json
{
  "name": "zvault_check_env",
  "arguments": { "file_path": ".env.zvault" }
}
```

Response:
```
Environment check for .env.zvault:
  ✓ STRIPE_KEY → resolved
  ✓ DATABASE_URL → resolved
  ✗ NEW_SERVICE_KEY → key 'NEW_SERVICE_KEY' not found in secret

Summary: 2 resolved, 1 failed
```

### zvault_generate_env_template

Generate a `.env.zvault` template from secrets stored under a project path.

```json
{
  "name": "zvault_generate_env_template",
  "arguments": { "project": "myapp" }
}
```

### zvault_set_secret

Store a secret value in the vault.

```json
{
  "name": "zvault_set_secret",
  "arguments": { "path": "env/myapp/API_KEY", "value": "sk_..." }
}
```

Response confirms storage without echoing the value:
```
Secret stored at 'env/myapp/API_KEY' (key: API_KEY). Value: [REDACTED]
```

### zvault_delete_secret

Delete a secret from the vault.

```json
{
  "name": "zvault_delete_secret",
  "arguments": { "path": "env/myapp/OLD_KEY" }
}
```

### zvault_vault_status

Check vault health — sealed/unsealed, initialized state.

```json
{
  "name": "zvault_vault_status",
  "arguments": {}
}
```

## Security Model

The MCP server enforces these invariants:

1. `zvault_list_secrets` returns paths only, never values
2. `zvault_describe_secret` returns metadata (version, timestamps, key names) but redacts all values
3. `zvault_set_secret` confirms storage without echoing the value back
4. No tool exists to read actual secret values — that's by design
5. Secrets are only injected into child processes via `zvault run`

## Protocol

The server implements MCP protocol version `2024-11-05` over newline-delimited JSON-RPC 2.0 on stdio. It handles:

- `initialize` — MCP handshake
- `notifications/initialized` — Post-handshake notification
- `tools/list` — Returns all tool definitions with input schemas
- `tools/call` — Executes a tool and returns results

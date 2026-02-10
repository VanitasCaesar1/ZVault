---
title: KV Secrets Engine
description: HTTP API for the KV v2 secrets engine.
sidebar:
  order: 2
---

The KV v2 secrets engine stores arbitrary key-value pairs with versioning.

## Write a Secret

```
POST /v1/secret/data/<path>
```

```bash
curl -X POST -H "X-Vault-Token: $VAULT_TOKEN" \
  -d '{"data": {"username": "admin", "password": "s3cret"}}' \
  http://127.0.0.1:8200/v1/secret/data/myapp/db
```

## Read a Secret

```
GET /v1/secret/data/<path>
```

```bash
curl -H "X-Vault-Token: $VAULT_TOKEN" \
  http://127.0.0.1:8200/v1/secret/data/myapp/db
```

Response:
```json
{
  "data": {
    "username": "admin",
    "password": "s3cret"
  },
  "lease_id": "",
  "lease_duration": 0
}
```

## Delete a Secret

```
DELETE /v1/secret/data/<path>
```

```bash
curl -X DELETE -H "X-Vault-Token: $VAULT_TOKEN" \
  http://127.0.0.1:8200/v1/secret/data/myapp/db
```

## List Secrets

```
GET /v1/secret/list/<prefix>
```

```bash
curl -H "X-Vault-Token: $VAULT_TOKEN" \
  http://127.0.0.1:8200/v1/secret/list/myapp
```

Response:
```json
{
  "keys": ["db", "api-keys", "config"]
}
```

## Secret Metadata

```
GET /v1/secret/metadata/<path>
```

Returns version info, timestamps, and key names without values.

```json
{
  "current_version": 3,
  "created_time": "2026-01-15T10:30:00Z",
  "updated_time": "2026-02-10T14:22:00Z"
}
```

## Path Rules

- Paths must match `^[a-zA-Z0-9_\-/]+$`
- Maximum depth: 10 segments
- Maximum value size: 1MB
- Use `/` as separator

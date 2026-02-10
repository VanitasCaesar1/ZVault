---
title: System Endpoints
description: HTTP API for vault system operations — health, init, seal, unseal.
sidebar:
  order: 3
---

System endpoints manage the vault lifecycle. Most are unauthenticated since they're needed before a token exists.

## Health Check

```
GET /v1/sys/health
```

No authentication required.

```bash
curl http://127.0.0.1:8200/v1/sys/health
```

```json
{
  "initialized": true,
  "sealed": false,
  "threshold": 3,
  "shares": 5,
  "progress": 0
}
```

## Initialize

```
POST /v1/sys/init
```

Initialize a new vault with Shamir's Secret Sharing. Can only be called once.

```bash
curl -X POST \
  -d '{"shares": 5, "threshold": 3}' \
  http://127.0.0.1:8200/v1/sys/init
```

```json
{
  "unseal_shares": [
    "base64-share-1...",
    "base64-share-2...",
    "base64-share-3...",
    "base64-share-4...",
    "base64-share-5..."
  ],
  "root_token": "hvs.root-token-here"
}
```

Parameters:
- `shares` (1-10): Number of unseal key shares to generate
- `threshold` (1 to shares): Minimum shares required to unseal

## Unseal

```
POST /v1/sys/unseal
```

Submit an unseal key share. Must be called `threshold` times with different shares.

```bash
curl -X POST \
  -d '{"share": "base64-share-1..."}' \
  http://127.0.0.1:8200/v1/sys/unseal
```

```json
{
  "sealed": true,
  "threshold": 3,
  "progress": 1
}
```

When enough shares are submitted:

```json
{
  "sealed": false,
  "threshold": 3,
  "progress": 0
}
```

## Seal

```
POST /v1/sys/seal
```

Requires authentication. Seals the vault, zeroizing all key material from memory.

```bash
curl -X POST -H "X-Vault-Token: $VAULT_TOKEN" \
  http://127.0.0.1:8200/v1/sys/seal
```

## Policies

```
GET    /v1/sys/policies          # List all policies
GET    /v1/sys/policies/<name>   # Read a policy
POST   /v1/sys/policies/<name>   # Create/update a policy
DELETE /v1/sys/policies/<name>   # Delete a policy
```

Policy format:
```json
{
  "rules": [
    {
      "path": "secret/data/myapp/*",
      "capabilities": ["read", "list"]
    }
  ]
}
```

Capabilities: `read`, `write`, `delete`, `list`, `sudo`.

---
title: Railway
description: Deploy ZVault on Railway with one click.
sidebar:
  order: 2
---

## One-Click Deploy

Deploy ZVault to Railway with persistent storage:

[![Deploy on Railway](https://railway.app/button.svg)](https://railway.app/template/zvault)

## Manual Setup

1. Create a new project on [Railway](https://railway.app)
2. Add a new service from GitHub repo
3. Set the build command and start command:

```toml
# railway.toml
[build]
builder = "dockerfile"

[deploy]
healthcheckPath = "/v1/sys/health"
healthcheckTimeout = 10
restartPolicyType = "on_failure"
restartPolicyMaxRetries = 3
```

4. Add a persistent volume mounted at `/data`
5. Set environment variables:

```
ZVAULT_LISTEN_ADDR=0.0.0.0:8200
ZVAULT_STORAGE_PATH=/data
RUST_LOG=info
```

## After Deployment

1. Get your Railway URL (e.g., `https://zvault-production.up.railway.app`)
2. Initialize the vault:

```bash
export VAULT_ADDR=https://zvault-production.up.railway.app
zvault init --shares 5 --threshold 3
```

3. Save the unseal keys and root token securely
4. Unseal the vault:

```bash
zvault unseal --share <key-1>
zvault unseal --share <key-2>
zvault unseal --share <key-3>
```

## Important Notes

- Railway provides HTTPS by default — no TLS configuration needed
- Attach a persistent volume to avoid data loss on redeploys
- The vault seals on restart — you'll need to unseal after each deploy
- Store unseal keys in a separate secure location (not in Railway)

# zvault-server

HTTP server for [ZVault](https://zvault.cloud) — the AI-native secrets manager.

REST API, web dashboard, and system routes powered by Axum + Tower.

## Endpoints

- `POST /v1/sys/init` — Initialize vault with Shamir shares
- `POST /v1/sys/unseal` — Submit unseal share
- `POST /v1/sys/seal` — Seal the vault
- `GET /v1/sys/seal-status` — Seal status
- `GET /v1/sys/health` — Health check
- `POST /v1/kv/data/{path}` — Write a secret
- `GET /v1/kv/data/{path}` — Read a secret
- `GET /v1/kv/metadata/{path}` — List secrets
- `POST /v1/transit/encrypt/{key}` — Encrypt data
- `POST /v1/transit/decrypt/{key}` — Decrypt data
- `/ui/*` — Web dashboard (React SPA)

## Quick Start

```bash
# In-memory (dev)
cargo run --package zvault-server

# Persistent (production)
ZVAULT_STORAGE=rocksdb ZVAULT_STORAGE_PATH=/var/lib/zvault \
  cargo run --release --package zvault-server
```

## Part of ZVault

Install the full CLI: `cargo install zvault-cli`

[Website](https://zvault.cloud) · [Docs](https://docs.zvault.cloud) · [GitHub](https://github.com/VanitasCaesar1/zvault)

<p align="center">
  <img src="https://img.shields.io/badge/rust-2024_edition-D4A843?style=flat-square&logo=rust&logoColor=white" alt="Rust 2024"/>
  <img src="https://img.shields.io/badge/crypto-AES--256--GCM-6B8E4E?style=flat-square" alt="AES-256-GCM"/>
  <img src="https://img.shields.io/badge/license-MIT%20%2F%20Apache--2.0-B8860B?style=flat-square" alt="License"/>
</p>

<h1 align="center">VaultRS</h1>
<p align="center">
  A secrets management platform written entirely in Rust.<br/>
  Takes the security architecture of HashiCorp Vault and ships it as a single static binary.
</p>

---

## What is VaultRS?

VaultRS is an in-house secrets manager that encrypts everything at rest using AES-256-GCM, protects the root key with Shamir's Secret Sharing, and serves a full HTTP API + web dashboard — all from one binary with zero external dependencies.

**Key properties:**
- Every byte in storage is encrypted (barrier pattern)
- Root key never touches disk in plaintext
- Shamir unseal with configurable threshold
- Pure Rust crypto (RustCrypto ecosystem, no OpenSSL)
- Embedded storage (RocksDB or redb)
- Built-in web UI with golden "treasure chest" theme

## Architecture

```
Clients (CLI, SDK, Web UI, K8s Operator)
         │
         ▼
   ┌─────────────┐
   │  HTTP API   │  Axum + Tower
   │  + Web UI   │
   └──────┬──────┘
          │
   ┌──────▼──────┐
   │   Barrier   │  AES-256-GCM encrypt/decrypt
   └──────┬──────┘
          │  ciphertext only below
   ┌──────▼──────┐
   │   Storage   │  RocksDB (default) or redb
   └─────────────┘
```

## Quick Start

```bash
# Build
cargo build --release --package vaultrs-server

# Run (in-memory storage for dev)
./target/release/vaultrs-server

# Open the web UI
open http://127.0.0.1:8200

# Or use the CLI
cargo build --release --package vaultrs-cli
./target/release/vaultrs-cli status
./target/release/vaultrs-cli init --shares 5 --threshold 3
./target/release/vaultrs-cli unseal <share>
```

## Configuration

All configuration is via environment variables:

| Variable | Default | Description |
|----------|---------|-------------|
| `PORT` | — | Bind port (Railway convention, binds to `0.0.0.0`) |
| `VAULTRS_BIND_ADDR` | `127.0.0.1:8200` | Full bind address (overrides `PORT`) |
| `VAULTRS_STORAGE` | `memory` | Storage backend: `memory`, `rocksdb`, `redb` |
| `VAULTRS_STORAGE_PATH` | `./data` | Path for persistent storage backends |
| `VAULTRS_LOG_LEVEL` | `info` | Log level: `debug`, `info`, `warn`, `error` |
| `VAULTRS_AUDIT_FILE` | — | Path to audit log file (optional) |
| `VAULTRS_ENABLE_TRANSIT` | `true` | Mount the transit encryption engine |
| `VAULTRS_LEASE_SCAN_INTERVAL` | `60` | Seconds between lease expiry scans |
| `VAULTRS_DISABLE_MLOCK` | `false` | Skip `mlockall` (for dev/containers) |

## Crate Structure

```
Z-vault/
├── crates/
│   ├── vaultrs-core/       # Barrier, seal, tokens, policies, audit, engines
│   ├── vaultrs-storage/    # StorageBackend trait + RocksDB/redb/memory impls
│   ├── vaultrs-server/     # HTTP server, routes, middleware, web UI
│   └── vaultrs-cli/        # Standalone CLI client (HTTP only)
├── docs/
│   └── DESIGN.md           # Full design document
├── Cargo.toml              # Workspace root
├── Dockerfile              # Multi-stage production build
├── railway.toml            # Railway deployment config
└── railpack.toml           # Railpack build config
```

## Secrets Engines

| Engine | Path | Description |
|--------|------|-------------|
| KV v2 | `secret/` | Versioned key-value secrets with metadata |
| Transit | `transit/` | Encryption-as-a-service (encrypt, decrypt, sign, verify) |
| Database | `database/` | Dynamic credentials with automatic revocation (planned) |
| PKI | `pki/` | X.509 certificate authority (planned) |

## Auth Methods

| Method | Description |
|--------|-------------|
| Token | Built-in token auth with policies and TTL |
| AppRole | Machine-to-machine auth (planned) |
| OIDC | OpenID Connect via external IdP (planned) |
| Kubernetes | Service account JWT validation (planned) |

## Security Model

- **Encryption barrier**: All data passes through AES-256-GCM before reaching storage. Storage backends never see plaintext.
- **Shamir unseal**: The root key is encrypted by an unseal key, which is split into N shares. T shares are required to reconstruct it.
- **Key zeroization**: All key material implements `Zeroize` + `ZeroizeOnDrop`. Memory is locked with `mlock` to prevent swapping.
- **Constant-time comparison**: Token verification uses `subtle::ConstantTimeEq` to prevent timing attacks.
- **Audit logging**: Every operation is logged before the response is sent. If all audit backends fail, the request is denied (fail-closed).
- **No unsafe crypto**: Pure Rust via the RustCrypto ecosystem. No OpenSSL, no ring.
- **Core dump prevention**: `RLIMIT_CORE` set to 0 on startup.

## API Reference

### System

```
POST /v1/sys/init          Initialize vault (generate root key + unseal shares)
POST /v1/sys/unseal        Submit an unseal share
POST /v1/sys/seal          Seal the vault
GET  /v1/sys/seal-status   Get seal status
GET  /v1/sys/health        Health check
```

### Secrets (KV v2)

```
GET    /v1/secret/data/:path     Read a secret
POST   /v1/secret/data/:path     Write a secret
DELETE /v1/secret/data/:path     Soft-delete a secret
GET    /v1/secret/metadata/:path Read secret metadata
POST   /v1/secret/destroy/:path  Hard-destroy versions
GET    /v1/secret/list/:prefix   List secrets
```

### Transit

```
POST /v1/transit/keys/:name       Create an encryption key
GET  /v1/transit/keys/:name       Read key info
POST /v1/transit/encrypt/:name    Encrypt plaintext
POST /v1/transit/decrypt/:name    Decrypt ciphertext
POST /v1/transit/sign/:name       Sign data
POST /v1/transit/verify/:name     Verify signature
POST /v1/transit/hash             Hash data
POST /v1/transit/random/:bytes    Generate random bytes
```

### Auth

```
POST /v1/auth/token/create    Create a new token
GET  /v1/auth/token/lookup     Lookup token info
POST /v1/auth/token/renew      Renew a token
POST /v1/auth/token/revoke     Revoke a token
```

### Policies

```
GET    /v1/sys/policies          List policies
GET    /v1/sys/policies/:name    Read a policy
POST   /v1/sys/policies/:name    Create/update a policy
DELETE /v1/sys/policies/:name    Delete a policy
```

### Leases

```
GET  /v1/sys/leases           List active leases
POST /v1/sys/leases/renew     Renew a lease
POST /v1/sys/leases/revoke    Revoke a lease
```

### Mounts

```
GET    /v1/sys/mounts          List engine mounts
POST   /v1/sys/mounts/:path    Mount a new engine
DELETE /v1/sys/mounts/:path    Unmount an engine
```

## Deployment

### Railway (recommended)

The project includes `railway.toml` and `railpack.toml` for one-click Railway deployment.

Set these environment variables in the Railway dashboard:

```
VAULTRS_STORAGE=rocksdb
VAULTRS_STORAGE_PATH=/data
VAULTRS_DISABLE_MLOCK=true
VAULTRS_LOG_LEVEL=info
```

Railway auto-assigns `PORT` and the server binds to `0.0.0.0:$PORT` automatically.

### Docker

```bash
docker build -t vaultrs .
docker run -p 8200:8200 \
  -e VAULTRS_STORAGE=rocksdb \
  -e VAULTRS_STORAGE_PATH=/data \
  -e VAULTRS_DISABLE_MLOCK=true \
  -v vaultrs-data:/data \
  vaultrs
```

### Binary

```bash
cargo build --release --package vaultrs-server
VAULTRS_STORAGE=rocksdb VAULTRS_STORAGE_PATH=/var/lib/vaultrs ./target/release/vaultrs-server
```

## Development

```bash
# Run with in-memory storage (no persistence)
cargo run --package vaultrs-server

# Run tests
cargo test --workspace

# Clippy (strict — unwrap/expect/panic are denied)
cargo clippy --workspace

# Format
cargo fmt --all
```

## License

Dual-licensed under MIT and Apache 2.0. See `Cargo.toml` for details.

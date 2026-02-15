<p align="center">
  <img src="https://img.shields.io/badge/rust-2024_edition-D4A843?style=flat-square&logo=rust&logoColor=white" alt="Rust 2024"/>
  <img src="https://img.shields.io/badge/crypto-AES--256--GCM-6B8E4E?style=flat-square" alt="AES-256-GCM"/>
  <img src="https://img.shields.io/badge/license-MIT%20%2F%20Apache--2.0-B8860B?style=flat-square" alt="License"/>
</p>

<h1 align="center">⟐ ZVault</h1>
<p align="center">
  <strong>Stop leaking secrets to LLMs.</strong><br/>
  The AI-native secrets manager. Let Cursor, Copilot, and Kiro build your app without seeing your API keys.
</p>

<p align="center">
  <a href="https://zvault.cloud">Website</a> ·
  <a href="https://docs.zvault.cloud">Docs</a> ·
  <a href="https://zvault.cloud/#pricing">Pricing</a> ·
  <a href="https://docs.zvault.cloud/getting-started/quickstart">Quick Start</a>
</p>

---

## The Problem

Every AI coding tool reads your `.env` file. That means your `STRIPE_SECRET_KEY`, `DATABASE_URL`, and `AWS_ACCESS_KEY` are sitting in someone else's context window.

ZVault fixes this. Your AI sees `zvault://payments/stripe-key` — a reference, not the value. At runtime, ZVault injects the real secrets. Nothing leaked.

## Install

```bash
# One-liner (macOS/Linux)
curl -fsSL https://zvault.cloud/install.sh | sh

# Homebrew
brew tap VanitasCaesar1/tap
brew install zvault

# Cargo (crates.io)
cargo install zvault-cli

# Or build from source
cargo install --git https://github.com/VanitasCaesar1/zvault zvault-cli
```

## How It Works

```bash
# 1. Import your .env (secrets encrypted, references generated)
zvault import .env
# ✓ Imported 12 secrets into vault
# ✓ Created .env.zvault (safe for git)
# ✓ Added .env to .gitignore

# 2. Your AI now sees references, not values
cat .env.zvault
# STRIPE_KEY=zvault://env/myapp/STRIPE_KEY
# DATABASE_URL=zvault://env/myapp/DATABASE_URL

# 3. Run your app — secrets injected at runtime
zvault run -- npm run dev
# ✓ 12 secrets resolved · ▶ npm run dev
```

## AI Mode (Pro)

The killer feature. Connect your IDE's AI to ZVault via MCP — it can query what secrets exist without ever seeing values.

```bash
# Setup for your IDE (one command)
zvault setup cursor    # or: kiro, continue, generic

# Starts an MCP server with 10 tools:
# Vault operations (Free):
# - zvault_list_secrets       (names only, never values)
# - zvault_describe_secret    (metadata, type, last rotated)
# - zvault_check_env          (verify all required secrets exist)
# - zvault_generate_env       (generate .env.example from vault)
# - zvault_set_secret         (store a secret)
# - zvault_delete_secret      (delete a secret)
# - zvault_vault_status       (seal status, health)
#
# Secure proxy tools (Pro — AI never sees credentials):
# - zvault_query_database     (SQL via vault-stored Postgres creds)
# - zvault_http_request       (HTTP with zvault:// refs in headers/URL)
# - zvault_check_service      (health-check postgres/redis/http)
```

## Features

| Feature | Free | Pro ($8/mo) |
|---------|------|-------------|
| Local encrypted vault | ✅ | ✅ |
| CLI (init, import, run) | ✅ | ✅ |
| .env import/export | ✅ | ✅ |
| Web dashboard | ✅ | ✅ |
| KV, Transit, PKI engines | ✅ | ✅ |
| AI Mode (MCP server) | — | ✅ |
| zvault:// references | — | ✅ |
| IDE setup (Cursor, Kiro, Continue) | — | ✅ |
| llms.txt generation | — | ✅ |


## Security Model

- **AES-256-GCM** encryption at rest (barrier pattern — storage never sees plaintext)
- **Shamir's Secret Sharing** for root key protection
- **Key zeroization** via `Zeroize` + `ZeroizeOnDrop` on all key material
- **Constant-time comparison** for token verification (`subtle::ConstantTimeEq`)
- **Fail-closed audit** — if audit logging fails, the request is denied
- **Pure Rust crypto** — RustCrypto ecosystem, no OpenSSL, no C dependencies
- **Core dump prevention** — `RLIMIT_CORE` set to 0, memory locked with `mlock`

## Architecture

```
Clients (CLI, Web UI, MCP Server)
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
          │  ciphertext only below this line
   ┌──────▼──────┐
   │   Storage   │  RocksDB (default) or redb
   └─────────────┘
```

## CLI Reference

```bash
zvault status                          # Vault health + seal status
zvault init --shares 5 --threshold 3   # Initialize with Shamir
zvault unseal --share <key>            # Submit unseal share
zvault seal                            # Seal (zeroize all keys)

zvault kv put myapp/config key=value   # Write a secret
zvault kv get myapp/config             # Read a secret
zvault kv list myapp/                  # List secrets
zvault kv delete myapp/config          # Delete a secret

zvault transit create-key my-key       # Create encryption key
zvault transit encrypt my-key <b64>    # Encrypt data
zvault transit decrypt my-key <ct>     # Decrypt data

zvault import .env                     # Import .env → vault + .env.zvault
zvault run -- npm run dev              # Run with secrets injected

zvault mcp-server                      # Start MCP server (Pro)
zvault setup cursor                    # Configure IDE (Pro)
zvault activate <license-key>          # Activate Pro/Team/Enterprise
zvault license                         # Show license status
```

## Self-Hosting

### Quick (in-memory, for dev)

```bash
cargo run --package zvault-server
# → http://127.0.0.1:8200 (API + Web UI)
```

### Production (persistent storage)

```bash
cargo build --release --package zvault-server
ZVAULT_STORAGE=rocksdb ZVAULT_STORAGE_PATH=/var/lib/zvault ./target/release/zvault-server
```

### Docker

```bash
docker build -t zvault .
docker run -p 8200:8200 \
  -e ZVAULT_STORAGE=rocksdb \
  -e ZVAULT_STORAGE_PATH=/data \
  -e ZVAULT_DISABLE_MLOCK=true \
  -v zvault-data:/data \
  zvault
```

### Railway

One-click deploy with `railway.toml` included. Set `ZVAULT_STORAGE=rocksdb` and `ZVAULT_STORAGE_PATH=/data`.

## Configuration

| Variable | Default | Description |
|----------|---------|-------------|
| `PORT` | — | Bind port (Railway convention) |
| `ZVAULT_BIND_ADDR` | `127.0.0.1:8200` | Full bind address |
| `ZVAULT_STORAGE` | `memory` | `memory`, `rocksdb`, or `redb` |
| `ZVAULT_STORAGE_PATH` | `./data` | Path for persistent storage |
| `ZVAULT_LOG_LEVEL` | `info` | `debug`, `info`, `warn`, `error` |
| `ZVAULT_AUDIT_FILE` | — | Audit log file path |
| `ZVAULT_DISABLE_MLOCK` | `false` | Skip `mlockall` (for containers) |

## Crate Structure

```
Z-vault/
├── crates/
│   ├── zvault-core/       # Barrier, seal, tokens, policies, audit, engines
│   ├── zvault-storage/    # StorageBackend trait + RocksDB/redb/memory
│   ├── zvault-server/     # HTTP server, routes, middleware, web UI
│   └── zvault-cli/        # Standalone CLI (HTTP client, MCP server, license)
├── dashboard/              # React SPA (served by server at /ui)
├── website/                # Landing page (zvault.cloud)
├── docs-site/              # Documentation (docs.zvault.cloud)
└── docs/                   # Design docs, roadmap, monetization
```

## Development

```bash
cargo run --package zvault-server     # Run server (dev)
cargo test --workspace                 # Run tests
cargo clippy --workspace               # Lint (strict)
cargo fmt --all                        # Format
```

## License

Dual-licensed under MIT and Apache 2.0.

---

<p align="center">
  <a href="https://zvault.cloud">zvault.cloud</a> · <a href="https://docs.zvault.cloud">docs</a> · <a href="https://github.com/VanitasCaesar1/zvault">github</a>
</p>

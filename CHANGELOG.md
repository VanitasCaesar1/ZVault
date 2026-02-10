# Changelog

All notable changes to ZVault will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.0] - 2026-02-11

### Added

- Core vault with AES-256-GCM barrier encryption
- Shamir's Secret Sharing for root key protection (configurable shares/threshold)
- Seal/unseal lifecycle with key zeroization
- KV secrets engine (put, get, list, delete)
- Transit encryption engine (create-key, encrypt, decrypt, rotate)
- PKI engine (root CA, intermediate CA, certificate issuance)
- Token-based authentication with policies
- ACL policy engine (path-based, capabilities: create/read/update/delete/list)
- HMAC-based audit logging with fail-closed semantics
- Storage backends: RocksDB (default), redb, in-memory
- HTTP API server (Axum) with full REST endpoints
- Web dashboard (React SPA served at `/ui`)
- CLI client (`zvault`) with all vault operations
- `zvault import .env` — import .env files into vault, generate `.env.zvault` references
- `zvault run -- <cmd>` — resolve `zvault://` URIs and inject as env vars
- AI Mode (Pro): MCP server with 7 tools (list, describe, check, generate, stats, search, run)
- IDE setup: `zvault setup cursor|kiro|continue|generic` with smart config merge
- License system: Ed25519-signed keys + Polar.sh API validation
- `zvault activate <key>` and `zvault license` commands
- Security hardening: `RLIMIT_CORE=0`, `mlockall`, constant-time token comparison
- Pure Rust cryptography (RustCrypto ecosystem, no OpenSSL)
- Docker support with `Dockerfile`
- Railway one-click deploy with `railway.toml`
- Landing page at zvault.cloud (Astro)
- Documentation site at docs.zvault.cloud (Astro Starlight, 18 pages)
- Install script (`curl -fsSL https://zvault.cloud/install.sh | sh`)
- npm wrapper (`npx zvault`)
- Homebrew formula
- GitHub Actions CI (fmt, clippy, test, audit) and Release workflows

### Security

- AES-256-GCM with fresh nonce per operation
- Shamir's Secret Sharing (RFC-compatible)
- Key material zeroized on drop (`Zeroize` + `ZeroizeOnDrop`)
- Constant-time token comparison (`subtle::ConstantTimeEq`)
- Core dump prevention (`RLIMIT_CORE=0`)
- Memory locking (`mlockall`) to prevent swap
- Fail-closed audit logging
- No unsafe code (`#![deny(unsafe_code)]`)

[Unreleased]: https://github.com/zvault/zvault/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/zvault/zvault/releases/tag/v0.1.0

---
inclusion: manual
---

# Z-Vault (VaultRS)

Rust secrets management system. CLI + server + storage backends. HashiCorp Vault alternative built in Rust.

## Project Structure

- `crates/zvault-core/` — Core crypto, seal/unseal, encryption barrier
- `crates/zvault-server/` — Axum HTTP server, API handlers
- `crates/zvault-storage/` — Storage backends (RocksDB, Redb)
- `crates/zvault-cli/` — CLI tool
- `src/` — Workspace-level code
- `dashboard/` — Web dashboard
- `docs/` — Documentation
- `docs-site/` — Documentation website
- `sdks/` — Client SDKs
- `migrations/` — Database migrations
- `website/` — Marketing website

## Key Commands

```bash
cargo run --bin zvault-server    # Run server
cargo run --bin zvault           # CLI
cargo build --release            # Release build
cargo test                       # Run tests
cargo clippy                     # Lint
cargo fmt                        # Format
```

## Conventions

- Follow Rust Production Code Standards (see `.kiro/steering/rust-code-standards.md`)
- thiserror for library crates, anyhow only in binary crates
- NEVER use .unwrap() or .expect() in production code
- All key material must implement Zeroize + ZeroizeOnDrop
- Use subtle::ConstantTimeEq for token comparison
- RustCrypto ecosystem for all crypto operations, rustls for TLS (no openssl)
- Newtypes for domain concepts (SecretPath, TokenHash, EncryptedPayload)
- Enums for states, not booleans (SealState::Sealed / SealState::Unsealed)
- panic = "abort" in release profile

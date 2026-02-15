# zvault-core

Core library for [ZVault](https://zvault.cloud) — the AI-native secrets manager.

This crate contains the encryption barrier, seal/unseal lifecycle, token authentication, policy engine, audit logging, and secrets engines (KV, Transit, PKI, Database).

## Key Components

- **Barrier** — AES-256-GCM encrypt-on-write, decrypt-on-read. Storage never sees plaintext.
- **Seal** — Shamir's Secret Sharing for root key protection.
- **Tokens** — SHA-256 hashed, constant-time verified (`subtle::ConstantTimeEq`).
- **Policies** — Path-based ACLs with capabilities (create, read, update, delete, list).
- **Audit** — HMAC'd entries, fail-closed semantics.
- **Engines** — KV secrets, Transit encryption, PKI certificates, Database credentials.

## Security

- AES-256-GCM with fresh nonce per operation
- Key material zeroized on drop (`Zeroize` + `ZeroizeOnDrop`)
- Pure Rust crypto (RustCrypto ecosystem, no OpenSSL)
- `#![deny(unsafe_code)]`

## Part of ZVault

```
CLI / MCP Server / Web UI
        │
   ┌────▼────┐
   │  Core   │  ← this crate
   └────┬────┘
   ┌────▼────┐
   │ Storage  │  ← zvault-storage
   └─────────┘
```

Install the full CLI: `cargo install zvault-cli`

[Website](https://zvault.cloud) · [Docs](https://docs.zvault.cloud) · [GitHub](https://github.com/VanitasCaesar1/zvault)

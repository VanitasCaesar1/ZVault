---
title: Threat Model
description: What ZVault protects against and its security boundaries.
sidebar:
  order: 3
---

## What ZVault Protects Against

### Secrets in LLM Context Windows

The primary threat ZVault addresses. When AI coding tools read your `.env` file, your secrets are sent to third-party LLM providers. ZVault replaces real values with `zvault://` references — the AI sees the reference, never the value.

### Secrets in Git History

`.env` files accidentally committed to git are the most common source of secret leaks. ZVault's `.env.zvault` file contains only references and is safe to commit.

### Unauthorized Access

- Token-based authentication with policy enforcement
- Tokens are hashed before storage (SHA-256)
- Constant-time token comparison prevents timing attacks
- AppRole for machine-to-machine authentication

### Data at Rest

All data in the storage backend is encrypted through the barrier (AES-256-GCM). Even if an attacker gains access to the storage files, they see only ciphertext.

### Memory Forensics

- Key material is zeroized on drop (`Zeroize` + `ZeroizeOnDrop`)
- Memory pages are pinned with `mlock` (no swap to disk)
- Core dumps are disabled (`RLIMIT_CORE=0`)
- Sealing the vault zeroizes all key material from memory

### Audit Evasion

- Audit logging is fail-closed — if the audit backend fails, the request is denied
- Audit entries are HMAC'd for integrity
- Append-only log — no updates or deletes

## Security Boundaries

### Trusted

- The ZVault server process and its memory space
- The operator who holds unseal key shares
- The root token holder (should be short-lived)

### Untrusted

- The storage backend (sees only ciphertext)
- Network traffic (encrypted with TLS 1.3)
- AI coding assistants (see only `zvault://` references and metadata)
- The MCP server responses (never contain actual secret values)

## Known Limitations

- A compromised ZVault server process with an unsealed vault has access to all secrets
- Shamir shares must be distributed through a secure out-of-band channel
- The root token has unlimited access — revoke it after creating scoped tokens
- Single-node deployments have no redundancy (HA clustering is an Enterprise feature)
- `zvault run` injects secrets as environment variables — other processes on the same machine with sufficient privileges could read `/proc/<pid>/environ` on Linux

## Recommendations

1. Run ZVault on a dedicated, hardened host
2. Use the minimum number of unseal shares needed
3. Revoke the root token after initial setup
4. Create scoped tokens with minimal policies
5. Enable audit logging and monitor for anomalies
6. Back up unseal shares in separate secure locations
7. Rotate secrets regularly using the transit engine

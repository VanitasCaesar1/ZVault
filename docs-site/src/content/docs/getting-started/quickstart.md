---
title: Quick Start
description: Get up and running with ZVault in under 2 minutes.
sidebar:
  order: 3
---

## 1. Initialize the Vault

```bash
zvault init
# ✓ Vault initialized with 5 key shares, threshold 3
# ✓ Unseal Key 1: abc123...
# ✓ Unseal Key 2: def456...
# ✓ Unseal Key 3: ghi789...
# ✓ Unseal Key 4: jkl012...
# ✓ Unseal Key 5: mno345...
# ✓ Root Token: hvs.xxxxxxxx
#
# ⚠ Save these keys securely. They cannot be recovered.
```

## 2. Unseal

```bash
zvault unseal <key-1>
zvault unseal <key-2>
zvault unseal <key-3>
# ✓ Vault unsealed
```

## 3. Import Your .env

```bash
zvault import .env
# ✓ Imported 12 secrets from .env
# ✓ Created .env.zvault (safe for git)
# ✓ Backed up original to .env.backup
# ✓ Added .env to .gitignore
```

## 4. Run Your App

```bash
zvault run -- npm run dev
# All zvault:// URIs resolved → real env vars injected → app starts
```

## 5. Set Up AI Mode (Pro)

```bash
zvault setup cursor   # or: kiro, continue, generic
# ✓ Created .cursor/mcp.json
# ✓ Created .cursor/rules/zvault.mdc
# Your AI assistant can now query secret metadata safely
```

## What Just Happened?

1. Your `.env` secrets are now encrypted in the vault (AES-256-GCM)
2. `.env.zvault` contains `zvault://` references instead of real values
3. `zvault run` resolves references at runtime — your app works normally
4. AI tools see `zvault://myapp/stripe-key`, never `sk_live_51J3...`
5. Full audit trail of every secret access

## Next Steps

- [CLI Reference](/cli/overview) — all available commands
- [AI Mode](/ai-mode/overview) — how the MCP server works
- [API Reference](/api/authentication) — HTTP API docs

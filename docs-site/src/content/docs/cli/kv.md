---
title: zvault kv
description: KV v2 secrets engine operations — read, write, delete, and list secrets.
sidebar:
  order: 5
---

The `zvault kv` commands interact with the KV v2 secrets engine for storing and retrieving key-value secrets.

## Commands

### kv put

Write one or more key-value pairs to a secret path.

```bash
zvault kv put <PATH> <KEY=VALUE>...
```

```bash
zvault kv put myapp/config db_host=10.0.0.1 db_port=5432 db_name=myapp
# ✓ Secret written to myapp/config
```

### kv get

Read a secret by path.

```bash
zvault kv get <PATH>
```

```bash
zvault kv get myapp/config
# 📦 Secret: myapp/config
# ─────────────────────────────────────────
#   db_host              10.0.0.1
#   db_port              5432
#   db_name              myapp
```

### kv delete

Soft-delete a secret.

```bash
zvault kv delete <PATH>
```

```bash
zvault kv delete myapp/old-config
# ✓ Secret at myapp/old-config deleted.
```

### kv list

List secret keys under a prefix.

```bash
zvault kv list <PATH>
```

```bash
zvault kv list myapp
# 📂 Keys: myapp
# ─────────────────────────────────────────
#   ├─ config
#   ├─ credentials
#   ├─ api-keys
```

## Path Conventions

Secrets are organized by path. Use `/` as a separator:

```
myapp/config          → Application configuration
myapp/credentials     → Database credentials
myapp/api-keys        → Third-party API keys
staging/myapp/config  → Environment-specific
```

When using `zvault import`, secrets are stored under `env/<project>/<KEY>`.

## Multiple Values

A single path can hold multiple key-value pairs:

```bash
zvault kv put payments/stripe public_key=pk_live_... secret_key=sk_live_...
zvault kv get payments/stripe
# public_key    pk_live_...
# secret_key    sk_live_...
```

---
title: zvault import
description: Import secrets from a .env file into the vault.
sidebar:
  order: 2
---

`zvault import` reads a `.env` file, stores each secret in the vault, and generates a `.env.zvault` file with `zvault://` references.

## Usage

```bash
zvault import [OPTIONS] [FILE]
```

## Arguments

| Argument | Default | Description |
|----------|---------|-------------|
| `FILE` | `.env` | Path to the .env file to import |

## Options

| Flag | Description |
|------|-------------|
| `--project <NAME>` | Project name for namespacing (default: current directory name) |
| `--no-backup` | Skip backing up the original .env file |
| `--no-ref` | Skip generating .env.zvault reference file |
| `--no-gitignore` | Skip adding .env to .gitignore |

## What It Does

1. Parses the `.env` file (supports `KEY=VALUE`, quoted values, `export` prefix, comments)
2. Stores each secret at `env/<project>/<KEY>` in the vault
3. Creates `.env.backup` (copy of original)
4. Creates `.env.zvault` with `zvault://` references instead of real values
5. Adds `.env` to `.gitignore`

## Example

```bash
# Before
cat .env
# STRIPE_KEY=sk_live_51J3...
# DATABASE_URL=postgres://admin:secret@db:5432/app
# JWT_SECRET=super-secret-key

zvault import .env
# ✓ STRIPE_KEY → zvault://env/myapp/STRIPE_KEY
# ✓ DATABASE_URL → zvault://env/myapp/DATABASE_URL
# ✓ JWT_SECRET → zvault://env/myapp/JWT_SECRET
# ✓ Backed up original to .env.backup
# ✓ Created .env.zvault (safe for git)
# ✓ Added .env to .gitignore

cat .env.zvault
# STRIPE_KEY=zvault://env/myapp/STRIPE_KEY
# DATABASE_URL=zvault://env/myapp/DATABASE_URL
# JWT_SECRET=zvault://env/myapp/JWT_SECRET
```

## Custom Project Name

```bash
zvault import .env --project payments-api
# Secrets stored under env/payments-api/*
```

## Supported .env Formats

```bash
# Standard
KEY=value

# Quoted values
KEY="value with spaces"
KEY='single quoted'

# Export prefix (stripped)
export KEY=value

# Comments (skipped)
# This is a comment

# Empty lines (skipped)
```

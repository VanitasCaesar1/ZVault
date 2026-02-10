---
title: zvault run
description: Run a command with secrets injected from the vault.
sidebar:
  order: 3
---

`zvault run` resolves `zvault://` URIs from your `.env.zvault` file and injects the real secret values as environment variables into a child process.

## Usage

```bash
zvault run [OPTIONS] -- <COMMAND> [ARGS...]
```

## Options

| Flag | Description |
|------|-------------|
| `--env-file <PATH>` | Path to env file (default: auto-detect `.env.zvault` or `.env`) |

## How It Works

1. Reads `.env.zvault` (or the file specified by `--env-file`)
2. For each `zvault://` URI, resolves the secret from the vault
3. Plain values (non-zvault URIs) are passed through as-is
4. Spawns the child process with all resolved environment variables
5. The child process runs with real secrets — the LLM never sees them

## Examples

```bash
# Run a Node.js dev server
zvault run -- npm run dev

# Run a Rust binary
zvault run -- cargo run

# Run a Python app
zvault run -- python manage.py runserver

# Run with a specific env file
zvault run --env-file .env.staging.zvault -- npm run dev

# Run a Docker container
zvault run -- docker compose up
```

## Auto-Detection

When no `--env-file` is specified, `zvault run` looks for files in this order:

1. `.env.zvault` (preferred — contains only references)
2. `.env` (fallback — may contain mixed real values and references)

If neither exists, it exits with an error suggesting `zvault import .env`.

## Mixed Files

Your env file can contain both `zvault://` references and plain values:

```bash
# .env.zvault
STRIPE_KEY=zvault://env/myapp/STRIPE_KEY    # Resolved from vault
DATABASE_URL=zvault://env/myapp/DATABASE_URL # Resolved from vault
NODE_ENV=development                          # Passed through as-is
PORT=3000                                     # Passed through as-is
```

## Error Handling

If any `zvault://` URI fails to resolve, the command exits immediately without starting the child process. This prevents your app from running with missing secrets.

```bash
zvault run -- npm run dev
# ✗ STRIPE_KEY — secret not found at env/myapp/STRIPE_KEY
# Error: failed to resolve STRIPE_KEY
```

---
title: What is AI Mode?
description: How ZVault protects your secrets from AI coding assistants.
sidebar:
  order: 1
---

AI Mode is ZVault's core differentiator — a proxy layer that lets LLMs interact with your secrets without ever seeing the actual values.

## The Problem

Every time you use Cursor, Copilot, Kiro, or any AI coding tool, your `.env` file ends up in the LLM's context window. That means your API keys, database passwords, and tokens are sent to third-party servers.

```bash
# What the LLM sees today:
STRIPE_SECRET_KEY=sk_live_51J3xKpQR7...
DATABASE_URL=postgres://admin:s3cret@prod-db:5432/app
AWS_SECRET_ACCESS_KEY=wJalrXUtnFEMI/K7MDENG...
```

## The Solution

With AI Mode, the LLM sees references instead of values:

```bash
# What the LLM sees with ZVault:
STRIPE_SECRET_KEY=zvault://env/myapp/STRIPE_SECRET_KEY
DATABASE_URL=zvault://env/myapp/DATABASE_URL
AWS_SECRET_ACCESS_KEY=zvault://env/myapp/AWS_SECRET_ACCESS_KEY
```

The AI knows the secret exists, knows what it's for, but never sees the actual value.

## How It Works

AI Mode has three components:

### 1. zvault:// URI References

After running `zvault import .env`, your `.env.zvault` file contains references like `zvault://env/myapp/STRIPE_KEY`. These are safe to commit, safe for AI to read.

### 2. MCP Server

The MCP (Model Context Protocol) server lets AI tools query secret metadata through structured tools:

- List what secrets exist
- Check if required secrets are present
- Generate env templates
- Store new secrets

The MCP server never returns actual secret values.

### 3. IDE Integration

One command configures your IDE to use ZVault as an MCP tool provider:

```bash
zvault setup cursor   # Cursor
zvault setup kiro     # Kiro
zvault setup continue # Continue
zvault setup generic  # Any tool (generates llms.txt)
```

## The Developer Flow

```bash
# 1. Import your secrets
zvault import .env

# 2. Set up your IDE
zvault setup cursor

# 3. Code with AI — it sees references, not values
#    AI reads .env.zvault: STRIPE_KEY=zvault://env/myapp/STRIPE_KEY
#    AI uses MCP tools to check what secrets exist

# 4. Run your app — secrets injected at runtime
zvault run -- npm run dev
```

## What the AI Can Do

With the MCP server connected, your AI assistant can:

- Know exactly what secrets your project needs
- Generate correct config files with `zvault://` references
- Verify all required secrets exist before running
- Help you add new secrets to the vault
- Never see or leak the actual secret values

## Pricing

AI Mode requires a Pro license ($8/mo per developer). The free tier includes the local vault, CLI, and .env import — but MCP server and IDE setup require Pro.

```bash
zvault activate <license-key>
```

Get a license at [zvault.cloud/pricing](https://zvault.cloud/pricing).

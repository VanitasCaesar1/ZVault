# zvault-cli

The CLI for [ZVault](https://zvault.cloud) — stop leaking secrets to LLMs.

Import `.env` files into an encrypted vault, inject secrets at runtime, and connect AI coding assistants via MCP — without ever exposing your API keys.

## Install

```bash
cargo install zvault-cli
```

## Quick Start

```bash
# 1. Import your .env (secrets encrypted, references generated)
zvault import .env
# ✓ Imported 12 secrets · Created .env.zvault (safe for git)

# 2. Your AI sees references, not values
cat .env.zvault
# STRIPE_KEY=zvault://env/myapp/STRIPE_KEY
# DATABASE_URL=zvault://env/myapp/DATABASE_URL

# 3. Run your app — secrets injected at runtime
zvault run -- npm run dev
# ✓ 12 secrets resolved · ▶ npm run dev
```

## AI Mode (Pro)

Connect your IDE's AI to ZVault via MCP. It can query databases, make HTTP requests, and check service health — all using vault-stored credentials the AI never sees.

```bash
zvault setup cursor    # or: kiro, continue, generic
zvault mcp-server      # Start MCP server (10 tools)
```

## Commands

```bash
zvault status                          # Vault health
zvault init --shares 3 --threshold 2   # Initialize
zvault unseal --share <key>            # Unseal
zvault seal                            # Seal

zvault kv put myapp/config key=value   # Write secret
zvault kv get myapp/config             # Read secret
zvault kv list myapp/                  # List secrets

zvault import .env                     # Import .env → vault
zvault run -- npm run dev              # Run with secrets

zvault mcp-server                      # MCP server (Pro)
zvault setup cursor                    # IDE setup (Pro)
zvault activate <license-key>          # Activate Pro
```

[Website](https://zvault.cloud) · [Docs](https://docs.zvault.cloud) · [GitHub](https://github.com/VanitasCaesar1/zvault)

# ZVault — Monetization & Go-to-Market Plan

> The AI-native secrets manager. Let LLMs build your app without leaking your keys.

---

## The Problem (Why People Will Pay)

Every developer using AI coding tools faces the same dilemma:

1. **LLMs need secrets to be useful** — Cursor, Copilot, Kiro, Claude, ChatGPT all need to understand your environment to write working code. They read `.env` files, config files, docker-compose files.

2. **But secrets in LLM context = leaked secrets** — Every API key in your `.env` that gets read into an LLM context window is now sitting on someone else's server. OpenAI, Anthropic, Google — they all see your `STRIPE_SECRET_KEY`, your `DATABASE_URL`, your `AWS_ACCESS_KEY`.

3. **The scale of the problem is massive:**
   - GitHub reports 12.8M+ secrets leaked in public repos in 2023
   - GitGuardian found secrets in 1 in 10 code authors' commits
   - LLM-assisted coding is making this 10x worse because devs paste entire `.env` files into chat
   - Companies like Samsung banned ChatGPT after employees leaked source code + secrets

4. **Existing solutions don't solve the LLM problem:**
   - HashiCorp Vault: complex, no LLM awareness, $0.03/secret/month at scale
   - Infisical: SaaS, your secrets on their servers
   - Doppler: SaaS, same problem
   - 1Password: not designed for programmatic access
   - `.env` files: the root cause of the problem

---

## The Solution: ZVault AI Mode

### Core Concept

ZVault introduces an "AI Mode" — a proxy layer that lets LLMs interact with your secrets without ever seeing the actual values.

### How It Works

```
Developer's IDE (Cursor/Kiro/Copilot)
         │
         │  LLM reads .env file
         ▼
┌─────────────────────────────┐
│  .env (ZVault-managed)      │
│                             │
│  STRIPE_KEY=zvault://stripe │  ← References, not values
│  DB_URL=zvault://db-prod    │
│  AWS_KEY=zvault://aws-main  │
└──────────┬──────────────────┘
           │
           │  At runtime, ZVault resolves references
           ▼
┌─────────────────────────────┐
│  ZVault Local Agent         │
│                             │
│  • Resolves zvault:// URIs  │
│  • Injects real values at   │
│    process start time       │
│  • LLM never sees actual    │
│    secret values            │
│  • Full audit trail         │
└─────────────────────────────┘
```

### The Developer Experience

```bash
# 1. Install (one command)
curl -fsSL https://zvault.cloud/install.sh | sh
# or
brew install zvault

# 2. Initialize in your project
zvault init

# 3. Import your existing .env
zvault import .env
# → Stores all secrets in encrypted local vault
# → Replaces .env with zvault:// references
# → Creates .env.zvault (safe to commit, no real values)

# 4. Run your app through ZVault
zvault run -- npm run dev
# → Injects real secrets as env vars at runtime
# → LLM sees "zvault://stripe-key" not "sk_live_..."

# 5. Your LLM can now safely read your project
# It sees: STRIPE_KEY=zvault://payments/stripe-live
# It knows the SECRET EXISTS and what it's FOR
# But it never sees: sk_live_51J3...
```

### MCP Server (Model Context Protocol)

This is the killer feature. ZVault ships an MCP server that AI coding tools can connect to:

```json
// .kiro/settings/mcp.json or .cursor/mcp.json
{
  "mcpServers": {
    "zvault": {
      "command": "zvault",
      "args": ["mcp-server"],
      "env": {}
    }
  }
}
```

The MCP server exposes tools like:
- `zvault_list_secrets` — List secret names (not values)
- `zvault_describe_secret` — Get metadata (type, last rotated, which service uses it)
- `zvault_check_env` — Verify all required secrets exist for a service
- `zvault_generate_env_template` — Generate .env.example from vault
- `zvault_run_with_secrets` — Execute a command with secrets injected

The LLM can now:
- Know exactly what secrets your project needs
- Generate correct config files with zvault:// references
- Run tests and dev servers with real secrets injected
- Never see or leak the actual secret values

### llms.txt Integration

```bash
zvault llms-txt
# Generates a llms.txt file that tells AI tools:
# "This project uses ZVault for secrets management.
#  Never hardcode secrets. Use zvault:// references.
#  Available secrets: stripe-key, db-url, aws-key, ...
#  To run: zvault run -- <command>"
```

---

## Pricing Strategy

### Why NOT the traditional Vault pricing model

HashiCorp charges per secret per month. That works for enterprises with 10,000 secrets.
For indie devs and small teams trying to hit $200/mo, you need volume at low price points.

### The Model: Developer Tool Pricing (like Tailwind, shadcn, Railway)

| Tier | Price | Target | What They Get |
|------|-------|--------|---------------|
| **Open Source** | Free forever | Individual devs, OSS | Local vault, CLI, .env import, single project, community support |
| **Pro** | $8/mo per developer | Freelancers, indie hackers | AI Mode (MCP server), team sharing, 5 projects, secret rotation, priority support |
| **Team** | $19/mo per developer | Startups (5-50 eng) | Everything in Pro + OIDC SSO, audit log export, unlimited projects, Slack alerts |
| **Enterprise** | $49/mo per developer | Mid-market | Everything in Team + HA clustering, K8s operator, namespaces, SLA, dedicated support |

### Revenue Math to $200/mo

You need just:
- 25 Pro users ($8 × 25 = $200), OR
- 11 Team users ($19 × 11 = $209), OR
- 5 Enterprise users ($49 × 5 = $245), OR
- Mix: 10 Pro + 5 Team = $80 + $95 = $175 + a couple more

This is very achievable with the right positioning.

### What's Free vs Paid

| Feature | Free | Pro ($8) | Team ($19) | Enterprise ($49) |
|---------|------|----------|------------|-------------------|
| Local encrypted vault | ✅ | ✅ | ✅ | ✅ |
| CLI (init, import, run) | ✅ | ✅ | ✅ | ✅ |
| .env import/export | ✅ | ✅ | ✅ | ✅ |
| Web dashboard | ✅ | ✅ | ✅ | ✅ |
| KV, Transit, PKI engines | ✅ | ✅ | ✅ | ✅ |
| **AI Mode (MCP server)** | ❌ | ✅ | ✅ | ✅ |
| **zvault:// references** | ❌ | ✅ | ✅ | ✅ |
| **llms.txt generation** | ❌ | ✅ | ✅ | ✅ |
| **Secret rotation** | ❌ | ✅ | ✅ | ✅ |
| Projects | 1 | 5 | Unlimited | Unlimited |
| Team sharing | ❌ | ❌ | ✅ | ✅ |
| OIDC SSO | ❌ | ❌ | ✅ | ✅ |
| Audit log export | ❌ | ❌ | ✅ | ✅ |
| Slack/Discord alerts | ❌ | ❌ | ✅ | ✅ |
| HA clustering | ❌ | ❌ | ❌ | ✅ |
| K8s operator | ❌ | ❌ | ❌ | ✅ |
| Namespaces | ❌ | ❌ | ❌ | ✅ |
| SLA | ❌ | ❌ | ❌ | ✅ |

---

## Implementation Plan (What to Build)

### Phase 1: The Hook (Week 1-2) — FREE, gets users in the door

Build the frictionless onboarding that makes people go "oh shit, this is nice":

```bash
# One command install
curl -fsSL https://zvault.cloud/install.sh | sh

# One command setup in any project
zvault init

# Import existing .env (the magic moment)
zvault import .env
# Output:
# ✓ Imported 12 secrets from .env
# ✓ Created .env.zvault (safe for git)
# ✓ Original .env backed up to .env.backup
# ✓ Added .env to .gitignore
#
# Your secrets are now encrypted at rest.
# Run your app with: zvault run -- npm run dev

# Run your app (secrets injected at runtime)
zvault run -- npm run dev
```

What this does technically:
1. `zvault init` — starts a local ZVault server in the background (or uses an existing one), creates a project namespace
2. `zvault import .env` — reads each KEY=VALUE, stores in the vault, replaces .env with zvault:// references
3. `zvault run -- <cmd>` — resolves all zvault:// references, sets real env vars, executes the command

### Phase 2: The AI Mode (Week 3-4) — PAID, this is the money maker

The MCP server + zvault:// reference system:

1. **MCP Server** — `zvault mcp-server` starts an MCP-compatible server
   - Tools: list_secrets, describe_secret, check_env, generate_template, run_with_secrets
   - LLMs can query what secrets exist without seeing values
   - LLMs can trigger `zvault run` to test code with real secrets

2. **zvault:// URI scheme** — replaces actual values in all config files
   - `.env` files: `STRIPE_KEY=zvault://payments/stripe-live`
   - `docker-compose.yml`: environment variables reference zvault
   - Any config file: zvault resolves at runtime

3. **llms.txt** — `zvault llms-txt` generates a file that tells AI tools how to work with the project
   ```
   # llms.txt
   This project uses ZVault for secrets management.
   
   ## Rules
   - Never hardcode secret values in code or config files
   - Use zvault:// references for all secrets
   - To run the project: zvault run -- npm run dev
   - To add a new secret: zvault set <path> <value>
   
   ## Available Secrets
   - zvault://payments/stripe-live (Stripe API key, last rotated 2026-01-15)
   - zvault://database/postgres-prod (PostgreSQL connection string)
   - zvault://aws/main-access-key (AWS access key for S3)
   - zvault://auth/jwt-secret (JWT signing secret)
   
   ## Environment Template
   STRIPE_KEY=zvault://payments/stripe-live
   DATABASE_URL=zvault://database/postgres-prod
   AWS_ACCESS_KEY_ID=zvault://aws/main-access-key
   JWT_SECRET=zvault://auth/jwt-secret
   ```

4. **IDE Integration Setup Commands**
   ```bash
   # Cursor
   zvault setup cursor
   # → Creates .cursor/mcp.json with zvault MCP server config
   # → Creates .cursorrules addition about zvault usage
   
   # Kiro
   zvault setup kiro
   # → Creates .kiro/settings/mcp.json with zvault MCP server config
   # → Creates .kiro/steering/zvault.md with usage instructions
   
   # VS Code + Continue
   zvault setup continue
   # → Creates .continue/config.json with zvault MCP server
   
   # Generic
   zvault setup generic
   # → Creates llms.txt + .env.zvault
   ```

### Phase 3: Team Features (Week 5-6) — TEAM tier

1. **Shared vault** — team members connect to a shared ZVault server
2. **RBAC** — devs see dev secrets, ops sees prod secrets
3. **Audit trail** — who accessed what, when, from which IDE
4. **Secret rotation** — auto-rotate and notify team
5. **Slack/Discord webhooks** — alerts on secret access, rotation, policy violations

### Phase 4: Enterprise (Week 8+) — ENTERPRISE tier

1. HA clustering (Raft)
2. K8s operator
3. Namespaces (dev/staging/prod isolation)
4. OIDC SSO
5. Compliance reports

---

## Marketing Strategy

### Positioning

**"Your .env file is a liability. ZVault makes it an asset."**

Alternative taglines:
- "Stop leaking secrets to LLMs"
- "The secrets manager that speaks AI"
- "Encrypted secrets. AI-aware config. One binary."
- "Let Cursor read your config, not your keys"

### Launch Strategy (Realistic for Solo Dev)

#### Week 1: Build in Public

- Tweet/post daily progress building ZVault AI Mode
- "Day 1: I'm building an MCP server that lets Cursor access secrets without seeing them"
- Show the terminal recordings, the before/after
- Tag @cursor_ai, @kaborhq, @anthropic — they want this ecosystem

#### Week 2: Show HN Launch

Post: **"Show HN: ZVault — Stop leaking API keys to your AI coding assistant"**

The HN post should demo:
1. `zvault import .env` (the "oh shit" moment)
2. Cursor reading zvault:// references (LLM sees names, not values)
3. `zvault run -- npm run dev` (everything works, nothing leaked)
4. The audit log showing exactly what was accessed

HN loves: Rust, security, single binary, open source, anti-big-tech-data-collection angle.

#### Week 3: Reddit + Dev Communities

- r/rust: "I built a secrets manager in Rust that protects your keys from AI coding tools"
- r/devops: "How we stopped leaking secrets to Cursor/Copilot"
- r/selfhosted: "Self-hosted secrets manager, single binary, zero dependencies"
- r/cursor: "MCP server that gives Cursor access to your secrets without exposing values"
- Dev.to: Tutorial — "How to use Cursor safely with production API keys"

#### Week 4: Content Marketing

Blog posts (on zvault.cloud/blog):
1. "The hidden cost of AI-assisted coding: your secrets" (SEO: "cursor api key leak")
2. "How to use .env files safely with LLMs" (SEO: "env file security cursor copilot")
3. "Building an MCP server in Rust" (developer audience, shows expertise)
4. "ZVault vs HashiCorp Vault vs Infisical: honest comparison" (SEO: comparison keywords)

#### Week 5+: Ongoing

- YouTube: 3-minute demo video
- Product Hunt launch
- Homebrew formula: `brew install zvault`
- Docker Hub: official image
- Railway template: one-click deploy
- npm package: `npx zvault init` (thin wrapper that downloads the binary)

### SEO Keywords to Target

High intent, low competition:
- "cursor ai api key security"
- "copilot secrets leak"
- "llm env file security"
- "mcp server secrets manager"
- "self hosted secrets manager"
- "vault alternative rust"
- "env file encryption"
- "stop leaking api keys ai"

### Distribution Channels

| Channel | Effort | Expected Impact |
|---------|--------|-----------------|
| Hacker News | 1 day | 500-2000 stars, 50-100 signups |
| r/rust + r/devops + r/selfhosted | 1 day | 200-500 stars |
| r/cursor + r/ChatGPTCoding | 1 day | Direct Pro conversions |
| Dev.to / Hashnode articles | 2 days | Long-tail SEO traffic |
| Twitter/X build-in-public | Ongoing | Community building |
| YouTube demo | 1 day | Evergreen discovery |
| Product Hunt | 1 day | 100-300 signups |
| Homebrew + npm | 1 day | Frictionless install |

---

## Technical Implementation Priority

### What to Build First (MVP for Revenue)

1. **`zvault import .env`** — The hook. Import existing .env, encrypt, replace with references.
2. **`zvault run -- <cmd>`** — The runtime. Resolve references, inject env vars, run command.
3. **`zvault mcp-server`** — The money maker. MCP server for Cursor/Kiro/Continue.
4. **`zvault setup <ide>`** — The onboarding. Auto-configure IDE MCP settings.
5. **`zvault llms-txt`** — The SEO play. Generate llms.txt for AI tool discovery.
6. **License verification** — Ed25519 signed license keys for Pro/Team/Enterprise.
7. **Landing page** — zvault.cloud with install command, demo video, pricing.

### What NOT to Build Yet

- HA clustering (enterprise, later)
- K8s operator (enterprise, later)
- Real database credential execution (nice to have, not the money maker)
- OIDC (team tier, later)
- SDK libraries (after you have users)

---

## Revenue Projections

### Month 1 (Launch)
- 500 GitHub stars
- 200 installs
- 10 Pro signups = $80/mo

### Month 2 (Content + Word of Mouth)
- 1500 GitHub stars
- 800 installs
- 25 Pro + 3 Team = $200 + $57 = $257/mo ← **TARGET HIT**

### Month 3 (Momentum)
- 3000 GitHub stars
- 2000 installs
- 40 Pro + 8 Team + 1 Enterprise = $320 + $152 + $49 = $521/mo

### Month 6
- 5000+ stars
- 5000+ installs
- 80 Pro + 20 Team + 5 Enterprise = $640 + $380 + $245 = $1,265/mo

---

## Payment Infrastructure

Use **Lemon Squeezy** or **Polar.sh** (built for developer tools):
- Handles global payments, tax, invoicing
- License key generation built-in
- Webhook to your server for activation
- 5% fee (vs Stripe's 2.9% + complexity of building billing yourself)

Flow:
1. User buys Pro on zvault.cloud → Lemon Squeezy processes payment
2. Lemon Squeezy generates license key → webhook to your API
3. User runs `zvault activate <license-key>`
4. ZVault verifies license locally (Ed25519 signature check, no phone-home needed)
5. AI Mode features unlock

---

## Competitive Landscape

| Product | AI Mode? | Self-Hosted? | Price | Complexity |
|---------|----------|-------------|-------|------------|
| **ZVault** | ✅ MCP + zvault:// | ✅ Single binary | $8-49/dev/mo | One command |
| HashiCorp Vault | ❌ | ✅ (complex) | $0.03/secret/mo | Days to set up |
| Infisical | ❌ | ⚠️ (Docker + Postgres) | $8/dev/mo | Medium |
| Doppler | ❌ | ❌ (SaaS only) | $18/dev/mo | Easy but SaaS |
| 1Password | ❌ | ❌ (SaaS only) | $8/user/mo | Not for devs |
| dotenv-vault | ❌ | ❌ (SaaS only) | Free-$4/mo | Easy but limited |

**ZVault's unique angle: the ONLY secrets manager with native AI/LLM integration.**

Nobody else is doing the MCP server + zvault:// reference + llms.txt combo. This is blue ocean.

---

## Summary: The Path to $200/mo

1. **Week 1-2**: Build `zvault import`, `zvault run`, polish the free tier
2. **Week 3-4**: Build MCP server + `zvault setup cursor/kiro` + license system
3. **Week 5**: Launch on HN, Reddit, Twitter. Landing page + pricing live.
4. **Week 6-8**: Content marketing, iterate based on feedback
5. **Month 2**: Hit $200/mo target with ~25 Pro users

The key insight: **the free tier gets people in, the AI Mode makes them pay.** Every developer using Cursor or Kiro will immediately understand why they need this.

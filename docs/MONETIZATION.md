# ZVault — Monetization & Go-to-Market Plan

> The AI-native secrets manager — dev to prod, one tool. Replaces AWS Secrets Manager, Doppler, and scattered .env files.
> Updated: 2026-02-20 (Cloud Platform Pivot)

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
   - AWS Secrets Manager: $0.40/secret/month + API call costs, no AI awareness, deep vendor lock-in
   - 1Password: not designed for programmatic access
   - `.env` files: the root cause of the problem

5. **The deployment gap is real:**
   - Devs manage secrets in .env locally, then copy-paste into Vercel/Railway/AWS dashboards
   - No single source of truth — secrets scattered across 5 platforms
   - Rotating a key means updating it in 5 places manually
   - AWS SM costs $0.40/secret/month — a team with 50 secrets pays $20/mo just for storage

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
| **Pro** | $12/mo per developer | Freelancers, indie hackers | AI Mode (MCP server), cloud vault, 3 envs, 50K API req/mo, 5 projects, priority email |
| **Team** | $27/mo per developer | Startups (5-50 eng) | Everything in Pro + 5 envs, 500K API req/mo, OIDC SSO, audit log export, unlimited projects, Slack alerts |
| **Business** | $89/mo per developer | Scale-ups (50-200 eng) | Everything in Team + 15 envs, 5M API req/mo, audit log streaming, secret rotation, dynamic credentials, Terraform provider, 99.5% SLA |
| **Enterprise** | $529/mo per developer | Mid-market (200+ eng) | Everything in Business + unlimited envs/API, HA clustering, K8s operator, SCIM, dedicated infra, custom SLA, dedicated support |

### Revenue Math to $200/mo

You need just:
- 17 Pro users ($12 × 17 = $204), OR
- 8 Team users ($27 × 8 = $216), OR
- 3 Business users ($89 × 3 = $267), OR
- 1 Enterprise user ($529 × 1 = $529), OR
- Mix: 8 Pro + 2 Team + 1 Business = $96 + $54 + $89 = $239

This is very achievable with the right positioning.

### What's Free vs Paid

| Feature | Free | Pro ($12/dev/mo) | Team ($27/dev/mo) | Business ($89/dev/mo) | Enterprise ($529/dev/mo) |
|---------|------|------------------|-------------------|-----------------------|--------------------------|
| Local encrypted vault | ✅ | ✅ | ✅ | ✅ | ✅ |
| CLI (init, import, run) | ✅ | ✅ | ✅ | ✅ | ✅ |
| .env import/export | ✅ | ✅ | ✅ | ✅ | ✅ |
| Local web dashboard | ✅ | ✅ | ✅ | ✅ | ✅ |
| KV + Transit + PKI engines | ✅ | ✅ | ✅ | ✅ | ✅ |
| **Cloud vault (zvault.cloud)** | — | ✅ | ✅ | ✅ | ✅ |
| **Environments per project** | — | 3 | 5 | 15 | Unlimited |
| **API requests/mo** | — | 50K | 500K | 5M | Unlimited |
| **Cloud dashboard** | — | ✅ | ✅ | ✅ | ✅ |
| **Service tokens (CI/CD + prod)** | — | ✅ | ✅ | ✅ | ✅ |
| **SDKs (13 languages)** | — | ✅ | ✅ | ✅ | ✅ |
| **AI Mode (MCP server)** | — | ✅ | ✅ | ✅ | ✅ |
| **zvault:// references** | — | ✅ | ✅ | ✅ | ✅ |
| **IDE setup (Cursor, Kiro, Continue)** | — | ✅ | ✅ | ✅ | ✅ |
| Projects | 1 local | 5 | Unlimited | Unlimited | Unlimited |
| Team members | — | 1 | Unlimited | Unlimited | Unlimited |
| RBAC + env-level permissions | — | — | ✅ | ✅ | ✅ |
| SSO (OIDC/SAML) | — | — | ✅ | ✅ | ✅ |
| Audit log export | — | — | ✅ | ✅ | ✅ |
| Slack/Discord alerts | — | — | ✅ | ✅ | ✅ |
| Audit log streaming | — | — | — | ✅ | ✅ |
| Secret rotation | — | — | — | ✅ | ✅ |
| Dynamic credentials | — | — | — | ✅ | ✅ |
| Terraform provider | — | — | — | ✅ | ✅ |
| SLA guarantee | — | — | — | 99.5% | 99.9% |
| Dedicated infrastructure | — | — | — | — | ✅ |
| SCIM provisioning | — | — | — | — | ✅ |
| Custom SLA | — | — | — | — | ✅ |
| Priority support | — | Email | Email | Priority email | Dedicated |

---

## Implementation Plan (Cloud-First)

### Phase 1: Local Hook (SHIPPED ✅)

The frictionless local experience is already live:

```bash
curl -fsSL https://zvault.cloud/install.sh | sh
zvault init && zvault import .env
zvault run -- npm run dev
```

### Phase 2: AI Mode + MCP (SHIPPED ✅)

MCP server, zvault:// references, IDE setup, llms.txt — all shipped in v0.2.0.

### Phase 3: ZVault Cloud (Week 1-4) — THE PLATFORM

This is the big pivot. ZVault becomes a full secrets platform that replaces AWS Secrets Manager.

1. **Cloud Backend** — PostgreSQL multi-tenant API (orgs → projects → environments → secrets)
2. **Cloud Dashboard** — app.zvault.cloud (auth, project view, env tabs, secret editor, team management)
3. **CLI Cloud Mode** — `zvault login`, `zvault cloud push`, `zvault run --env prod`
4. **Service Tokens** — scoped to project + environment, for CI/CD and production runtime
5. **SDKs** — Node.js, Go, Python thin HTTP clients for runtime secret fetching
6. **CI/CD Integrations** — GitHub Actions, Docker entrypoint, Railway/Fly.io/Vercel docs

### Phase 4: Production Hardening (Week 5-8)

1. Multi-region deployment + 99.9% SLA
2. SOC 2 Type I preparation
3. Secret rotation with auto-propagation
4. Import from AWS SM / Doppler migration tools
5. Environment promotion (`promote staging → prod`)

### Phase 5: Team & Enterprise (Week 9+)

1. RBAC with environment-level permissions
2. OIDC / SAML SSO, SCIM provisioning
3. Slack/Discord notifications
4. K8s operator, Terraform provider
5. Dedicated infrastructure option

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

## Revenue Projections (Cloud Tier Math)

The cloud platform changes the revenue math significantly — teams pay per seat, and the value prop (replace AWS SM + AI protection) justifies the price easily.

### AWS SM Cost Comparison (Selling Point)

A team with 50 secrets across 3 environments:
- **AWS SM**: 150 secrets × $0.40 = $60/mo + API call costs (~$10/mo) = **$70/mo**
- **ZVault Pro**: $12/dev/mo × 5 devs = **$60/mo** (unlimited secrets)
- **ZVault Team**: $27/dev/mo × 5 devs = **$135/mo** (unlimited secrets + RBAC + SSO)

For larger teams (20 devs, 200 secrets):
- **AWS SM**: 600 secrets × $0.40 = $240/mo + API calls = **~$280/mo**
- **ZVault Team**: $27 × 20 = **$540/mo** (but includes AI Mode, RBAC, SSO — AWS SM doesn't)
- **ZVault Business**: $89 × 20 = **$1,780/mo** (adds rotation, dynamic creds, Terraform, 99.5% SLA)

### Month 1 (Launch)
- 500 GitHub stars
- 200 installs
- 15 Pro + 3 Team = $180 + $81 = **$261/mo**

### Month 2 (Content + Word of Mouth)
- 1500 GitHub stars
- 800 installs
- 30 Pro + 8 Team = $360 + $216 = **$576/mo** ← exceeds $200 target

### Month 3 (Momentum)
- 3000 GitHub stars
- 2000 installs
- 50 Pro + 15 Team + 1 Business + 1 Enterprise = $600 + $405 + $89 + $529 = **$1,623/mo**

### Month 6
- 5000+ stars
- 5000+ installs
- 100 Pro + 30 Team + 5 Business + 3 Enterprise = $1,200 + $810 + $445 + $1,587 = **$4,042/mo**

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

| Product | AI Mode? | Cloud Platform? | Self-Hosted? | Price | Complexity |
|---------|----------|----------------|-------------|-------|------------|
| **ZVault** | ✅ MCP + zvault:// | ✅ zvault.cloud | ✅ Single binary | $12-529/dev/mo | One command |
| AWS Secrets Manager | ❌ | ✅ (AWS only) | ❌ | $0.40/secret/mo + API calls | Deep AWS lock-in |
| HashiCorp Vault | ❌ | ✅ (HCP) | ✅ (complex) | $0.03/secret/mo | Days to set up |
| Infisical | ❌ | ✅ | ⚠️ (Docker + Postgres) | $8/dev/mo | Medium |
| Doppler | ❌ | ✅ (SaaS only) | ❌ | $18/dev/mo | Easy but SaaS |
| 1Password | ❌ | ✅ (SaaS only) | ❌ | $8/user/mo | Not for devs |
| dotenv-vault | ❌ | ❌ | ❌ | Free-$4/mo | Easy but limited |

**ZVault's unique angles:**
1. **Only secrets manager with native AI/LLM integration** (MCP server + zvault:// references + llms.txt)
2. **Full replacement for AWS SM** at a fraction of the cost ($12 flat vs $0.40/secret/mo)
3. **Dev to prod in one tool** — local vault for free, cloud vault for paid, same CLI

---

## Summary: The Path to $200/mo

1. **Week 1-2**: Build `zvault import`, `zvault run`, polish the free tier
2. **Week 3-4**: Build MCP server + `zvault setup cursor/kiro` + license system
3. **Week 5**: Launch on HN, Reddit, Twitter. Landing page + pricing live.
4. **Week 6-8**: Content marketing, iterate based on feedback
5. **Month 2**: Hit $200/mo target with ~25 Pro users

The key insight: **the free tier gets people in, the AI Mode makes them pay.** Every developer using Cursor or Kiro will immediately understand why they need this.

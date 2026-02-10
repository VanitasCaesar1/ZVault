# ZVault — Production Roadmap

> Last updated: 2026-02-11
> See also: [MONETIZATION.md](./MONETIZATION.md) for full go-to-market plan
> See also: [AUDIT.md](./AUDIT.md) for security audit (all 9 findings fixed)

---

## Vision

**The AI-native secrets manager.** Let LLMs build your app without leaking your keys.

ZVault sits between your IDE's AI assistant and your secrets. The LLM sees `zvault://stripe-key` — never `sk_live_51J3...`. Full audit trail. Single binary. Zero dependencies.

---

## Current State (What's Done)

| Component | Status | Notes |
|-----------|--------|-------|
| Encryption Barrier | ✅ 100% | AES-256-GCM, key hierarchy, zeroization |
| Shamir Seal/Unseal | ✅ 100% | N shares, T threshold, full lifecycle |
| KV v2 Engine | ✅ 95% | CRUD, versioning, metadata |
| Transit Engine | ✅ 90% | Encrypt/decrypt/rewrap/sign/verify, key rotation |
| Token Auth | ✅ 90% | Create/lookup/renew/revoke, TTL, SHA-256 hashing |
| Policy Engine | ✅ 85% | CRUD, path-based RBAC, glob matching |
| Audit Logging | ✅ 90% | File backend, HMAC'd fields, fail-closed |
| Lease Manager | ✅ 85% | Create/lookup/renew/revoke, expiry scan |
| Mount Manager | ✅ 80% | Mount/unmount/list/resolve |
| Database Engine | ✅ 75% | Config/role CRUD, credential generation |
| PKI Engine | ✅ 80% | Root CA, cert issuance, role-based |
| AppRole Auth | ✅ 85% | Role CRUD, secret ID, login flow |
| Storage Backends | ✅ 100% | RocksDB, redb, memory |
| Security Hardening | ✅ 100% | mlock, core dump prevention, constant-time |
| Web Dashboard | ✅ 95% | Login, init, unseal, secrets (CRUD modal), policies (CRUD modal), audit (real data), bento dashboard, sparkline |
| CLI Client | ✅ 100% | Status, init, unseal, token, kv, transit, import, run, mcp-server, setup, activate, license, doctor, project-init, lease, audit-export, notify, rotate |
| Deployment | ✅ 90% | Railway, Docker, binary |

---

## Phase 0: Dashboard UI/UX Overhaul (Now)

Goal: Match the Crextio-style premium feel — desaturated cream background, airy glass cards, neutral shadows, amber used sparingly as accent only.

### 0.1 Design Token Refresh ✅

- [x] Desaturate background gradient (sandy cream, not golden amber)
- [x] Increase card opacity (`rgba(255,255,255,0.72)` — cards read as near-white)
- [x] Replace white glow borders with subtle dark borders (`rgba(0,0,0,0.06)`)
- [x] Neutralize shadows across all components (drop warm amber tint)
- [x] Use amber only for accents (active nav, buttons, badges) — not text/borders

### 0.2 Component Updates ✅

- [x] StatCard: neutral shadow (`rgba(0,0,0,.06)`), stone text colors
- [x] Table/Card: neutral borders (`border-stone-200/60`), `border-glass-border`
- [x] Topbar: lighter glass, neutral text (`text-stone-800`, `text-stone-600`)
- [x] Sidebar: softened section labels (`text-stone-500`)
- [x] Login page: neutral glass, stone text, `bg-stone-800` Spring button
- [x] Init/Unseal pages: neutral card style, stone labels/hints
- [x] Secrets/Policies/Audit pages: neutral code blocks, hover states
- [x] AuthMethods/Leases pages: neutral styling
- [x] Badges: neutral `primary` variant (`bg-stone-100 text-stone-600`)

### 0.3 Dashboard Content ✅

- [x] Wire stat cards to real data (secret count, policy count, mount count)
- [x] Wire Secrets page to real `/v1/secret/list/` endpoint
- [x] Wire Policies page to real `/v1/sys/policies` endpoint
- [x] Wire Auth Methods page to real mount data
- [x] Wire Leases page to real lookup/renew/revoke endpoints
- [x] Wire Mounted Engines card to real `/v1/sys/mounts` data
- [x] Add bento-style mixed card sizes (not uniform grid)
- [x] Add simple activity sparkline or bar chart
- [x] Dark contrast card for recent activity (like Crextio's onboarding card)

---

## Phase 1: The Free Hook (Week 1-2)

Goal: Frictionless onboarding that makes devs go "oh, this is nice."

### 1.1 `zvault import .env` — The Magic Moment ✅

```bash
zvault import .env
# ✓ Imported 12 secrets from .env
# ✓ Created .env.zvault (safe for git, has zvault:// references)
# ✓ Backed up original to .env.backup
# ✓ Added .env to .gitignore
```

- [x] Parse .env files (KEY=VALUE, comments, multiline, quoted, export prefix)
- [x] Store each secret in local vault under `env/<project>/<key>`
- [x] Generate `.env.zvault` with `KEY=zvault://<project>/<key>` references
- [x] Backup original .env
- [x] Auto-add .env to .gitignore if not already there

### 1.2 `zvault run -- <command>` — Secret Injection ✅

```bash
zvault run -- npm run dev
# Resolves all zvault:// references → injects real env vars → runs command
```

- [x] Read .env.zvault (or .env with zvault:// values)
- [x] Resolve each zvault:// URI against local vault
- [x] Set as environment variables
- [x] Exec the child process with injected env
- [x] Auto-detect .env.zvault → .env fallback

### 1.3 `zvault project-init` — Project Setup ✅

```bash
zvault project-init
# ✓ Created .zvault.toml for project "my-app"
```

- [ ] Auto-start local vault daemon if not running (post-launch)
- [x] Create project namespace via `.zvault.toml` config
- [x] Generate `.zvault.toml` project config file

### 1.4 Dashboard Polish

- [x] Wire Auth Methods page to real mount data
- [x] Wire Leases page to real lease lookup/renew/revoke
- [x] Wire Secrets page to correct list endpoint
- [x] Wire Audit page to real `GET /v1/sys/audit-log` endpoint (needs backend)
- [x] Add secret create/edit modal
- [x] Add policy editor with JSON validation

### 1.5 Lease Expiry Worker ✅

- [x] Wire `find_expired()` → `revoke()` loop in main.rs (background worker with configurable interval)
- [x] Add `GET /v1/sys/leases` list endpoint (auth-gated, requires `sys/leases` read)
- [ ] Engine-specific revocation callbacks (post-launch — currently revokes lease storage only)

---

## Phase 2: AI Mode — The Money Maker (Week 3-4)

Goal: MCP server + IDE integration that makes the Pro tier irresistible.

### 2.1 MCP Server ✅

```bash
zvault mcp-server
# Starts MCP-compatible server for AI coding tools
```

Tools exposed:
- [x] `zvault_list_secrets` — List secret names/paths (never values)
- [x] `zvault_describe_secret` — Metadata: version, created_at, key names (never values)
- [x] `zvault_check_env` — Verify all required secrets exist for a project
- [x] `zvault_generate_env_template` — Generate .env.zvault from vault contents
- [x] `zvault_set_secret` — Store a new secret
- [x] `zvault_delete_secret` — Delete a secret
- [x] `zvault_vault_status` — Check vault health (sealed/unsealed/initialized)

### 2.2 IDE Setup Commands ✅

```bash
zvault setup cursor    # → .cursor/mcp.json + .cursor/rules/zvault.mdc
zvault setup kiro      # → .kiro/settings/mcp.json + .kiro/steering/zvault.md
zvault setup continue  # → .continue/config.json
zvault setup generic   # → llms.txt
```

- [x] Detect existing IDE config, merge (don't overwrite)
- [x] Generate IDE-specific steering/rules files
- [x] Cursor: MCP config + .mdc rules file
- [x] Kiro: MCP config (with auto-approve for read-only tools) + steering file
- [x] Continue: MCP config
- [x] Generic: llms.txt with project secret inventory

### 2.3 llms.txt Generation ✅

```bash
zvault setup generic
# → Generates llms.txt (also available via `zvault setup generic`)
```

- [x] List all secret paths with descriptions (no values)
- [x] Include usage instructions for AI tools
- [x] Include zvault:// reference format docs
- [x] Include `zvault run` instructions

### 2.4 License System ✅

- [x] Ed25519-signed license keys (verify locally, no phone-home)
- [x] `zvault activate <license-key>` command
- [x] `zvault license` status command
- [x] License tiers: Free, Pro, Team, Enterprise
- [x] Feature gating: MCP server, `zvault setup` require Pro+
- [x] `GET /v1/sys/license` endpoint for dashboard
- [x] Lemon Squeezy / Polar.sh webhook integration

### 2.5 Landing Page (zvault.cloud) ✅

- [x] Hero: "Stop leaking secrets to LLMs"
- [x] Demo video/GIF: import → run → MCP in action
- [x] Pricing table
- [x] Install command front and center
- [x] Comparison table vs Vault/Infisical/Doppler
- [x] Blog section

---

## Phase 3: Team Features (Week 5-6)

### 3.1 Shared Vault (Post-Launch)

- [ ] Team members connect to shared ZVault server
- [ ] Project-scoped access (dev sees dev secrets, not prod)
- [ ] Invite flow: `zvault team invite user@email.com`

### 3.2 Secret Rotation ✅

- [x] Rotation policies per secret path (`zvault rotate set-policy/get-policy/list-policies/remove-policy`)
- [x] Manual trigger with status tracking (`zvault rotate trigger/status`)
- [x] Webhook notifications on rotation (integrates with notify subsystem)

### 3.3 Audit Log Export ✅

- [x] Export audit logs as JSON/CSV (`zvault audit-export --format json|csv`)
- [x] Filter by limit (`--limit N`)
- [x] Output to file or stdout (`--output <path>`)
- [x] Dashboard: audit log viewer with search (wired to `GET /v1/sys/audit-log`)

### 3.4 Notifications ✅

- [x] Slack/Discord/generic webhook support (`zvault notify set-webhook <url>`)
- [x] Event filtering (secret.accessed, secret.rotated, lease.expired, policy.violated)
- [x] Test notification command (`zvault notify test`)
- [ ] Email digest (daily/weekly) — post-launch

---

## Phase 4: Enterprise (Week 8+)

- [x] OIDC authentication (Spring OAuth2 + PKCE, auto-mint vault tokens with role-based policies)
- [ ] Raft HA clustering
- [ ] K8s operator (VaultSecret CRD)
- [ ] Namespaces (dev/staging/prod)
- [x] Prometheus metrics endpoint (`/v1/sys/metrics` — seal status, lease counts, mount counts, build info)
- [x] Backup/restore (`GET /v1/sys/backup`, `POST /v1/sys/restore` + CLI `zvault backup`/`zvault restore`)
- [ ] Real database credential execution (PostgreSQL, MySQL)
- [ ] Compliance reports

---

## Launch Timeline

```
Week 1-2:  Phase 1 — Free tier (import, run, init, dashboard polish)
Week 3-4:  Phase 2 — AI Mode (MCP server, IDE setup, license system, landing page)
Week 5:    LAUNCH — HN, Reddit, Twitter, Product Hunt
Week 5-6:  Phase 3 — Team features (shared vault, rotation, notifications)
Week 8+:   Phase 4 — Enterprise features

Target: $200/mo by end of Month 2 (~25 Pro users)
```

---

## Immediate Next Steps (This Week)

1. ~~Build `zvault import .env` CLI command~~ ✅
2. ~~Build `zvault run -- <cmd>` secret injection~~ ✅
3. ~~Wire dashboard pages to real API endpoints~~ ✅
4. ~~Complete lease expiry worker~~ ✅ (already in server main.rs)
5. ~~Build MCP server (Phase 2.1)~~ ✅
6. ~~Build `zvault project-init` project setup command~~ ✅
7. ~~Build IDE setup commands (`zvault setup cursor/kiro/continue`)~~ ✅
8. ~~Build `zvault llms-txt` generation~~ ✅
9. ~~Build license system (Ed25519-signed keys)~~ ✅
10. ~~Landing page (zvault.cloud)~~ ✅
11. ~~Polar.sh webhook integration~~ ✅
12. ~~Dashboard bento layout + sparkline~~ ✅
13. ~~Secret create/edit modal + policy editor~~ ✅
14. ~~Audit page wired to real endpoint~~ ✅
15. ~~Lease CLI (list/lookup/revoke)~~ ✅
16. ~~Audit export (JSON/CSV)~~ ✅
17. ~~Webhook notifications (Slack/Discord)~~ ✅
18. ~~Secret rotation policies~~ ✅

### Remaining (Post-Launch)
- Shared vault (team connect, project-scoped access, invite flow) — Phase 3.1
- Auto-start local vault daemon — Phase 1.3
- Engine-specific lease revocation callbacks — Phase 1.5
- Email digest notifications — Phase 3.4
- Phase 4 enterprise features

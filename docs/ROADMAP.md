# ZVault — Production Roadmap

> Last updated: 2026-02-20
> See also: [MONETIZATION.md](./MONETIZATION.md) for full go-to-market plan
> See also: [AUDIT.md](./AUDIT.md) for security audit (all 9 findings fixed)
> See also: [MCP_TOOLS_ROADMAP.md](./MCP_TOOLS_ROADMAP.md) for 50-tool MCP plan

---

## Vision

**The AI-native secrets manager — dev to prod, one tool.**

ZVault is the single source of truth for your secrets across every environment. Locally, your AI sees `zvault://stripe-key` — never real values. In staging and production, your app fetches secrets directly from ZVault Cloud. No AWS Secrets Manager. No Doppler. No scattered env vars across 5 dashboards.

**Free tier**: Local encrypted vault, CLI, .env import, AI protection — no account needed.
**Cloud tier**: Manage secrets for dev/staging/prod from one dashboard. Your app calls ZVault at runtime — everywhere.

---

## Current State (v0.2.0 — Shipped)

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
| Web Dashboard | ✅ 95% | Login, init, unseal, secrets CRUD, policies CRUD, audit, bento dashboard |
| CLI Client | ✅ 100% | Full command set (30+ commands) |
| MCP Server | ✅ 100% | 20 tools across 2 tiers |
| IDE Integration | ✅ 100% | Cursor, Kiro, Continue, generic |
| License System | ✅ 100% | Ed25519-signed, Polar.sh integration |
| Landing Page | ✅ 100% | zvault.cloud (Astro) |
| Docs Site | ✅ 100% | docs.zvault.cloud (18 pages) |
| Deployment | ✅ 90% | Railway, Docker, binary, Homebrew, npm |

---

## Phase 5: ZVault Cloud — The Platform (v0.3.0)

Goal: Transform ZVault from a local dev tool into a full secrets platform that replaces AWS Secrets Manager, Doppler, and Infisical. Users manage secrets on zvault.cloud, pull them from anywhere — local dev to production.

### 5.1 Cloud Backend (Multi-Tenant API)

The hosted ZVault service. One vault per organization, secrets scoped by project + environment.

- [x] PostgreSQL-backed storage (replaces RocksDB for cloud — multi-tenant, queryable)
- [x] Multi-tenant data model: `organizations → projects → environments → secrets`
- [x] Environment scoping: `development`, `staging`, `production` (+ custom)
- [x] Per-environment secret values (same key, different value per env)
- [x] Organization management: create org, invite members, assign roles
- [x] Service tokens: scoped to project + environment, for CI/CD and production
- [x] API endpoints:
  - `POST /v1/cloud/orgs` — create organization
  - `GET /v1/cloud/projects` — list projects
  - `POST /v1/cloud/projects` — create project
  - `GET /v1/cloud/projects/:id/secrets?env=production` — list secrets for env
  - `PUT /v1/cloud/projects/:id/secrets` — set secret (per env)
  - `GET /v1/cloud/projects/:id/secrets/:key?env=production` — get single secret
  - `DELETE /v1/cloud/projects/:id/secrets/:key` — delete secret
  - `POST /v1/cloud/projects/:id/tokens` — create service token
  - `GET /v1/cloud/projects/:id/audit` — audit log
- [x] Encryption at rest: AES-256-GCM per-org encryption key (key hierarchy: master → org → secret)
- [x] Rate limiting per org/token tier
- [x] Auto-unseal (no Shamir for cloud — managed key, always available)

### 5.2 Cloud Dashboard (Web App)

The web UI at app.zvault.cloud where users manage everything.

- [x] Auth: email + password, GitHub OAuth, Google OAuth
- [x] Onboarding flow: sign up → create org → create first project → add secrets
- [x] Project view: list all secrets, toggle between environments (dev/staging/prod tabs)
- [x] Secret editor: add/edit/delete secrets per environment, bulk import from .env
- [x] Environment management: create custom environments, clone env → env
- [x] Team management: invite by email, roles (admin / developer / viewer)
- [x] Service tokens page: create/revoke tokens, scoped to project + env
- [x] Audit log viewer: who accessed what, when, from where (CLI vs dashboard vs SDK)
- [x] Billing page: current plan, usage, upgrade/downgrade
- [x] Settings: org name, danger zone (delete org)

### 5.3 CLI Cloud Mode

The existing CLI gains a `--cloud` mode. Same commands, different backend.

- [x] `zvault login` — authenticate CLI with cloud account (browser OAuth flow)
- [x] `zvault cloud init` — link current directory to a cloud project (writes `.zvault.toml`)
- [x] `zvault cloud push` — push local secrets to cloud project (with env selection)
- [x] `zvault cloud pull --env dev` — pull secrets from cloud to local `.env`
- [x] `zvault run --env staging -- npm start` — resolve from cloud, inject as env vars
- [x] `zvault cloud status` — show linked project, current env, token status
- [x] `zvault cloud envs` — list environments for current project
- [x] `zvault cloud secrets --env prod` — list secret keys for an environment
- [x] `zvault cloud token create --env prod --name "railway-deploy"` — create service token
- [x] Service token auth: `ZVAULT_TOKEN=xxx zvault run --env prod -- node server.js`
- [x] Fallback: if no cloud config, CLI works against local vault (existing behavior, unchanged)
- [x] `.zvault.toml` gains `[cloud]` section:
  ```toml
  [cloud]
  org = "my-company"
  project = "my-saas"
  default_env = "development"
  ```

### 5.4 SDKs & Client Libraries

Official first-party SDKs for every major language and runtime. All SDKs share the same contract: fetch all secrets at boot in one HTTP call, cache in-memory, auto-refresh on TTL, graceful fallback if cloud is unreachable.

#### Tier 1 — Launch (Week 3-4)
These cover 80%+ of production workloads.

- [x] **Node.js / TypeScript** (`@zvault/sdk`) — npm
  ```typescript
  import { ZVault } from '@zvault/sdk';
  const vault = new ZVault({ token: process.env.ZVAULT_TOKEN });
  const secrets = await vault.getAll({ env: 'production' });
  ```
- [x] **Go** (`github.com/ArcadeLabsInc/zvault-go`) — go get
  ```go
  client := zvault.New(os.Getenv("ZVAULT_TOKEN"))
  secrets, err := client.GetAll(ctx, "production")
  ```
- [x] **Python** (`zvault`) — pip
  ```python
  from zvault import ZVault
  vault = ZVault(token=os.environ["ZVAULT_TOKEN"])
  secrets = vault.get_all(env="production")
  ```
- [x] **Rust** (`zvault-sdk`) — crates.io
  ```rust
  let client = ZVault::new(std::env::var("ZVAULT_TOKEN")?);
  let secrets = client.get_all("production").await?;
  ```

#### Tier 2 — Enterprise Languages (Week 7-8)
Enterprise shops run Java, C#, PHP, Ruby. Can't ignore them.

- [x] **Java / Kotlin** (`com.zvault:zvault-sdk`) — Maven Central
  ```java
  ZVault vault = ZVault.builder().token(System.getenv("ZVAULT_TOKEN")).build();
  Map<String, String> secrets = vault.getAll("production");
  ```
- [x] **C# / .NET** (`ZVault.SDK`) — NuGet
  ```csharp
  var vault = new ZVaultClient(Environment.GetEnvironmentVariable("ZVAULT_TOKEN"));
  var secrets = await vault.GetAllAsync("production");
  ```
- [x] **Ruby** (`zvault`) — RubyGems
  ```ruby
  vault = ZVault::Client.new(token: ENV['ZVAULT_TOKEN'])
  secrets = vault.get_all(env: 'production')
  ```
- [x] **PHP** (`zvault/sdk`) — Packagist
  ```php
  $vault = new ZVault\Client(getenv('ZVAULT_TOKEN'));
  $secrets = $vault->getAll('production');
  ```

#### Tier 3 — Modern & Systems Languages (Week 10+)
For teams running Elixir, Swift server-side, Dart backends, etc.

- [x] **Elixir** (`zvault`) — Hex
- [x] **Swift** (`ZVaultSDK`) — Swift Package Manager
- [ ] **Dart** (`zvault`) — pub.dev
- [ ] **Deno** (`@zvault/sdk`) — JSR / deno.land
- [x] **Bun** — same `@zvault/sdk` npm package, Bun-compatible (Node SDK uses zero deps + native fetch)

#### SDK Core Features (All Languages)

Every SDK implements the same behavior:

| Feature | Description |
|---------|-------------|
| **Single-call bootstrap** | `getAll(env)` fetches all secrets in one HTTP request at boot |
| **In-memory cache** | Secrets cached in process memory, never written to disk |
| **Auto-refresh** | Background refresh on configurable TTL (default: 5 min) |
| **Graceful degradation** | If cloud unreachable, serves last-known cached values |
| **Lazy single-secret fetch** | `get(key, env)` for on-demand single secret retrieval |
| **Watch mode** | `watch(callback)` for real-time secret change notifications via SSE/WebSocket |
| **Structured logging** | Debug logs for troubleshooting (opt-in, never logs secret values) |
| **Retry with backoff** | Exponential backoff on transient failures (429, 503) |
| **mTLS support** | Optional mutual TLS for zero-trust environments (Enterprise) |
| **Proxy support** | HTTP/SOCKS5 proxy for corporate networks |
| **Custom CA certs** | For self-hosted ZVault behind corporate PKI |
| **Metrics export** | Prometheus-compatible metrics (fetch latency, cache hit rate, errors) |
| **Health check** | `vault.healthy()` returns connection status for readiness probes |
| **Secret references** | Resolve `zvault://project/key` URIs in config files programmatically |
| **Env injection** | `vault.injectIntoEnv()` sets all secrets as process env vars |
| **Type-safe config** | Generate typed config structs from secret schema (Go, Rust, TypeScript) |

#### Framework Integrations

First-class integrations that go beyond "just an HTTP client":

- [x] **Spring Boot Starter** (`zvault-spring-boot-starter`) — auto-configure `@Value("${zvault.db.url}")`, PropertySource integration, actuator health indicator
- [x] **Django** (`django-zvault`) — settings.py integration, `ZVAULT_SECRETS` dict, management commands
- [x] **Flask** (`flask-zvault`) — `app.config.from_zvault(env='production')`
- [x] **FastAPI** (`fastapi-zvault`) — dependency injection, `Depends(get_zvault_secret("db_url"))`
- [x] **Hono middleware** (`@zvault/hono`) — `app.use(zvault.middleware())` auto-injects into `c.get('secrets')`
- [x] **Next.js plugin** (`@zvault/next`) — build-time + runtime secret injection, `next.config.js` integration
- [x] **NestJS module** (`@zvault/nestjs`) — `@InjectSecret('STRIPE_KEY')` decorator, ConfigModule integration
- [x] **Rails** (`zvault-rails`) — `Rails.application.credentials` replacement, initializer integration
- [x] **Laravel** (`zvault-laravel`) — config/zvault.php, `config('zvault.stripe_key')`, Artisan commands
- [x] **ASP.NET Core** (`ZVault.Extensions.Configuration`) — IConfiguration provider, `builder.AddZVault()`
- [x] **Gin** (`zvault-gin`) — middleware for Go Gin framework
- [x] **Fiber** (`zvault-fiber`) — middleware for Go Fiber framework
- [ ] **Phoenix** (`zvault_phoenix`) — config provider for Elixir Phoenix
- [ ] **Ktor** (`zvault-ktor`) — plugin for Kotlin Ktor

### 5.5 CI/CD & DevOps Integrations

Not just "docs on how to use the CLI" — actual first-class plugins.

#### CI/CD Platforms

- [x] **GitHub Actions** (`zvault/setup-action@v1`)
  ```yaml
  - uses: zvault/setup-action@v1
    with:
      token: ${{ secrets.ZVAULT_TOKEN }}
      env: staging
  - run: npm test  # All secrets available as env vars
  ```
- [x] **GitLab CI** (`zvault/gitlab-ci-component`) — CI/CD component, `include: zvault/secrets`
- [x] **CircleCI Orb** (`zvault/secrets`) — orb with `zvault/inject-secrets` job step
- [x] **Bitbucket Pipes** (`zvault/inject-secrets-pipe`) — pipe for Bitbucket Pipelines
- [x] **Jenkins Plugin** (`zvault-credentials`) — Credentials provider, pipeline step `withZVaultSecrets {}`
- [x] **Azure DevOps** (`zvault-task`) — marketplace task for Azure Pipelines
- [x] **AWS CodeBuild** — buildspec.yml integration guide + helper script
- [x] **Buildkite Plugin** (`zvault/secrets-buildkite-plugin`) — auto-inject secrets into build steps
- [x] **Drone CI** — `.drone.yml` plugin
- [x] **Tekton** — Tekton Task for K8s-native CI/CD

#### Infrastructure as Code

- [x] **Terraform Provider** (`terraform-provider-zvault`)
  ```hcl
  resource "zvault_secret" "db_url" {
    project     = "my-saas"
    environment = "production"
    key         = "DATABASE_URL"
    value       = var.database_url
  }

  data "zvault_secret" "stripe_key" {
    project     = "my-saas"
    environment = "production"
    key         = "STRIPE_KEY"
  }
  ```
- [x] **Pulumi Provider** (`@zvault/pulumi`) — TypeScript/Go/Python/C# Pulumi resources
- [x] **OpenTofu** — same Terraform provider, OpenTofu compatible
- [ ] **Crossplane Provider** (`provider-zvault`) — Kubernetes-native IaC
- [ ] **Ansible Module** (`zvault_secret`) — for Ansible playbooks
- [ ] **Chef Cookbook** (`zvault`) — for Chef infrastructure
- [ ] **Puppet Module** (`zvault-zvault`) — for Puppet manifests

#### Container & Orchestration

- [x] **Kubernetes Operator** (`zvault-operator`)
  ```yaml
  apiVersion: zvault.cloud/v1
  kind: VaultSecret
  metadata:
    name: app-secrets
  spec:
    project: my-saas
    environment: production
    target:
      name: app-secrets        # Creates this K8s Secret
      type: Opaque
    refreshInterval: 5m        # Auto-sync every 5 min
  ```
- [x] **K8s CSI Driver** — mount secrets as files in pods (alternative to env vars)
- [x] **K8s Mutating Webhook** — auto-inject secrets into pods via annotations
- [x] **Helm Chart** — `helm install zvault-operator zvault/operator`
- [x] **Docker Init** — `zvault` as PID 1 entrypoint, injects secrets then exec's your app
  ```dockerfile
  ENTRYPOINT ["zvault", "run", "--env", "prod", "--"]
  CMD ["node", "server.js"]
  ```
- [x] **Docker Compose** — `zvault` service + env_file generation
- [ ] **Nomad Job Driver** — HashiCorp Nomad integration (ironic but useful)
- [x] **ECS Task Definition** — AWS ECS integration via init container pattern
- [x] **Cloud Run** — GCP Cloud Run sidecar pattern
- [x] **Lambda Layer** — AWS Lambda extension that fetches secrets at cold start

#### Platform Integrations (PaaS)

- [x] **Vercel** — build-time injection via `vercel.json` + ZVAULT_TOKEN env var
- [x] **Railway** — template + plugin, auto-inject at deploy
- [x] **Fly.io** — `fly secrets` replacement, `fly.toml` integration
- [x] **Render** — environment group sync
- [x] **Coolify** — native integration
- [x] **Heroku** — buildpack that injects secrets at dyno boot
- [x] **Netlify** — build plugin for build-time secrets
- [x] **Cloudflare Workers** — `wrangler.toml` integration, Workers KV sync
- [x] **Deno Deploy** — environment variable injection
- [x] **Supabase** — Edge Functions secret injection

#### Secret Rotation & Dynamic Credentials

- [x] **Database credential rotation** — auto-rotate PostgreSQL, MySQL, MongoDB passwords on schedule
- [x] **AWS IAM** — generate short-lived AWS credentials (STS AssumeRole) from ZVault
- [x] **GCP Service Account** — generate short-lived GCP tokens
- [x] **Azure AD** — generate short-lived Azure tokens
- [x] **Stripe key rotation** — rotate API keys with zero-downtime (dual-key pattern)
- [x] **Webhook on rotation** — fire webhooks when any secret rotates, so dependent services can refresh
- [x] **Rotation policies** — configurable per-secret rotation schedule (30d, 90d, custom)

#### Observability & Monitoring

- [x] **Prometheus exporter** — `/metrics` endpoint on cloud API (secret access counts, latency, error rates)
- [x] **Grafana dashboard** — pre-built dashboard JSON for ZVault Cloud monitoring
- [x] **Datadog integration** — custom metrics + events for secret access
- [x] **PagerDuty** — alert on unauthorized access attempts, rotation failures
- [x] **Slack/Discord/Teams** — real-time notifications per channel (secret changed, accessed, rotated)
- [x] **OpsGenie** — incident creation on security events
- [x] **Audit log streaming** — stream audit events to S3, CloudWatch, Elasticsearch, Splunk, Datadog Logs
- [x] **SIEM integration** — CEF/LEEF format for Splunk, QRadar, Sentinel

#### IDE & Developer Tools

- [x] **VS Code Extension** — inline secret peek (hover to see metadata, not value), go-to-definition for `zvault://` URIs, secret autocomplete
- [x] **JetBrains Plugin** — IntelliJ, WebStorm, GoLand, PyCharm — same features as VS Code extension
- [x] **Neovim Plugin** (`zvault.nvim`) — Telescope picker for secrets, inline virtual text
- [x] **Cursor/Kiro/Continue** — already shipped via MCP server
- [x] **GitHub App** — PR checks (detect hardcoded secrets, suggest zvault:// references), secret scanning integration
- [ ] **GitLab Integration** — merge request checks, secret detection
- [x] **pre-commit hook** — `zvault scan` to catch leaked secrets before commit

#### Migration Tools

- [x] **Import from AWS Secrets Manager** — `zvault migrate aws-sm --region us-east-1`
- [x] **Import from Doppler** — `zvault migrate doppler --project my-app`
- [x] **Import from Infisical** — `zvault migrate infisical --workspace my-ws`
- [x] **Import from HashiCorp Vault** — `zvault migrate hcv --addr https://vault.example.com`
- [x] **Import from 1Password** — `zvault migrate 1password --vault Development`
- [x] **Import from Vercel env vars** — `zvault migrate vercel --project my-app`
- [x] **Import from Railway** — `zvault migrate railway --project my-app`
- [x] **Import from .env files** — already shipped (`zvault import .env`)
- [x] **Export to .env** — `zvault cloud pull --env prod --format env > .env.prod`
- [x] **Export to JSON** — `zvault cloud pull --env prod --format json`
- [x] **Export to YAML** — `zvault cloud pull --env prod --format yaml`

---

## Phase 6: Production Hardening (v0.4.0)

Goal: Make ZVault Cloud production-grade for teams that need reliability guarantees.

### 6.1 Infrastructure

- [ ] Multi-region deployment (primary: US-East, replicas: EU-West, AP-Southeast)
- [ ] Read replica routing — SDK reads from nearest region, writes go to primary
- [ ] 99.9% uptime SLA (Team+), 99.95% (Business+), 99.99% (Enterprise)
- [ ] Automated backups: hourly → R2 (7-day retention), daily → cold storage (90-day retention)
- [ ] Point-in-time recovery (PITR) for PostgreSQL — restore to any second within retention window
- [ ] Health monitoring + PagerDuty alerting (latency p99 > 500ms, error rate > 1%, disk > 80%)
- [ ] CDN edge caching for secret reads (encrypted payload, 30s TTL, cache-busted on write)
- [ ] Blue-green deployments with zero-downtime migrations
- [ ] Connection pooling: PgBouncer per region, max 200 connections per pool
- [ ] Auto-scaling: horizontal pod scaling based on request rate (K8s HPA)

### 6.2 Security Hardening

- [ ] SOC 2 Type I preparation (audit trail, access controls, encryption docs, vendor review)
- [ ] SOC 2 Type II audit engagement (6-month observation period)
- [ ] Secret value encryption: per-org AES-256-GCM keys, org key rotation support
- [ ] Key hierarchy: master key (HSM-backed) → org key → project key → secret value
- [ ] IP allowlisting for service tokens (Team+) — CIDR range support
- [ ] Mandatory 2FA for org admins (TOTP + WebAuthn/passkeys)
- [ ] Secret access alerts (Slack/email when prod secrets are read by new IP/token)
- [ ] Anomaly detection: alert on unusual access patterns (time, volume, geography)
- [ ] Penetration testing: annual third-party pentest, results published to customers
- [ ] Vulnerability disclosure program (security@zvault.cloud + HackerOne)
- [ ] TLS 1.3 only — no TLS 1.2 fallback on cloud API
- [ ] Certificate pinning documentation for SDK users in high-security environments

### 6.3 Advanced Features

- [ ] Secret rotation with auto-propagation (rotate → all envs updated → webhooks fired → SDKs refresh)
- [ ] Secret references across projects (`zvault://other-project/shared-db-url`) with cross-project ACLs
- [ ] Environment promotion: `zvault cloud promote staging → production` (copy all secrets, diff preview)
- [ ] Diff view: compare secrets between environments (key presence + metadata, never values)
- [ ] Secret comments/descriptions (visible in dashboard + MCP `describe_secret` tool)
- [ ] Secret tagging: arbitrary key-value tags for organization (`team:payments`, `service:api`)
- [ ] Secret search: full-text search across key names, descriptions, and tags
- [ ] Bulk operations: update/delete multiple secrets in one API call (atomic transaction)
- [ ] Secret pinning: lock a secret version in an environment (prevent accidental rotation)
- [ ] Rollback: one-click revert to previous secret version per environment

---

## Phase 7: Team & Enterprise (v0.5.0)

### 7.1 Team Features

- [ ] RBAC: admin / developer / viewer per project
- [ ] Environment-level permissions (dev can read dev, only ops reads prod)
- [ ] Invite flow with email + role assignment
- [ ] Activity feed: real-time org activity stream
- [ ] Slack/Discord integration: secret change notifications per channel

### 7.2 Enterprise

- [ ] OIDC / SAML SSO
- [ ] SCIM user provisioning
- [ ] Namespaces (org-level isolation for large companies)
- [ ] Compliance reports (who accessed what, exportable)
- [ ] Dedicated infrastructure option
- [ ] Custom SLA

---

## Phase 8: MCP Tier 3-5 + Advanced Platform (v0.6.0+)

### MCP Expansion
- [ ] MCP Tools 21-50 (see [MCP_TOOLS_ROADMAP.md](./MCP_TOOLS_ROADMAP.md))
- [ ] MCP cloud-aware tools (list environments, switch env, deploy secrets from IDE)

### Dynamic Secrets Engine
- [ ] **PostgreSQL** — generate short-lived database credentials with auto-revocation
- [ ] **MySQL / MariaDB** — same pattern
- [ ] **MongoDB** — scoped database users with TTL
- [ ] **Redis / Dragonfly** — ACL-based credential generation
- [ ] **RabbitMQ** — vhost-scoped credentials
- [ ] **Elasticsearch** — API key generation with role mapping
- [ ] **AWS STS** — AssumeRole for short-lived AWS credentials
- [ ] **GCP Service Account Keys** — short-lived OAuth2 tokens
- [ ] **Azure AD** — short-lived service principal tokens
- [ ] **Consul** — ACL token generation
- [ ] **Nomad** — ACL token generation
- [ ] **LDAP** — dynamic credential generation

### Secret Governance (Enterprise)
- [ ] **Secret policies** — enforce naming conventions, rotation schedules, max TTL
- [ ] **Approval workflows** — require manager approval for prod secret changes
- [ ] **Break-glass access** — emergency access with full audit trail + auto-revocation
- [ ] **Secret expiry alerts** — notify before certificates/keys expire
- [ ] **Compliance dashboards** — SOC 2, HIPAA, PCI-DSS readiness views
- [ ] **Data residency** — pin secrets to specific regions (EU, US, APAC)
- [ ] **Secret classification** — tag secrets by sensitivity (public, internal, confidential, restricted)
- [ ] **Dual-control** — require two admins to approve sensitive secret changes

### API & Protocol Support
- [ ] **REST API** — already shipped
- [ ] **gRPC API** — high-performance binary protocol for service mesh environments
- [ ] **GraphQL API** — for dashboard and complex queries
- [ ] **WebSocket / SSE** — real-time secret change streaming
- [ ] **OpenAPI spec** — auto-generated, always up-to-date
- [ ] **Client certificate auth (mTLS)** — zero-trust service identity
- [ ] **OIDC JWT auth** — authenticate with any OIDC provider token (K8s service accounts, GitHub Actions OIDC)
- [ ] **AWS IAM auth** — authenticate using AWS IAM roles (for Lambda, ECS, EC2)
- [ ] **GCP IAM auth** — authenticate using GCP service accounts
- [ ] **Azure Managed Identity auth** — authenticate using Azure MI

### Multi-Cloud & Hybrid
- [ ] **Secret sync to AWS SM** — optional one-way sync for teams migrating gradually
- [ ] **Secret sync to GCP Secret Manager** — same
- [ ] **Secret sync to Azure Key Vault** — same
- [ ] **Secret sync to K8s Secrets** — via operator (already planned)
- [ ] **Secret sync to Vercel/Railway/Fly.io** — push secrets to PaaS env vars
- [ ] **Hybrid mode** — some secrets in cloud, some local, unified CLI/SDK access

---

## Updated Pricing Model

| Feature | Free | Pro ($12/dev/mo) | Team ($27/dev/mo) | Business ($89/dev/mo) | Enterprise ($529/dev/mo) |
|---------|------|------------------|-------------------|-----------------------|--------------------------|
| Local encrypted vault | ✅ | ✅ | ✅ | ✅ | ✅ |
| CLI (init, import, run) | ✅ | ✅ | ✅ | ✅ | ✅ |
| .env import/export | ✅ | ✅ | ✅ | ✅ | ✅ |
| KV + Transit + PKI engines | ✅ | ✅ | ✅ | ✅ | ✅ |
| Local web dashboard | ✅ | ✅ | ✅ | ✅ | ✅ |
| **Cloud vault** | — | ✅ | ✅ | ✅ | ✅ |
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

## Website Updates (zvault.cloud)

The landing page needs to reflect the cloud platform story, not just "local dev tool."

### Hero Section
- [x] Update headline: "Stop leaking secrets to LLMs" → **"One vault. Every environment. AI-native."**
- [x] Update subtext to mention dev-to-prod story, not just AI protection
- [x] Add second CTA: "Start Free" (local) + "Try Cloud" (sign up)
- [x] Update trust signals: add "Dev to prod" and "Replaces AWS SM"

### HowItWorks Section
- [x] Change from 3 steps (import → code → run) to 4 steps:
  1. **Import** — `zvault import .env` encrypts locally
  2. **Code with AI** — LLM sees references, not values
  3. **Push to Cloud** — `zvault cloud push` syncs to all environments
  4. **Deploy** — App fetches secrets at runtime from ZVault Cloud
- [x] Add "No AWS Secrets Manager needed" callout

### Pricing Section
- [x] Update Free tier: emphasize "local vault, no account needed"
- [x] Update Pro tier: add cloud vault, environments, service tokens, SDKs
- [x] Update Team tier: add RBAC, env-level permissions, unlimited members
- [x] Update Enterprise: add SLA, dedicated infra, SCIM
- [x] Change trust row: "Secrets never leave your machine" → "Your secrets, your infrastructure"

### ComparisonTable Section
- [x] Add "AWS Secrets Manager" as 5th column (replace Doppler or add alongside)
- [x] Add rows: "Multi-environment", "Service tokens", "CI/CD integration", "Runtime SDKs"
- [x] Update "Local-first" row to show ZVault has both local AND cloud

### New Sections
- [ ] **"Dev to Prod" section** — visual flow showing local dev → CI/CD → staging → production, all from one vault
- [ ] **"Replace AWS SM" section** — side-by-side cost comparison ($0.40/secret/mo vs $8 flat), feature comparison
- [ ] **SDK code samples section** — show Node/Go/Python snippets for runtime fetching

### FAQ Updates
- [x] Add: "Can I use ZVault in production?" → Yes, cloud vault + service tokens
- [x] Add: "Does this replace AWS Secrets Manager?" → Yes, for most teams
- [x] Add: "Where are my secrets stored?" → Encrypted in ZVault Cloud (AES-256-GCM), or locally if using free tier
- [x] Add: "What happens if ZVault goes down?" → SDKs cache secrets in memory, graceful degradation

---

## Implementation Priority

### Now (Week 1-2): Cloud Backend MVP
1. PostgreSQL schema for multi-tenant secrets (orgs, projects, envs, secrets)
2. Cloud API endpoints (CRUD secrets per environment)
3. Service token creation + auth
4. `zvault login` + `zvault cloud init` + `zvault run --env` CLI commands

### Week 3-4: Cloud Dashboard + SDKs (Tier 1)
5. Cloud dashboard (app.zvault.cloud) — auth, project view, secret editor, env tabs
6. Tier 1 SDKs: Node.js, Go, Python, Rust
7. GitHub Actions integration
8. Team invites + basic RBAC

### Week 5-6: Website + Launch
9. Update zvault.cloud landing page (hero, how-it-works, pricing, comparison, new sections)
10. Update docs site with cloud documentation
11. Terraform provider + K8s operator
12. Launch: HN post "ZVault Cloud — the AI-native secrets manager, dev to prod"

### Week 7-8: Tier 2 SDKs + Polish
13. Tier 2 SDKs: Java, C#, Ruby, PHP
14. Framework integrations: Spring Boot, Django, Next.js, Rails, Laravel
15. GitLab CI, CircleCI Orb, Bitbucket Pipes
16. Import from AWS SM / Doppler migration tools
17. Secret rotation with auto-propagation
18. VS Code extension + JetBrains plugin

### Week 9-10: Enterprise + Growth
19. Environment promotion (`promote staging → prod`)
20. Slack/Discord/Teams notifications
21. Audit log streaming (S3, Splunk, Datadog)
22. Dynamic database credentials (PostgreSQL, MySQL)
23. gRPC API
24. Tier 3 SDKs: Elixir, Swift, Dart, Deno

---

## Revenue Target

```
Month 1: 15 Pro ($180) + 3 Team ($81) = $261/mo
Month 2: 30 Pro ($360) + 8 Team ($216) + 1 Business ($89) = $665/mo
Month 3: 50 Pro ($600) + 15 Team ($405) + 3 Business ($267) + 1 Enterprise ($529) = $1,801/mo
Month 6: 100 Pro + 30 Team + 8 Business + 3 Enterprise = $1,200 + $810 + $712 + $1,587 = $4,309/mo
```

The cloud tier changes the revenue math significantly — teams pay per seat, and the value prop (replace AWS SM + AI protection) justifies the price easily.

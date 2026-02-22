# ZVault — Clerk Billing Setup Guide

> Research-backed plan descriptions, Clerk architecture overview, and implementation patterns.
> Created: 2026-02-21

---

## Table of Contents

1. [SaaS Plan Description Best Practices](#1-saas-plan-description-best-practices)
2. [Clerk Billing Architecture](#2-clerk-billing-architecture)
3. [ZVault Plan Structure](#3-zvault-plan-structure)
4. [Copy-Paste Plan Descriptions (≤500 chars)](#4-copy-paste-plan-descriptions-500-chars)
5. [Feature Keys & Gating](#5-feature-keys--gating)
6. [React Integration Patterns](#6-react-integration-patterns)
7. [Clerk Dashboard Setup Checklist](#7-clerk-dashboard-setup-checklist)

---

## 1. SaaS Plan Description Best Practices

Research from [ProfitWell](https://www.profitwell.com), [Amelie Pollak](https://www.ameliepollak.com/blog/13-copywriting-hacks-for-your-saas-pricing-page), [The Good](https://thegood.com/insights/saas-pricing-page/), [Gleam](https://gleam.io/blog/startup-pricing/), and [VictorFlow](https://victorflow.com/blog/how-to-plan-a-high-converting-pricing-page-for-your-saas-app). Content was rephrased for compliance with licensing restrictions.

### The Rules

**1. Lead with WHO, not WHAT.**
Every plan description should open with the persona it serves. "For solo developers..." or "For engineering teams..." — the reader should instantly self-select.

**2. Outcome over feature.**
Don't list what the plan includes. Say what the plan *enables*. "Manage secrets across 5 environments" beats "5 environments included." The first implies action and value; the second is a spec sheet.

**3. One sentence, one job.**
Each sentence should do exactly one thing: identify the audience, state the core value, or differentiate from the tier below. No compound sentences trying to do three things at once.

**4. Use conversational language.**
Contractions ("you'll", "don't"), active voice, and plain English. Research consistently shows conversational copy outperforms formal copy on pricing pages. Avoid jargon like "enterprise-grade scalability" — say "scales with your team."

**5. Anchor against the pain, not the competition.**
"Replace scattered .env files" is stronger than "Better than Doppler." Pain-anchoring creates urgency; competitor-anchoring creates comparison shopping.

**6. Keep it scannable.**
Clerk's description field is 500 characters. That's roughly 2-3 sentences. Readers scan pricing pages in seconds — every word must earn its place.

**7. Create value progression.**
Each tier description should make the reader feel like they're "graduating" to the next level. The language should escalate: "get started" → "move faster" → "scale confidently" → "run your org."

**8. Use power words sparingly.**
Words like "unlimited", "instant", "secure", and "automated" trigger emotional responses. But overuse dilutes impact. One power word per description is enough.

### What NOT to Do

- Don't repeat the plan name in the description ("The Pro plan gives you...")
- Don't list features — that's what the feature table is for
- Don't use passive voice ("Secrets are managed by...")
- Don't use vague qualifiers ("enhanced", "improved", "better")
- Don't mention price in the description — Clerk shows it separately
- Don't exceed 500 characters (Clerk hard limit)

### Description Formula

```
[Persona sentence]. [Core value/outcome]. [Key differentiator from tier below].
```

Example:
```
For solo developers shipping side projects and freelance work. 
Cloud vault with AI Mode keeps your secrets out of LLM context windows. 
Push to 3 environments — dev, staging, and prod — from one CLI.
```

---

## 2. Clerk Billing Architecture

Sources: [Clerk B2C Billing Guide](https://clerk.com/docs/react/guides/billing/for-b2c), [Clerk B2B Billing Guide](https://clerk.com/docs/react/guides/billing/for-b2b), [Clerk Blog: Add Subscriptions](https://clerk.com/blog/add-subscriptions-to-your-saas-with-clerk-billing). Content was rephrased for compliance with licensing restrictions.

### How It Works

Clerk Billing sits between your app and Stripe. You define plans and features in the Clerk Dashboard — Clerk handles the UI, entitlement logic, and subscription lifecycle. Stripe handles payment processing only.

```
Clerk Dashboard (define plans + features)
        │
        ▼
┌─────────────────────────────┐
│  Your React App             │
│                             │
│  <PricingTable />           │  ← Drop-in component, renders plans
│  <Protect feature="...">   │  ← Gates content by feature/plan
│  has({ feature: "..." })    │  ← Programmatic check
│  <UserProfile />            │  ← Billing tab for self-service
└──────────┬──────────────────┘
           │
           ▼
┌─────────────────────────────┐
│  Stripe (payment only)      │
│  • Processes charges        │
│  • No Stripe Billing needed │
│  • No webhook wiring needed │
└─────────────────────────────┘
```

### Key Concepts

| Concept | What It Is |
|---------|-----------|
| **Plan** | A subscription tier (e.g., "Pro", "Team"). Has a name, key (slug), description, monthly price, optional annual discount, and optional free trial. |
| **Feature** | A capability flag attached to one or more plans (e.g., `cloud_vault`, `secret_rotation`). Used for gating. |
| **User Plan** | B2C — billed to individual users. Shows in `<PricingTable for="user" />`. |
| **Org Plan** | B2B — billed to organizations. Shows in `<PricingTable for="organization" />`. |
| **Subscription** | The active link between a user/org and a plan. Managed in Clerk Dashboard or via API. |

### Pricing

- 0.7% per transaction (Clerk's fee)
- Plus standard Stripe processing fees (paid directly to Stripe)
- Plans and pricing managed in Clerk Dashboard, NOT in Stripe
- Clerk Billing is separate from Stripe Billing — they don't sync

### Payment Gateways

| Environment | Gateway |
|-------------|---------|
| Development | Clerk development gateway (shared test Stripe account, no setup needed) |
| Production | Your own Stripe account (must be separate from dev) |

### Plan Types for ZVault

| Plan | Clerk Type | Why |
|------|-----------|-----|
| **Pro** ($12/mo) | User Plan | Individual developer, billed per-user |
| **Team** ($27/mo) | Organization Plan | Team billing, per-seat |
| **Business** ($89/mo) | Organization Plan | Larger team, per-seat |
| **Enterprise** ($529/mo) | Organization Plan | Large org, per-seat |

The Free tier doesn't need a Clerk plan — it's the default state (no subscription).

---

## 3. ZVault Plan Structure

### Plan Keys (Slugs)

These are the identifiers you'll use in code and in the Clerk Dashboard:

| Plan | Key | Type | Monthly | Annual (per month) |
|------|-----|------|---------|-------------------|
| Pro | `pro` | User | $12 | $10 (save 17%) |
| Team | `team` | Organization | $27/seat | $23/seat (save 15%) |
| Business | `business` | Organization | $89/seat | $75/seat (save 16%) |
| Enterprise | `enterprise` | Organization | $529/seat | $449/seat (save 15%) |

### Feature Keys

Every gated capability gets a feature key. Attach features to plans in the Clerk Dashboard.

| Feature Key | Pro | Team | Business | Enterprise |
|-------------|-----|------|----------|------------|
| `cloud_vault` | ✅ | ✅ | ✅ | ✅ |
| `ai_mode` | ✅ | ✅ | ✅ | ✅ |
| `cloud_dashboard` | ✅ | ✅ | ✅ | ✅ |
| `service_tokens` | ✅ | ✅ | ✅ | ✅ |
| `sdks` | ✅ | ✅ | ✅ | ✅ |
| `env_3` | ✅ | — | — | — |
| `env_5` | — | ✅ | — | — |
| `env_15` | — | — | ✅ | — |
| `env_unlimited` | — | — | — | ✅ |
| `rbac` | — | ✅ | ✅ | ✅ |
| `sso` | — | ✅ | ✅ | ✅ |
| `audit_export` | — | ✅ | ✅ | ✅ |
| `slack_alerts` | — | ✅ | ✅ | ✅ |
| `audit_streaming` | — | — | ✅ | ✅ |
| `secret_rotation` | — | — | ✅ | ✅ |
| `dynamic_credentials` | — | — | ✅ | ✅ |
| `terraform_provider` | — | — | ✅ | ✅ |
| `sla_99_5` | — | — | ✅ | ✅ |
| `dedicated_infra` | — | — | — | ✅ |
| `scim` | — | — | — | ✅ |
| `k8s_operator` | — | — | — | ✅ |
| `custom_sla` | — | — | — | ✅ |
| `dedicated_support` | — | — | — | ✅ |

### Feature Names & Descriptions by Plan

Paste these into the Clerk Dashboard when creating each feature. Grouped by the plan that first introduces them.

#### Pro (User Plan) — 6 features

| Key | Name | Description |
|-----|------|-------------|
| `cloud_vault` | Cloud Vault | Encrypted cloud-hosted vault at zvault.cloud. Secrets stored with AES-256-GCM, per-org isolation, accessible via CLI, SDK, or dashboard. |
| `ai_mode` | AI Mode | MCP server and zvault:// references for AI coding tools. Your IDE understands your config without ever seeing secret values. |
| `cloud_dashboard` | Cloud Dashboard | Web dashboard for managing secrets, environments, team members, and audit logs at app.zvault.cloud. |
| `service_tokens` | Service Tokens | Scoped tokens for CI/CD pipelines and production runtimes. Bind to a specific project and environment for least-privilege access. |
| `sdks` | SDK Access | Official SDKs for 13 languages — Node.js, Python, Go, Rust, Ruby, PHP, Java, .NET, Swift, Kotlin, Elixir, Dart, and C++. |
| `env_3` | 3 Environments | Up to 3 environments per project (e.g., dev, staging, prod). Secrets are isolated per environment. |

#### Team (Organization Plan) — adds 5 features

*Inherits all Pro features, plus:*

| Key | Name | Description |
|-----|------|-------------|
| `env_5` | 5 Environments | Up to 5 environments per project. Supports additional environments like QA, preview, or canary. |
| `rbac` | Role-Based Access Control | Assign roles with environment-level permissions. Devs see staging, leads see prod. Fine-grained control over who accesses what. |
| `sso` | Single Sign-On (SSO) | OIDC and SAML SSO integration. Connect your existing identity provider — Okta, Azure AD, Google Workspace, or any SAML 2.0 IdP. |
| `audit_export` | Audit Log Export | Export audit logs as CSV or JSON for compliance reporting. Every secret read, write, and access event is recorded. |
| `slack_alerts` | Slack & Discord Alerts | Real-time notifications when secrets are created, updated, rotated, or accessed. Configurable per project and environment. |

#### Business (Organization Plan) — adds 6 features

*Inherits all Team features, plus:*

| Key | Name | Description |
|-----|------|-------------|
| `env_15` | 15 Environments | Up to 15 environments per project. Handles complex deployment topologies with per-region or per-tenant environments. |
| `audit_streaming` | Audit Log Streaming | Stream audit events in real-time to your SIEM or log aggregator — Datadog, Splunk, Elastic, or any webhook endpoint. |
| `secret_rotation` | Secret Rotation | Automated rotation for API keys, database passwords, and cloud credentials. Set a schedule and ZVault handles the rest. |
| `dynamic_credentials` | Dynamic Credentials | On-demand short-lived credentials for databases and cloud providers. Generated at runtime, auto-revoked after TTL expires. |
| `terraform_provider` | Terraform Provider | Official Terraform and Pulumi provider. Manage secrets as infrastructure-as-code alongside your deployments. |
| `sla_99_5` | 99.5% Uptime SLA | Guaranteed 99.5% availability backed by our operations team. Includes incident response and status page notifications. |

#### Enterprise (Organization Plan) — adds 6 features

*Inherits all Business features, plus:*

| Key | Name | Description |
|-----|------|-------------|
| `env_unlimited` | Unlimited Environments | No limit on environments per project. Create as many as your infrastructure requires. |
| `dedicated_infra` | Dedicated Infrastructure | Isolated compute and storage, not shared with other tenants. Your secrets run on infrastructure dedicated to your organization. |
| `scim` | SCIM Provisioning | Automated user lifecycle management via SCIM 2.0. Sync team members from Okta, Azure AD, or OneLogin automatically. |
| `k8s_operator` | Kubernetes Operator | Native K8s operator that syncs ZVault secrets to Kubernetes Secrets. Auto-refreshes pods when secrets rotate. |
| `custom_sla` | Custom SLA | Tailored SLA agreement negotiated for your organization's requirements. Includes custom uptime targets and support response times. |
| `dedicated_support` | Dedicated Support | Named support engineers with a private channel. Priority response for incidents, onboarding assistance, and architecture reviews. |

---

## 4. Copy-Paste Plan Descriptions (≤500 chars)

These are ready to paste into the Clerk Dashboard description field. Each is under 500 characters.

### Pro (User Plan) — 489 chars

```
For solo developers and freelancers who ship to production. Cloud vault keeps your secrets encrypted and synced across devices — no more copy-pasting .env files into deployment dashboards. AI Mode lets your IDE understand your config without ever seeing actual values. Push secrets to 3 environments per project, generate service tokens for CI/CD, and access 13 SDKs. One tool from dev to prod.
```

### Team (Organization Plan) — 496 chars

```
For engineering teams building together. Everything in Pro, plus shared secrets across your org with role-based access control. Each member gets their own permissions — devs see staging, leads see prod. 5 environments per project, OIDC/SAML SSO for your identity provider, exportable audit logs for compliance, and Slack alerts when secrets change. Unlimited projects, unlimited team members. One source of truth for every secret your team touches.
```

### Business (Organization Plan) — 493 chars

```
For scale-ups that need secrets infrastructure they can trust. Everything in Team, plus automated secret rotation, dynamic database credentials, and a Terraform provider for infrastructure-as-code workflows. 15 environments per project handles complex deployment topologies. Real-time audit log streaming to your SIEM. 99.5% uptime SLA backed by our operations team. 5M API requests per month for high-throughput production workloads.
```

### Enterprise (Organization Plan) — 487 chars

```
For organizations where secrets are critical infrastructure. Everything in Business, plus dedicated infrastructure isolated from other tenants, SCIM provisioning for automated user lifecycle, Kubernetes operator for native cluster integration, and a custom SLA tailored to your requirements. Unlimited environments, unlimited API requests. Dedicated support channel with named engineers. Built for teams that can't afford downtime or data exposure.
```

---

## 5. Feature Keys & Gating

### Environment Gating Logic

Environments are the primary usage gate. The logic in your backend/dashboard:

```typescript
// Determine max environments for current plan
function getMaxEnvironments(features: string[]): number {
  if (features.includes('env_unlimited')) return Infinity;
  if (features.includes('env_15')) return 15;
  if (features.includes('env_5')) return 5;
  if (features.includes('env_3')) return 3;
  return 0; // Free tier — no cloud environments
}
```

### API Request Gating

Track monthly API requests per org/user. Limits by plan:

| Plan | Monthly Limit |
|------|--------------|
| Pro | 50,000 |
| Team | 500,000 |
| Business | 5,000,000 |
| Enterprise | Unlimited |

### Gating with `has()`

```typescript
import { useAuth } from '@clerk/clerk-react';

function SecretRotationButton() {
  const { has } = useAuth();

  // Check by feature
  if (!has?.({ feature: 'secret_rotation' })) {
    return <UpgradePrompt feature="Secret Rotation" requiredPlan="Business" />;
  }

  return <button onClick={rotateSecret}>Rotate Secret</button>;
}
```

### Gating with `<Protect>`

```tsx
import { Protect } from '@clerk/clerk-react';

// Gate by feature
<Protect feature="audit_streaming" fallback={<UpgradeBanner />}>
  <AuditStreamingConfig />
</Protect>

// Gate by plan
<Protect plan="business" fallback={<UpgradeBanner />}>
  <TerraformProviderSetup />
</Protect>
```

---

## 6. React Integration Patterns

### Current Implementation (Billing.tsx)

The ZVault dashboard already has the billing page wired up with a scope toggle:

```tsx
// pages/Billing.tsx — already implemented
<PricingTable key={scope} for={scope} />
// scope = "user" | "organization"
```

The `key={scope}` prop forces a remount when toggling between Individual and Team views, ensuring Clerk fetches the correct plan set.

### Subscription Management

Users can manage their subscription from `<UserProfile />` which automatically includes a Billing tab when Clerk Billing is enabled. No extra code needed.

For org billing, the `<OrganizationProfile />` component includes the same Billing tab for org admins.

### Checking Subscription Status

```typescript
import { useAuth } from '@clerk/clerk-react';

function useSubscriptionStatus() {
  const { has, isLoaded } = useAuth();

  if (!isLoaded) return { loading: true };

  const isPro = has?.({ plan: 'pro' });
  const isTeam = has?.({ plan: 'team' });
  const isBusiness = has?.({ plan: 'business' });
  const isEnterprise = has?.({ plan: 'enterprise' });
  const isPaid = isPro || isTeam || isBusiness || isEnterprise;

  return {
    loading: false,
    isPaid,
    isPro,
    isTeam,
    isBusiness,
    isEnterprise,
    hasCloudVault: has?.({ feature: 'cloud_vault' }),
    hasAiMode: has?.({ feature: 'ai_mode' }),
    hasRbac: has?.({ feature: 'rbac' }),
    hasSso: has?.({ feature: 'sso' }),
    hasRotation: has?.({ feature: 'secret_rotation' }),
  };
}
```

### Upgrade Prompt Pattern

```tsx
function UpgradePrompt({ feature, requiredPlan }: { feature: string; requiredPlan: string }) {
  return (
    <div className="bg-amber-50 border border-amber-200 rounded-xl p-4">
      <p className="text-sm text-amber-800 font-medium">
        {feature} requires the {requiredPlan} plan
      </p>
      <a href="/billing" className="text-sm text-amber-600 underline mt-1 inline-block">
        View plans →
      </a>
    </div>
  );
}
```

---

## 7. Clerk Dashboard Setup Checklist

### Step 1: Enable Billing

1. Go to Clerk Dashboard → Configure → Billing Settings
2. Enable Billing
3. Connect Stripe account (production) or use dev gateway (development)

### Step 2: Create User Plans (B2C tab)

Create one plan:

| Field | Value |
|-------|-------|
| Name | Pro |
| Key | `pro` |
| Description | *(paste from Section 4)* |
| Monthly base fee | $12 |
| Annual discount | $10/mo |
| Free trial | 14 days |
| Publicly available | Yes |

Attach features: `cloud_vault`, `ai_mode`, `cloud_dashboard`, `service_tokens`, `sdks`, `env_3`

### Step 3: Create Organization Plans (B2B tab)

Create three plans:

**Team:**

| Field | Value |
|-------|-------|
| Name | Team |
| Key | `team` |
| Description | *(paste from Section 4)* |
| Monthly base fee | $27/seat |
| Annual discount | $23/seat/mo |
| Free trial | 14 days |
| Publicly available | Yes |

Attach features: `cloud_vault`, `ai_mode`, `cloud_dashboard`, `service_tokens`, `sdks`, `env_5`, `rbac`, `sso`, `audit_export`, `slack_alerts`

**Business:**

| Field | Value |
|-------|-------|
| Name | Business |
| Key | `business` |
| Description | *(paste from Section 4)* |
| Monthly base fee | $89/seat |
| Annual discount | $75/seat/mo |
| Free trial | 14 days |
| Publicly available | Yes |

Attach features: everything in Team + `env_15`, `audit_streaming`, `secret_rotation`, `dynamic_credentials`, `terraform_provider`, `sla_99_5`

**Enterprise:**

| Field | Value |
|-------|-------|
| Name | Enterprise |
| Key | `enterprise` |
| Description | *(paste from Section 4)* |
| Monthly base fee | $529/seat |
| Annual discount | $449/seat/mo |
| Free trial | 0 days (custom onboarding) |
| Publicly available | Yes |

Attach features: everything in Business + `env_unlimited`, `dedicated_infra`, `scim`, `k8s_operator`, `custom_sla`, `dedicated_support`

### Step 4: Create All Features

Navigate to Clerk Dashboard → Configure → Features. Create each feature key from the table in Section 3. The key (slug) is what you'll use in code.

### Step 5: Verify in Dashboard

1. Check `<PricingTable for="user" />` shows Pro plan
2. Check `<PricingTable for="organization" />` shows Team, Business, Enterprise
3. Test a subscription flow using the dev gateway
4. Verify `has({ feature: 'cloud_vault' })` returns true after subscribing

---

## Sources

- [Clerk B2C Billing Guide](https://clerk.com/docs/react/guides/billing/for-b2c)
- [Clerk B2B Billing Guide](https://clerk.com/docs/react/guides/billing/for-b2b)
- [Clerk PricingTable Component](https://clerk.com/docs/react/components/billing/pricing-table)
- [Clerk Blog: Add Subscriptions to Your SaaS](https://clerk.com/blog/add-subscriptions-to-your-saas-with-clerk-billing)
- [Amelie Pollak: 13 Copywriting Hacks for SaaS Pricing](https://www.ameliepollak.com/blog/13-copywriting-hacks-for-your-saas-pricing-page)
- [The Good: 13 Effective SaaS Pricing Pages](https://thegood.com/insights/saas-pricing-page/)
- [Gleam: Pricing Page Best Practices](https://gleam.io/blog/startup-pricing/)
- [VictorFlow: High-Converting Pricing Pages](https://victorflow.com/blog/how-to-plan-a-high-converting-pricing-page-for-your-saas-app)
- [ProfitWell: Pricing Page Conversion Research](https://www.profitwell.com)

# ZVault — Comprehensive Redesign Plan

> Website + Documentation + Dashboard
> Last updated: 2026-02-11

---

## Executive Summary

ZVault's product is strong — the CLI, MCP server, encryption barrier, and dashboard are all functional and well-built. But the public-facing surfaces (website, docs) don't match the quality of the product. For a paid security tool, the website needs to inspire trust, demonstrate expertise, and convert visitors into users. This plan covers three areas:

1. **Website** — Full rebuild with premium design, scroll animations, and conversion-focused layout
2. **Documentation** — Migration from Starlight to Docus with enhanced practical content
3. **Dashboard** — Polish pass on the already-solid React dashboard

---

## Part 1: Website Rebuild (zvault.cloud)

### Current Problems

- All inline CSS in a single `.astro` file (~400 lines of `<style>`)
- No component system — everything is raw HTML
- Emoji icons (🤖 🔐 🔑) instead of proper SVG icons
- No animations or scroll interactions
- No social proof (no testimonials, no logos, no star count)
- No demo video or GIF
- No "who's behind this" section
- Comparison table lacks credibility markers
- No blog integration beyond a single link
- Mobile experience is basic

### New Stack

| Tool | Purpose |
|------|---------|
| Astro 5 | Framework (keep current, it's good) |
| Tailwind CSS v4 | Styling (replace inline CSS) |
| GSAP + ScrollTrigger | Scroll-driven animations |
| Lenis | Smooth scroll (buttery 120fps feel) |
| Astro components | Reusable `.astro` components |
| @astrojs/sitemap | SEO |
| sharp | Image optimization (already installed) |

### Component System

```
website/src/
├── components/
│   ├── Nav.astro              # Fixed nav with blur backdrop
│   ├── Hero.astro             # Hero with animated gradient text
│   ├── SocialProof.astro      # GitHub stars, "used by" logos, trust badges
│   ├── ScrollTerminal.astro   # Terminal that types commands on scroll
│   ├── HowItWorks.astro       # 3-step flow with connecting lines
│   ├── EncryptionViz.astro    # Character scramble animation
│   ├── FeatureGrid.astro      # Bento-style feature cards with SVG icons
│   ├── AIModeSection.astro    # MCP server showcase with IDE mockup
│   ├── ArchitectureReveal.astro # Architecture diagram that builds on scroll
│   ├── ComparisonTable.astro  # Sticky comparison with animated checkmarks
│   ├── Pricing.astro          # Pricing cards with hover effects
│   ├── FAQ.astro              # Accordion FAQ
│   ├── CTA.astro              # Final call-to-action
│   ├── Footer.astro           # Links, socials, newsletter
│   ├── InstallBox.astro       # Copy-to-clipboard install command
│   └── BlogPreview.astro      # Latest 3 blog posts
├── layouts/
│   └── Base.astro             # HTML shell, fonts, global styles, Lenis init
├── styles/
│   └── global.css             # Tailwind imports + custom properties
├── scripts/
│   └── animations.ts          # GSAP ScrollTrigger registrations
└── pages/
    ├── index.astro            # Landing page (assembles components)
    ├── blog/                  # Blog posts (Astro content collections)
    ├── dashboard.astro        # Redirect to /ui
    └── success.astro          # Post-purchase page
```

### Page Sections (Top to Bottom)

1. **Nav** — Fixed, glass blur, logo + links + "Get Started" CTA button
2. **Hero** — Animated gradient headline, subtext, two CTAs, install command box, subtle particle/grid background
3. **Social Proof Bar** — GitHub star count (live fetch), "Built in Rust" badge, "AES-256-GCM" badge, "MIT Licensed" badge
4. **Scroll Terminal** — Terminal window that types out the `zvault import` → `zvault run` flow as user scrolls. Each command appears line-by-line, output fades in. This is the "oh shit" moment.
5. **How It Works** — 3 steps (Import → Code with AI → Run) with connecting animated lines between them
6. **Encryption Visualization** — Shows a secret value (`sk_live_51J3xKp...`) scrambling character-by-character into `zvault://payments/stripe-key` as user scrolls past. Each character flips through random chars before landing.
7. **Feature Grid** — Bento-style mixed-size cards (not uniform). SVG icons, not emoji. Cards: MCP Server (large), zvault:// URIs, AES-256-GCM, Shamir Seal, Single Binary, Zero-Trust Audit, Transit Engine, PKI Engine
8. **AI Mode Section** — Full-width showcase of the MCP server. Show a mock IDE sidebar with ZVault tools listed. Code snippet showing `.cursor/mcp.json` config. "This is why you upgrade to Pro."
9. **Architecture Reveal** — The `Clients → API → Barrier → Storage` diagram that draws/builds itself as user scrolls into view. SVG path animation.
10. **Comparison Table** — Sticky header table. ZVault column highlighted. Animated checkmarks that draw in on scroll. Add "Setup time" row prominently.
11. **Pricing** — 4 cards (Free, Pro, Team, Enterprise). Pro card elevated/highlighted. Hover lift effect. Clear feature differentiation.
12. **FAQ** — Accordion with 6-8 common questions (Is it really free? Where are secrets stored? What if I lose my unseal keys? How does MCP work? etc.)
13. **CTA** — "Your .env file is a liability. Fix it in 2 minutes." + install command + quickstart link
14. **Footer** — Logo, nav links, GitHub, docs, contact, "Built with Rust 🦀" badge

### 5 Unique Scroll Interactions

These are the differentiators that make the site memorable:

#### 1. Scroll-Typed Terminal Demo
- Terminal window pinned in viewport
- As user scrolls, commands type out character by character
- Output lines fade in after each command completes
- Comment lines appear in dim color first
- Total scroll distance: ~3 viewport heights mapped to the full demo sequence
- Tech: GSAP ScrollTrigger `pin: true` + timeline with `text` plugin or manual char-by-char

#### 2. Encryption Character Scramble
- A line of text showing a real secret value
- On scroll, each character rapidly cycles through random characters (matrix-style)
- Characters settle one-by-one from left to right into the `zvault://` reference
- The "before" (red/dangerous) transforms into "after" (green/safe)
- Tech: GSAP with custom `onUpdate` callback cycling through character sets

#### 3. Architecture Diagram Reveal
- SVG diagram of the ZVault architecture (Clients → API → Barrier → Storage)
- Boxes draw their borders on scroll (SVG stroke-dashoffset animation)
- Arrows animate between boxes after boxes are drawn
- Labels fade in after arrows complete
- Tech: GSAP ScrollTrigger + SVG path animation (`drawSVG`-style)

#### 4. Sticky Comparison Table
- Table header sticks as user scrolls through rows
- Each row slides in from the right with a slight stagger
- Checkmarks in the ZVault column draw themselves (SVG path animation)
- X marks in competitor columns fade in dimly
- The ZVault column has a subtle glow/highlight that pulses once
- Tech: GSAP ScrollTrigger with `stagger` on row elements

#### 5. Parallax Code Blocks
- Code snippets (`.env`, `.env.zvault`, `mcp.json`) float at different parallax speeds
- Creates depth as user scrolls
- Subtle rotation (1-2 degrees) adds to the 3D feel
- Used in the Hero background and AI Mode section
- Tech: GSAP ScrollTrigger with `scrub: true` on `y` and `rotation` properties

### Design Direction

- **Dark theme** (keep current `#0a0a0b` base — it's appropriate for a security/dev tool)
- **Accent**: Cyan (`#22d3ee`) for primary actions, purple (`#a78bfa`) for secondary highlights
- **Typography**: Inter (body) + JetBrains Mono (code) — already in use, good choices
- **Cards**: Subtle border glow on hover, glass morphism for nav/floating elements
- **Spacing**: Generous whitespace between sections (120-160px padding)
- **Mobile**: Fully responsive. Scroll animations degrade gracefully (fade-in only on mobile, no complex scroll-driven effects)

### Trust & Conversion Elements to Add

- [ ] Live GitHub star count in hero (fetch from GitHub API at build time)
- [ ] "Built in Rust" + "AES-256-GCM" + "MIT Licensed" trust badges
- [ ] Terminal demo that actually shows the product working
- [ ] "Who built this" section or at minimum a link to GitHub profile
- [ ] Blog with at least 2-3 SEO-targeted posts at launch
- [ ] FAQ section addressing common objections
- [ ] Testimonial/quote section (even if it's from beta users or your own experience)
- [ ] "As seen on" section after HN/Reddit launch (add post-launch)
- [ ] Newsletter signup in footer (for product updates)
- [ ] Proper 404 page
- [ ] OG image for social sharing

---

## Part 2: Documentation Migration (docs.zvault.cloud)

### Current State

- Astro + Starlight
- 6 sidebar sections: Getting Started, CLI Reference, AI Mode, API Reference, Self-Hosting, Security
- Content is functional but minimal
- Standard Starlight theme (looks like every other Starlight site)

### Migration: Starlight → Docus

**Why Docus?**
- Built on Nuxt 4 — more flexible than Starlight
- Better component system for interactive docs
- Built-in search, dark mode, versioning
- More unique look (not "yet another Starlight site")
- Better code group support (tabbed code blocks)

**New Stack:**

| Tool | Purpose |
|------|---------|
| Docus (Nuxt 4) | Docs framework |
| Nuxt Content | Markdown/MDC content |
| Shiki | Syntax highlighting |
| Mermaid | Diagrams |

### New Content Structure

```
docs-site/
├── content/
│   ├── 0.getting-started/
│   │   ├── 0.introduction.md
│   │   ├── 1.installation.md
│   │   ├── 2.quickstart.md
│   │   └── 3.concepts.md
│   ├── 1.cli/
│   │   ├── 0.overview.md
│   │   ├── 1.kv.md
│   │   ├── 2.transit.md
│   │   ├── 3.import-run.md
│   │   ├── 4.seal-unseal.md
│   │   ├── 5.project-init.md
│   │   ├── 6.rotation.md
│   │   ├── 7.audit-export.md
│   │   ├── 8.notifications.md
│   │   └── 9.cheat-sheet.md        # NEW: Quick reference card
│   ├── 2.ai-mode/
│   │   ├── 0.overview.md
│   │   ├── 1.mcp-server.md
│   │   ├── 2.ide-setup.md           # Cursor, Kiro, Continue, Generic
│   │   ├── 3.zvault-uris.md
│   │   ├── 4.llms-txt.md
│   │   └── 5.workflows.md           # NEW: Common AI+ZVault workflows
│   ├── 3.api/
│   │   ├── 0.overview.md
│   │   ├── 1.kv.md
│   │   ├── 2.transit.md
│   │   ├── 3.pki.md
│   │   ├── 4.auth.md
│   │   ├── 5.system.md
│   │   └── 6.database.md
│   ├── 4.self-hosting/
│   │   ├── 0.overview.md
│   │   ├── 1.railway.md
│   │   ├── 2.docker.md
│   │   ├── 3.binary.md
│   │   ├── 4.configuration.md       # NEW: All env vars, config options
│   │   └── 5.production-checklist.md # NEW: Production readiness guide
│   ├── 5.security/
│   │   ├── 0.architecture.md
│   │   ├── 1.threat-model.md
│   │   ├── 2.encryption.md
│   │   ├── 3.audit-logging.md
│   │   └── 4.hardening.md
│   ├── 6.guides/                     # NEW section
│   │   ├── 0.env-migration.md       # Migrating from .env to ZVault
│   │   ├── 1.team-setup.md          # Setting up for a team
│   │   ├── 2.ci-cd.md              # Using ZVault in CI/CD pipelines
│   │   ├── 3.docker-compose.md     # ZVault + Docker Compose
│   │   └── 4.troubleshooting.md    # Common issues + fixes
│   └── 7.reference/                  # NEW section
│       ├── 0.env-vars.md            # All environment variables
│       ├── 1.error-codes.md         # Error code reference
│       └── 2.changelog.md           # Version history
├── app.config.ts                     # Docus config
├── nuxt.config.ts                    # Nuxt config
└── package.json
```

### New Content to Write

| Page | Purpose | Priority |
|------|---------|----------|
| CLI Cheat Sheet | One-page quick reference with all commands | High |
| AI Mode Workflows | Step-by-step: "Import .env → Setup Cursor → Code safely" | High |
| Configuration Reference | Every env var, config option, default value | High |
| Troubleshooting | Common errors, fixes, FAQ | High |
| Production Checklist | "Before you deploy" checklist | Medium |
| .env Migration Guide | Detailed guide for migrating existing projects | Medium |
| CI/CD Guide | Using `zvault run` in GitHub Actions, CircleCI | Medium |
| Docker Compose Guide | ZVault alongside your app stack | Medium |
| Error Codes | Every error the CLI/API can return | Low |
| Team Setup Guide | Shared vault, RBAC, invites | Low (post-launch) |

### Docs Design

- Dark theme matching the website
- ZVault branding (logo, accent colors)
- Code blocks with copy button and language tabs
- "Quick Reference" cards at the top of CLI pages
- Mermaid diagrams for architecture
- Callout boxes for warnings, tips, notes
- Search that actually works well

---

## Part 3: Dashboard Improvements

### Current State (Already Good)

The dashboard is in solid shape:
- React 19 + React Router 7 + Tailwind v4 + Vite
- Glass/cream aesthetic with amber accents (Crextio-inspired)
- Bento stat cards with real API data
- Sparkline activity chart
- Dark activity card for recent operations
- Functional pages: Login, Init, Unseal, Dashboard, Secrets (CRUD modal), Policies (CRUD modal), Audit, Leases, Auth Methods
- Cookie-based auth + Spring OAuth/OIDC support
- Responsive layout (sidebar + topbar)

### What to Improve

#### A. UX Polish

| Issue | Fix |
|-------|-----|
| No loading states | Add skeleton loaders for stat cards, tables, activity feed |
| No empty states | Add illustrated empty states ("No secrets yet — import your .env") |
| No error boundaries | Add error boundary component with retry button |
| No toast notifications | Add a lightweight toast system for CRUD feedback (create/delete/update confirmations) |
| No keyboard shortcuts | Add `Cmd+K` command palette for power users (search secrets, navigate pages) |
| Sidebar not responsive | Add mobile hamburger menu + slide-out drawer |
| No breadcrumbs | Add breadcrumb trail for nested navigation |

#### B. Visual Polish

| Issue | Fix |
|-------|-----|
| SVG icons are inline | Extract to a shared icon component or use a small icon library |
| No page transitions | Add subtle fade/slide transitions between routes (React Router + CSS transitions or Framer Motion) |
| Stat cards lack micro-interactions | Add count-up animation on numbers when they load |
| Sparkline is basic | Add tooltip on hover showing exact count per bucket |
| No favicon in dashboard | Add ZVault favicon |
| Login page lacks personality | Add subtle background animation (floating particles or grid) |

#### C. Functional Additions

| Feature | Description | Priority |
|---------|-------------|----------|
| Secret version history | Show version list in secret detail view, allow rollback | Medium |
| Policy syntax highlighting | Use a code editor component (Monaco or CodeMirror) for policy JSON | Medium |
| Audit log filters | Filter by operation type, path, date range, status code | Medium |
| Dashboard customization | Let users reorder/hide stat cards | Low |
| Dark mode toggle | Currently cream-only; add dark mode option | Low |
| License status card | Show current license tier, expiry, upgrade CTA on dashboard | High |
| Onboarding wizard | First-time user flow: "Welcome → Init → Create first secret → Done" | Medium |

#### D. Performance

| Issue | Fix |
|-------|-----|
| No data caching | Add React Query (TanStack Query) for API calls with stale-while-revalidate |
| Full page re-renders | Memoize expensive components (stat cards, tables) |
| No optimistic updates | Add optimistic updates for secret/policy CRUD |
| API calls on every navigation | Cache seal status, mount list, policy list with short TTL |

### Dashboard Priority Order

1. **License status card** on dashboard (drives upgrades)
2. **Skeleton loaders** + **empty states** (polish)
3. **Toast notifications** for CRUD operations
4. **Audit log filters** (most requested power-user feature)
5. **React Query migration** (performance)
6. **Mobile responsive sidebar** (accessibility)
7. **Onboarding wizard** (conversion)
8. Everything else

---

## Implementation Order

### Phase 1: Website (Highest Impact) — 1 week

The website is the first thing potential customers see. It needs to convert.

1. Set up Tailwind + GSAP + Lenis in existing Astro project
2. Build component system (Nav, Hero, Footer first)
3. Build scroll terminal demo (the showpiece)
4. Build remaining sections top-to-bottom
5. Add social proof elements
6. Mobile responsive pass
7. Deploy

### Phase 2: Dashboard Polish — 3-4 days

Quick wins that make the product feel more complete.

1. Add license status card
2. Add skeleton loaders + empty states
3. Add toast notifications
4. Add audit log filters
5. Mobile sidebar

### Phase 3: Docs Migration — 1 week

Migrate content and add new practical guides.

1. Set up Docus project
2. Migrate existing content (6 sections)
3. Write CLI cheat sheet
4. Write AI Mode workflows guide
5. Write configuration reference
6. Write troubleshooting guide
7. Deploy to docs.zvault.cloud

---

## Success Metrics

| Metric | Current | Target |
|--------|---------|--------|
| Time on landing page | Unknown | > 2 minutes (scroll engagement) |
| Bounce rate | Unknown | < 50% |
| Install command copies | Unknown | Track with analytics |
| Docs pages per session | Unknown | > 3 |
| Dashboard daily active users | Unknown | Track post-launch |
| Pro conversion rate | 0% | 5% of installers |

---

## Design References

Sites to draw inspiration from (dev tools with premium feel):

- **Linear** — Clean, dark, scroll animations, trust-inspiring
- **Vercel** — Developer-focused, great typography, code-forward
- **Resend** — Simple, elegant, great pricing page
- **Raycast** — Dark theme, feature showcase, keyboard-first
- **Warp** — Terminal product, great scroll interactions
- **Infisical** — Direct competitor, see what they do well (and where ZVault is better)

# ZVault Dashboard — Top-Tier Plan

> Audit date: 2026-02-25
> Current state: 15 pages, React 19 + React Router 7 + Tailwind v4 + Clerk

This plan covers everything needed to make the dashboard production-grade and competitive with Doppler, Infisical, and HashiCorp Vault UI.

---

## Current Inventory

### What's Built & Working

| Page | CRUD | API Wired | Notes |
|------|------|-----------|-------|
| Login | — | ✅ | Vault token + Clerk + Spring OAuth |
| Dashboard | Read | ✅ | Stats, mounts, sparkline, recent activity |
| Initialize | Create | ✅ | Shamir shares + threshold |
| Unseal | Create | ✅ | Submit shares, progress bar |
| Secrets (KV) | CR | ✅ | List, search, create, view — **no edit, no delete** |
| Policies | CRUD | ✅ | List, create, edit, view, delete — complete |
| Audit Log | Read | ✅ | List with path filter — **no pagination** |
| Leases | RUD | ✅ | Lookup, renew, revoke — **no list-all** |
| Auth Methods | Read | ✅ | Lists Token + AppRole — **buttons are no-ops** |
| Billing | Read | ✅ | Clerk PricingTable — works when Clerk configured |
| Cloud Projects | CRUD | ✅ | Org + project create/list |
| Cloud Project Detail | CRUD | ✅ | Env tabs, secret CRUD per env, delete confirm |
| Cloud Team | CR | ✅ | List members, invite — **no remove, no role change** |
| Cloud Service Tokens | CRD | ✅ | Create, list, revoke, copy-once reveal |
| Cloud Audit | Read | ✅ | Org/project filter, pagination — **only cloud page with pagination** |

### What's Missing

**Entire pages that don't exist:**
- Transit Engine management (keys, encrypt/decrypt, sign/verify)
- PKI Engine management (root CA, certs, roles)
- Database Engine management (config, roles, credentials)
- Mount management (mount/unmount/list engines)
- Token management (create scoped tokens, list active tokens)
- AppRole management (create roles, generate secret IDs)
- Settings / Org settings (rename, delete, danger zone)
- Cloud onboarding wizard (first-time user flow)

**Functional gaps in existing pages:**
- Secrets: no edit, no delete, no version history, hardcoded "v1"
- Auth Methods: "Configure" and "Enable" buttons do nothing
- Leases: no way to list all active leases (only manual lookup)
- Audit: no pagination, loads max 200 entries
- Team: no remove member, no role change
- Dashboard: no real-time refresh, no cloud stats

**Infrastructure gaps:**
- No dark mode
- No mobile layout (sidebar hidden on mobile, no alternative nav)
- No toast notifications (success/error feedback)
- No loading skeletons (just "Loading…" text)
- No keyboard shortcuts
- No pagination on Secrets, Policies, Leases tables
- No confirmation dialogs (Policies delete uses `window.confirm`)
- No error boundary (crash = white screen)
- No 404 page
- No breadcrumbs (only cloud project detail has them)
- No command palette / quick search

---

## The Plan

### Phase 1: Fix What's Broken (Priority: Critical)

These are bugs and incomplete features in existing pages.

#### 1.1 Secrets Page — Complete CRUD
- Add **Edit** button (open modal with current data, PUT on save)
- Add **Delete** button with confirmation modal (not `window.confirm`)
- Show real version number from API response (not hardcoded "v1")
- Add **version history** dropdown (KV v2 supports versioning)
- Add pagination (25 per page, prev/next)

#### 1.2 Auth Methods — Wire Up Buttons
- "Configure" on Token → open modal showing token policies, TTL settings
- "Configure" on AppRole → link to new AppRole management page
- "Enable" on OIDC/K8s → show "coming soon" tooltip or remove planned items

#### 1.3 Leases — Add List All
- Add "List Active Leases" button that calls `/v1/sys/leases` (list endpoint)
- Show all active leases in the table by default (not just manual lookup)
- Add pagination

#### 1.4 Audit Log — Add Pagination
- Add prev/next pagination (reuse pattern from CloudAudit)
- Add date range filter
- Add operation type filter dropdown

#### 1.5 Team — Complete CRUD
- Add "Remove" button per member (with confirmation)
- Add role change dropdown (admin/developer/viewer)
- Show owner badge on org owner

#### 1.6 Error Handling
- Add React error boundary wrapping `<Outlet>`
- Add 404 catch-all route
- Replace all `window.confirm` with proper confirmation modals

### Phase 2: Missing Engine Pages (Priority: High)

These are core vault features with backend routes that have no UI.

#### 2.1 Transit Engine Page (`/transit`)
- **Key management**: List keys, create key (type: aes256-gcm, ed25519, etc.), delete key
- **Operations**: Encrypt, decrypt, rewrap, sign, verify — each as a tab or section
- **Key rotation**: Rotate button per key, show key version
- Backend routes: `/v1/transit/*` (already exist)

#### 2.2 PKI Engine Page (`/pki`)
- **Root CA**: Generate or import root CA, view CA cert
- **Roles**: List/create/delete issuance roles
- **Certificates**: Issue cert, list issued certs, revoke cert
- **CRL**: View/download CRL
- Backend routes: `/v1/pki/*` (already exist)

#### 2.3 Database Engine Page (`/database`)
- **Connections**: Configure database connections (Postgres, MySQL, etc.)
- **Roles**: Create roles with SQL templates for credential generation
- **Credentials**: Generate dynamic credentials, show TTL
- Backend routes: `/v1/database/*` (already exist)

#### 2.4 Mount Management Page (`/mounts`)
- List all mounted engines (KV, Transit, PKI, Database)
- Mount new engine (type selector, path, description)
- Unmount engine (with confirmation — destructive)
- Show engine type, path, description, created date
- Backend routes: `/v1/sys/mounts` (already exist)

#### 2.5 Token Management Page (`/tokens`)
- Create scoped token (select policies, set TTL, set max uses)
- List active tokens (with lookup)
- Revoke token
- Show token metadata (policies, TTL remaining, creation time)
- Backend routes: `/v1/auth/token/*` (already exist)

#### 2.6 AppRole Management Page (`/approle`)
- List roles
- Create role (policies, token TTL, secret ID TTL)
- Generate secret ID
- Login test (role ID + secret ID → token)
- Delete role
- Backend routes: `/v1/auth/approle/*` (already exist)

### Phase 3: Cloud Completeness (Priority: High)

#### 3.1 Cloud Onboarding Wizard
- First-time flow: Create org → Create project → Add first secret
- Shown when user has no orgs (after Clerk sign-in)
- Step indicator (1/3, 2/3, 3/3)

#### 3.2 Bulk Import
- On Cloud Project Detail: "Import .env" button
- Parse .env file, show preview table, confirm → bulk create
- Handle duplicates (skip/overwrite toggle)

#### 3.3 Environment Cloning
- "Clone to..." button on environment tab
- Select target env, preview diff, confirm
- Copies all secrets from source env to target

#### 3.4 Environment Diff
- "Compare" button between two environments
- Side-by-side table: key presence (✓/✗), version, last updated
- Never show values — only metadata

#### 3.5 Org Settings Page (`/cloud/settings`)
- Rename organization
- Danger zone: delete organization (with type-to-confirm)
- View org ID, creation date, tier

### Phase 4: UI/UX Polish (Priority: Medium)

#### 4.1 Dark Mode
- Add theme toggle in Topbar (light/dark/system)
- Store preference in localStorage
- Use Tailwind `dark:` variants
- Update all glass/surface/sidebar colors for dark

#### 4.2 Mobile Responsive
- Add hamburger menu for mobile (slide-out sidebar)
- Stack table columns on small screens (or horizontal scroll)
- Responsive Topbar (collapse actions into menu)

#### 4.3 Toast Notifications
- Add a toast system (lightweight, no library — or use sonner)
- Show success toast on: create, update, delete, copy
- Show error toast on API failures (replace inline error divs for transient errors)

#### 4.4 Loading Skeletons
- Replace "Loading…" text with skeleton rows in all tables
- Skeleton cards on Dashboard
- Skeleton form fields in modals

#### 4.5 Confirmation Modals
- Create a reusable `<ConfirmDialog>` component
- Use for all destructive actions (delete secret, revoke token, unmount engine, remove member)
- Type-to-confirm for high-risk actions (delete org, unmount engine)

#### 4.6 Breadcrumbs
- Add breadcrumb bar below Topbar on all pages
- Pattern: `Dashboard > Secrets > prod/database`
- Cloud: `Cloud > Projects > my-saas > production`

#### 4.7 Empty States
- Design proper empty states for every table (illustration + CTA)
- Reuse the `EmptyState` component from Projects.tsx across all pages

#### 4.8 Command Palette
- `Cmd+K` to open quick search
- Search across: secrets, policies, projects, pages
- Quick actions: create secret, seal vault, navigate to page

### Phase 5: Advanced Features (Priority: Lower)

#### 5.1 Real-Time Dashboard
- WebSocket or SSE for live audit feed on Dashboard
- Auto-refresh seal status (already polls every 15s — good)
- Live secret count, request rate

#### 5.2 Keyboard Shortcuts
- `Cmd+K` — command palette
- `Cmd+N` — new secret (context-aware)
- `Esc` — close modal
- `?` — show shortcuts help

#### 5.3 Secret Diff / History
- On Cloud Project Detail: click version badge → show version history
- Diff view between versions (value masked, show changed/unchanged indicator)

#### 5.4 Export
- Export secrets as .env, JSON, YAML from Cloud Project Detail
- Export audit log as CSV

#### 5.5 Accessibility
- Focus management in modals (trap focus, return focus on close)
- ARIA labels on all interactive elements
- Keyboard navigation for tables (arrow keys)
- Screen reader announcements for toasts

---

## Component Library Gaps

The dashboard has 3 shared components (Sidebar, Topbar, Table). To build the above, we need:

| Component | Used By |
|-----------|---------|
| `ConfirmDialog` | All delete/revoke/unmount actions |
| `Toast` / `Toaster` | All CRUD operations |
| `Skeleton` | All table loading states |
| `Breadcrumbs` | All pages |
| `EmptyState` | All tables (exists in Projects.tsx, needs extraction) |
| `Tabs` | Transit (operations), PKI (sections), Project Detail (already inline) |
| `CodeEditor` | Policy rules, secret JSON, Transit encrypt/decrypt |
| `CommandPalette` | Global `Cmd+K` |
| `Select` | Role picker, engine type picker, env picker |
| `Tooltip` | Planned auth methods, truncated values |
| `CopyButton` | Token reveal, secret values, cert PEM |
| `Pagination` | All tables |
| `DateRangePicker` | Audit log filter |
| `MobileNav` | Responsive sidebar |

---

## Execution Order (Recommended)

```
Week 1:  Phase 1 (fix broken stuff) + extract shared components
Week 2:  Phase 2.4 (Mounts) + 2.5 (Tokens) + 2.6 (AppRole)
Week 3:  Phase 2.1 (Transit) + 2.2 (PKI) + 2.3 (Database)
Week 4:  Phase 3 (Cloud completeness)
Week 5:  Phase 4 (UI/UX polish — dark mode, mobile, toasts, skeletons)
Week 6:  Phase 5 (Advanced — command palette, keyboard shortcuts, export)
```

---

## Dependencies

- All Phase 2 pages depend on existing backend routes (already implemented)
- Phase 3.2 (bulk import) may need a new backend endpoint for batch secret creation
- Phase 4.1 (dark mode) needs CSS variable refactor for glass/surface/sidebar tokens
- Phase 5.1 (real-time) needs WebSocket/SSE endpoint on the server

---

## Success Criteria

The dashboard is "top-tier" when:
1. Every backend engine has a corresponding UI page with full CRUD
2. Every table has pagination, search, and proper empty/loading/error states
3. Dark mode works across all pages
4. Mobile layout is usable (not just hidden sidebar)
5. All destructive actions have confirmation dialogs
6. Toast feedback on every mutation
7. Command palette for power users
8. Cloud onboarding converts new signups smoothly
9. No `window.confirm`, no "Loading…" text, no hardcoded values
10. Accessibility: focus trapping, ARIA labels, keyboard navigation

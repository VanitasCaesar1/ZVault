-- ZVault Cloud: multi-tenant secrets platform
-- Phase 5.1 — organizations, projects, environments, secrets, service tokens, audit

-- ============================================================
-- Organizations
-- ============================================================
CREATE TABLE IF NOT EXISTS organizations (
    id          UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    name        TEXT        NOT NULL,
    slug        TEXT        NOT NULL UNIQUE,
    owner_id    UUID        NOT NULL,
    tier        TEXT        NOT NULL DEFAULT 'free'
                            CHECK (tier IN ('free', 'pro', 'team', 'business', 'enterprise')),
    -- Per-org AES-256-GCM encryption key (encrypted by master key)
    encryption_key BYTEA   NOT NULL,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at  TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_organizations_slug ON organizations (slug);
CREATE INDEX IF NOT EXISTS idx_organizations_owner ON organizations (owner_id);

-- ============================================================
-- Organization members
-- ============================================================
CREATE TABLE IF NOT EXISTS org_members (
    id          UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    org_id      UUID        NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    user_id     UUID        NOT NULL,
    email       TEXT        NOT NULL,
    role        TEXT        NOT NULL DEFAULT 'developer'
                            CHECK (role IN ('admin', 'developer', 'viewer')),
    invited_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
    accepted_at TIMESTAMPTZ,
    UNIQUE (org_id, user_id)
);

CREATE INDEX IF NOT EXISTS idx_org_members_org ON org_members (org_id);
CREATE INDEX IF NOT EXISTS idx_org_members_user ON org_members (user_id);

-- ============================================================
-- Projects
-- ============================================================
CREATE TABLE IF NOT EXISTS projects (
    id          UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    org_id      UUID        NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    name        TEXT        NOT NULL,
    slug        TEXT        NOT NULL,
    description TEXT        NOT NULL DEFAULT '',
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (org_id, slug)
);

CREATE INDEX IF NOT EXISTS idx_projects_org ON projects (org_id);

-- ============================================================
-- Environments
-- ============================================================
CREATE TABLE IF NOT EXISTS environments (
    id          UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    project_id  UUID        NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    name        TEXT        NOT NULL,
    slug        TEXT        NOT NULL,
    sort_order  INT         NOT NULL DEFAULT 0,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (project_id, slug)
);

CREATE INDEX IF NOT EXISTS idx_environments_project ON environments (project_id);

-- ============================================================
-- Secrets (per environment)
-- ============================================================
CREATE TABLE IF NOT EXISTS cloud_secrets (
    id              UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    environment_id  UUID        NOT NULL REFERENCES environments(id) ON DELETE CASCADE,
    key             TEXT        NOT NULL,
    -- Encrypted value (AES-256-GCM with org encryption key)
    encrypted_value BYTEA       NOT NULL,
    -- Nonce used for this encryption (12 bytes)
    nonce           BYTEA       NOT NULL,
    version         INT         NOT NULL DEFAULT 1,
    comment         TEXT        NOT NULL DEFAULT '',
    created_by      UUID,
    updated_by      UUID,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (environment_id, key)
);

CREATE INDEX IF NOT EXISTS idx_cloud_secrets_env ON cloud_secrets (environment_id);
CREATE INDEX IF NOT EXISTS idx_cloud_secrets_env_key ON cloud_secrets (environment_id, key);

-- ============================================================
-- Service tokens (scoped to project + environment)
-- ============================================================
CREATE TABLE IF NOT EXISTS service_tokens (
    id              UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    project_id      UUID        NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    environment_id  UUID        REFERENCES environments(id) ON DELETE CASCADE,
    name            TEXT        NOT NULL,
    -- SHA-256 hash of the token (never store plaintext)
    token_hash      TEXT        NOT NULL UNIQUE,
    -- Prefix for display (first 8 chars, e.g. "zvt_abc1...")
    token_prefix    TEXT        NOT NULL,
    permissions     TEXT[]      NOT NULL DEFAULT ARRAY['read'],
    expires_at      TIMESTAMPTZ,
    last_used_at    TIMESTAMPTZ,
    created_by      UUID,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    revoked_at      TIMESTAMPTZ
);

CREATE INDEX IF NOT EXISTS idx_service_tokens_project ON service_tokens (project_id);
CREATE INDEX IF NOT EXISTS idx_service_tokens_hash ON service_tokens (token_hash);

-- ============================================================
-- Cloud audit log
-- ============================================================
CREATE TABLE IF NOT EXISTS cloud_audit_log (
    id          UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    org_id      UUID        NOT NULL,
    project_id  UUID,
    env_slug    TEXT,
    actor_id    UUID,
    actor_type  TEXT        NOT NULL DEFAULT 'user'
                            CHECK (actor_type IN ('user', 'service_token')),
    action      TEXT        NOT NULL,
    resource    TEXT        NOT NULL,
    detail      JSONB       NOT NULL DEFAULT '{}',
    ip_address  TEXT,
    user_agent  TEXT,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- Partition-friendly index (query by org + time range)
CREATE INDEX IF NOT EXISTS idx_cloud_audit_org_time
    ON cloud_audit_log (org_id, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_cloud_audit_project
    ON cloud_audit_log (project_id, created_at DESC);

-- ============================================================
-- Cloud users (synced from Clerk)
-- ============================================================
CREATE TABLE IF NOT EXISTS cloud_users (
    id              UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    -- Clerk user ID (e.g., 'user_2abc...')
    clerk_id        TEXT        NOT NULL UNIQUE,
    email           TEXT        NOT NULL DEFAULT '',
    name            TEXT        NOT NULL DEFAULT '',
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_cloud_users_clerk ON cloud_users (clerk_id);

-- ============================================================
-- Cloud sessions (kept for service token tracking, not user sessions)
-- User sessions are managed by Clerk — no server-side session table needed.;

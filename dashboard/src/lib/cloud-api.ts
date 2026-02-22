/**
 * Cloud API client — typed functions for all `/v1/cloud/*` endpoints.
 *
 * Uses `cloudFetch` which sends Clerk Bearer token (or falls back to vault token).
 *
 * IMPORTANT: Backend wraps list responses in named objects (e.g. `{ organizations: [...] }`).
 * This client unwraps them so callers get flat arrays.
 */

import { cloudFetch } from "./api";

// ── Types ────────────────────────────────────────────────────────────

export interface Organization {
  id: string;
  name: string;
  slug: string;
  owner_id: string;
  tier: string;
  created_at: string;
  updated_at: string;
}

export interface OrgMember {
  id: string;
  org_id: string;
  user_id: string;
  email: string;
  role: string;
  invited_at: string;
  accepted_at: string | null;
}

export interface Project {
  id: string;
  org_id: string;
  name: string;
  slug: string;
  description: string;
  created_at: string;
  updated_at: string;
}

export interface Environment {
  id: string;
  project_id: string;
  name: string;
  slug: string;
  sort_order: number;
  created_at: string;
}

export interface SecretEntry {
  key: string;
  value: string;
  version: number;
  comment: string;
  created_at: string;
  updated_at: string;
}

export interface SecretKey {
  key: string;
  version: number;
  comment: string;
  updated_at: string;
}

export interface ServiceToken {
  id: string;
  project_id: string;
  environment_id: string | null;
  name: string;
  token_prefix: string;
  permissions: string[];
  expires_at: string | null;
  last_used_at: string | null;
  created_by: string | null;
  created_at: string;
  revoked_at: string | null;
}

export interface AuditEntry {
  id: string;
  org_id: string;
  project_id: string | null;
  env_slug: string | null;
  actor_id: string | null;
  actor_type: string;
  action: string;
  resource: string;
  detail: Record<string, unknown>;
  ip_address: string | null;
  user_agent: string | null;
  created_at: string;
}

export interface UserInfo {
  id: string;
  clerk_id: string;
  email: string;
  name: string;
  created_at: string;
}

// ── Auth ─────────────────────────────────────────────────────────────

export const getMe = () => cloudFetch<UserInfo>("/v1/cloud/me");

// ── Organizations ────────────────────────────────────────────────────

export async function listOrgs(): Promise<Organization[]> {
  const res = await cloudFetch<{ organizations: Organization[] }>("/v1/cloud/orgs");
  return res.organizations;
}

export const createOrg = (name: string, slug: string) =>
  cloudFetch<Organization>("/v1/cloud/orgs", {
    method: "POST",
    body: JSON.stringify({ name, slug }),
  });

export const getOrg = (orgId: string) =>
  cloudFetch<Organization>(`/v1/cloud/orgs/${orgId}`);

// ── Members ──────────────────────────────────────────────────────────

export async function listMembers(orgId: string): Promise<OrgMember[]> {
  const res = await cloudFetch<{ members: OrgMember[] }>(`/v1/cloud/orgs/${orgId}/members`);
  return res.members;
}

export const inviteMember = (orgId: string, email: string, role: string) =>
  cloudFetch<OrgMember>(`/v1/cloud/orgs/${orgId}/members`, {
    method: "POST",
    body: JSON.stringify({ email, role }),
  });

// ── Projects ─────────────────────────────────────────────────────────

export async function listProjects(orgId: string): Promise<Project[]> {
  const res = await cloudFetch<{ projects: Project[] }>(`/v1/cloud/orgs/${orgId}/projects`);
  return res.projects;
}

export const createProject = (
  orgId: string,
  name: string,
  slug: string,
  description: string
) =>
  cloudFetch<Project>(`/v1/cloud/orgs/${orgId}/projects`, {
    method: "POST",
    body: JSON.stringify({ name, slug, description }),
  });

export const getProject = (orgId: string, projectId: string) =>
  cloudFetch<Project>(`/v1/cloud/orgs/${orgId}/projects/${projectId}`);

// ── Environments ─────────────────────────────────────────────────────

export async function listEnvironments(orgId: string, projectId: string): Promise<Environment[]> {
  const res = await cloudFetch<{ environments: Environment[] }>(
    `/v1/cloud/orgs/${orgId}/projects/${projectId}/environments`
  );
  return res.environments;
}

export const createEnvironment = (
  orgId: string,
  projectId: string,
  name: string,
  slug: string
) =>
  cloudFetch<Environment>(
    `/v1/cloud/orgs/${orgId}/projects/${projectId}/environments`,
    {
      method: "POST",
      body: JSON.stringify({ name, slug }),
    }
  );

// ── Secrets ──────────────────────────────────────────────────────────
// Backend routes: /v1/cloud/orgs/{org_id}/projects/{project_id}/envs/{env_slug}/secrets

export async function listSecretKeys(
  orgId: string,
  projectId: string,
  envSlug: string
): Promise<SecretKey[]> {
  const res = await cloudFetch<{ keys: SecretKey[] }>(
    `/v1/cloud/orgs/${orgId}/projects/${projectId}/envs/${envSlug}/secrets`
  );
  return res.keys;
}

export async function getSecret(
  orgId: string,
  projectId: string,
  key: string,
  envSlug: string
): Promise<SecretEntry> {
  const res = await cloudFetch<{ secret: SecretEntry }>(
    `/v1/cloud/orgs/${orgId}/projects/${projectId}/envs/${envSlug}/secrets/${encodeURIComponent(key)}`
  );
  return res.secret;
}

export const setSecret = (
  orgId: string,
  projectId: string,
  envSlug: string,
  key: string,
  value: string,
  comment?: string
) =>
  cloudFetch<{ secret: SecretEntry }>(
    `/v1/cloud/orgs/${orgId}/projects/${projectId}/envs/${envSlug}/secrets/${encodeURIComponent(key)}`,
    {
      method: "PUT",
      body: JSON.stringify({ value, comment: comment ?? "" }),
    }
  );

export const deleteSecret = (
  orgId: string,
  projectId: string,
  key: string,
  envSlug: string
) =>
  cloudFetch(
    `/v1/cloud/orgs/${orgId}/projects/${projectId}/envs/${envSlug}/secrets/${encodeURIComponent(key)}`,
    { method: "DELETE" }
  );

// ── Service Tokens ───────────────────────────────────────────────────
// Backend routes: /v1/cloud/orgs/{org_id}/projects/{project_id}/tokens

export async function listServiceTokens(orgId: string, projectId: string): Promise<ServiceToken[]> {
  const res = await cloudFetch<{ tokens: ServiceToken[] }>(
    `/v1/cloud/orgs/${orgId}/projects/${projectId}/tokens`
  );
  return res.tokens;
}

export async function createServiceToken(
  orgId: string,
  projectId: string,
  name: string,
  environmentId?: string,
  permissions?: string[],
  expiresInDays?: number
): Promise<{ token: string; service_token: ServiceToken }> {
  const res = await cloudFetch<{ plaintext_token: string; token: ServiceToken }>(
    `/v1/cloud/orgs/${orgId}/projects/${projectId}/tokens`,
    {
      method: "POST",
      body: JSON.stringify({
        name,
        environment_id: environmentId,
        permissions: permissions ?? ["read"],
        expires_at: expiresInDays
          ? new Date(Date.now() + expiresInDays * 86400000).toISOString()
          : undefined,
      }),
    }
  );
  // Remap backend field names to what the UI expects
  return { token: res.plaintext_token, service_token: res.token };
}

export const revokeServiceToken = (
  orgId: string,
  projectId: string,
  tokenId: string
) =>
  cloudFetch(
    `/v1/cloud/orgs/${orgId}/projects/${projectId}/tokens/${tokenId}/revoke`,
    { method: "POST" }
  );

// ── Audit ────────────────────────────────────────────────────────────

export async function listAudit(
  orgId: string,
  projectId: string,
  limit = 50,
  offset = 0
): Promise<AuditEntry[]> {
  const res = await cloudFetch<AuditEntry[]>(
    `/v1/cloud/orgs/${orgId}/projects/${projectId}/audit?limit=${limit}&offset=${offset}`
  );
  return res;
}

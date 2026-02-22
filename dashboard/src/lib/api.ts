/** Base URL for the ZVault API server. */
const API_BASE = import.meta.env.VITE_API_URL ?? "";

// ---------------------------------------------------------------------------
// Vault token (cookie-based, for local vault)
// ---------------------------------------------------------------------------

/** Get the vault token from cookies. */
export function getToken(): string | null {
  const match = document.cookie.match(/(?:^|; )zvault-token=([^;]*)/);
  return match ? decodeURIComponent(match[1]) : null;
}

/** Set the vault token cookie (24h expiry). */
export function setToken(token: string) {
  document.cookie = `zvault-token=${encodeURIComponent(token)};path=/;max-age=86400;SameSite=Strict`;
}

/** Clear the vault token cookie. */
export function clearToken() {
  document.cookie = "zvault-token=;path=/;max-age=0";
}

// ---------------------------------------------------------------------------
// Cloud token getter (for Clerk or any cloud auth provider)
// ---------------------------------------------------------------------------

/** In-memory store for the cloud access token getter. */
let _getCloudToken: (() => Promise<string | null>) | null = null;

/** Register the cloud token getter (called once from DashboardLayout). */
export function setCloudTokenGetter(getter: () => Promise<string | null>) {
  _getCloudToken = getter;
}

/** Clear the cloud token getter on logout. */
export function clearCloudTokenGetter() {
  _getCloudToken = null;
}

// ---------------------------------------------------------------------------
// Fetch wrappers
// ---------------------------------------------------------------------------

/** Typed fetch wrapper that injects the vault token header. */
export async function vaultFetch<T = unknown>(
  path: string,
  options?: RequestInit
): Promise<T> {
  const token = getToken();
  const headers: Record<string, string> = {
    "Content-Type": "application/json",
    ...(options?.headers as Record<string, string>),
  };
  if (token) headers["X-Vault-Token"] = token;

  const res = await fetch(`${API_BASE}${path}`, { ...options, headers });
  if (!res.ok) {
    const body = await res.json().catch(() => null);
    throw new ApiError(res.status, body?.message ?? res.statusText);
  }
  return res.json() as Promise<T>;
}

/**
 * Fetch wrapper for cloud API calls — uses Clerk Bearer token.
 *
 * Falls back to vault token if Clerk is not configured.
 */
export async function cloudFetch<T = unknown>(
  path: string,
  options?: RequestInit
): Promise<T> {
  const headers: Record<string, string> = {
    "Content-Type": "application/json",
    ...(options?.headers as Record<string, string>),
  };

  // Prefer cloud token (Clerk) for cloud endpoints.
  if (_getCloudToken) {
    try {
      const accessToken = await _getCloudToken();
      if (accessToken) {
        headers["Authorization"] = `Bearer ${accessToken}`;
      }
    } catch {
      // Token fetch failed — fall through to vault token.
    }
  }

  // Fallback: vault token.
  if (!headers["Authorization"]) {
    const vaultToken = getToken();
    if (vaultToken) headers["X-Vault-Token"] = vaultToken;
  }

  const res = await fetch(`${API_BASE}${path}`, { ...options, headers });
  if (!res.ok) {
    const body = await res.json().catch(() => null);
    throw new ApiError(res.status, body?.message ?? res.statusText);
  }
  return res.json() as Promise<T>;
}

export class ApiError extends Error {
  constructor(
    public status: number,
    message: string
  ) {
    super(message);
    this.name = "ApiError";
  }
}

/** Seal status response shape. */
export interface SealStatus {
  initialized: boolean;
  sealed: boolean;
  threshold: number;
  shares: number;
  progress?: number;
}

export const getSealStatus = () =>
  vaultFetch<SealStatus>("/v1/sys/seal-status");

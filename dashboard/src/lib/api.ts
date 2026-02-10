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

  const res = await fetch(path, { ...options, headers });
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

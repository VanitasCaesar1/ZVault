/**
 * Server-side ZVault client for Next.js.
 *
 * Use in Server Components, API Routes, and middleware.
 * Never import this in client components — secrets stay server-side.
 */

import type { ZVaultNextConfig, SecretEntry } from './types';

const DEFAULT_BASE_URL = 'https://api.zvault.cloud';
const DEFAULT_CACHE_TTL = 300_000; // 5 min
const DEFAULT_TIMEOUT = 10_000;
const MAX_RETRIES = 2;
const RETRY_BASE_DELAY = 300;

interface CacheEntry {
  value: string;
  expiresAt: number;
}

/** In-memory secret cache (lives for the duration of the server process). */
const cache = new Map<string, CacheEntry>();
let cacheTtl = DEFAULT_CACHE_TTL;

function resolveConfig(override?: ZVaultNextConfig): Required<
  Pick<ZVaultNextConfig, 'token' | 'orgId' | 'projectId' | 'env' | 'url'>
> & { cacheTtl: number } {
  const token = override?.token ?? process.env.ZVAULT_TOKEN ?? '';
  const orgId = override?.orgId ?? process.env.ZVAULT_ORG_ID ?? '';
  const projectId = override?.projectId ?? process.env.ZVAULT_PROJECT_ID ?? '';
  const env = override?.env ?? process.env.ZVAULT_ENV ?? 'production';
  const url = (override?.url ?? process.env.ZVAULT_URL ?? DEFAULT_BASE_URL).replace(/\/+$/, '');
  const ttl = override?.cacheTtl ?? DEFAULT_CACHE_TTL;
  cacheTtl = ttl;
  return { token, orgId, projectId, env, url, cacheTtl: ttl };
}

async function request<T>(
  url: string,
  token: string,
  retries = MAX_RETRIES,
): Promise<T> {
  let lastErr: Error | undefined;

  for (let i = 0; i <= retries; i++) {
    try {
      const controller = new AbortController();
      const tid = setTimeout(() => controller.abort(), DEFAULT_TIMEOUT);

      const res = await fetch(url, {
        headers: {
          Authorization: `Bearer ${token}`,
          'Content-Type': 'application/json',
          'User-Agent': '@zvault/next/0.1.0',
        },
        signal: controller.signal,
        cache: 'no-store',
      });

      clearTimeout(tid);

      if (res.ok) {
        if (res.status === 204) return undefined as T;
        return (await res.json()) as T;
      }

      if (res.status === 401 || res.status === 403 || res.status === 404) {
        const body = await res.json().catch(() => null);
        throw new Error(body?.error?.message ?? `HTTP ${res.status}`);
      }

      lastErr = new Error(`HTTP ${res.status}`);
      if (i < retries) {
        await new Promise((r) => setTimeout(r, RETRY_BASE_DELAY * 2 ** i));
        continue;
      }
    } catch (err) {
      lastErr = err instanceof Error ? err : new Error(String(err));
      if (i < retries) {
        await new Promise((r) => setTimeout(r, RETRY_BASE_DELAY * 2 ** i));
        continue;
      }
    }
  }

  throw lastErr ?? new Error('ZVault request failed');
}

/**
 * Fetch a single secret by key. Results are cached in-memory.
 *
 * @example
 * ```ts
 * // app/api/route.ts
 * import { getSecret } from '@zvault/next';
 * const dbUrl = await getSecret('DATABASE_URL');
 * ```
 */
export async function getSecret(
  key: string,
  config?: ZVaultNextConfig,
): Promise<string> {
  const cfg = resolveConfig(config);
  const cacheKey = `${cfg.env}:${key}`;

  const cached = cache.get(cacheKey);
  if (cached && cached.expiresAt > Date.now()) {
    return cached.value;
  }

  const url = `${cfg.url}/v1/cloud/orgs/${cfg.orgId}/projects/${cfg.projectId}/envs/${cfg.env}/secrets/${encodeURIComponent(key)}`;
  const res = await request<{ secret: SecretEntry }>(url, cfg.token);

  cache.set(cacheKey, {
    value: res.secret.value,
    expiresAt: Date.now() + cfg.cacheTtl,
  });

  return res.secret.value;
}

/**
 * Fetch all secrets for an environment. Returns a plain object.
 *
 * @example
 * ```ts
 * const secrets = await getAllSecrets({ env: 'staging' });
 * console.log(secrets.STRIPE_KEY);
 * ```
 */
export async function getAllSecrets(
  config?: ZVaultNextConfig,
): Promise<Record<string, string>> {
  const cfg = resolveConfig(config);

  // Fetch key list
  const keysUrl = `${cfg.url}/v1/cloud/orgs/${cfg.orgId}/projects/${cfg.projectId}/envs/${cfg.env}/secrets`;
  const keysRes = await request<{ keys: Array<{ key: string }> }>(keysUrl, cfg.token);

  // Fetch values in parallel (batches of 20)
  const result: Record<string, string> = {};
  const batchSize = 20;

  for (let i = 0; i < keysRes.keys.length; i += batchSize) {
    const batch = keysRes.keys.slice(i, i + batchSize);
    const settled = await Promise.allSettled(
      batch.map((k) =>
        request<{ secret: SecretEntry }>(
          `${cfg.url}/v1/cloud/orgs/${cfg.orgId}/projects/${cfg.projectId}/envs/${cfg.env}/secrets/${encodeURIComponent(k.key)}`,
          cfg.token,
        ),
      ),
    );

    for (const entry of settled) {
      if (entry.status === 'fulfilled') {
        const s = entry.value.secret;
        result[s.key] = s.value;
        cache.set(`${cfg.env}:${s.key}`, {
          value: s.value,
          expiresAt: Date.now() + cfg.cacheTtl,
        });
      }
    }
  }

  return result;
}

/**
 * Get a pre-configured ZVault client instance for advanced usage.
 * Returns an object with `get` and `getAll` methods bound to the config.
 */
export function getZVaultClient(config?: ZVaultNextConfig) {
  const cfg = config;
  return {
    get: (key: string) => getSecret(key, cfg),
    getAll: () => getAllSecrets(cfg),
  };
}

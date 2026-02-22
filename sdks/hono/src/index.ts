/**
 * @zvault/hono — ZVault middleware for Hono
 *
 * @example
 * ```ts
 * import { Hono } from 'hono';
 * import { zvault } from '@zvault/hono';
 *
 * const app = new Hono();
 *
 * app.use(zvault({ env: 'production' }));
 *
 * app.get('/', (c) => {
 *   const dbUrl = c.get('secrets').DATABASE_URL;
 *   return c.json({ ok: true });
 * });
 * ```
 */

import type { Context, MiddlewareHandler } from 'hono';

export interface ZVaultHonoConfig {
  /** Service token. Defaults to ZVAULT_TOKEN env var. */
  token?: string;
  /** Organization ID. Defaults to ZVAULT_ORG_ID env var. */
  orgId?: string;
  /** Project ID. Defaults to ZVAULT_PROJECT_ID env var. */
  projectId?: string;
  /** Environment slug. Defaults to ZVAULT_ENV or 'production'. */
  env?: string;
  /** API base URL. Defaults to ZVAULT_URL or 'https://api.zvault.cloud'. */
  url?: string;
  /** Cache TTL in ms. Default: 300000 (5 min). */
  cacheTtl?: number;
}

// Augment Hono context
declare module 'hono' {
  interface ContextVariableMap {
    secrets: Record<string, string>;
  }
}

const DEFAULT_BASE_URL = 'https://api.zvault.cloud';
const DEFAULT_CACHE_TTL = 300_000;
const DEFAULT_TIMEOUT = 10_000;
const MAX_RETRIES = 2;

interface CachedSecrets {
  data: Record<string, string>;
  expiresAt: number;
}

let cached: CachedSecrets | null = null;

function envVar(name: string): string | undefined {
  try {
    const v = typeof process !== 'undefined' ? process.env[name] : undefined;
    return v && v.length > 0 ? v : undefined;
  } catch {
    return undefined;
  }
}

async function fetchWithRetry(url: string, token: string, retries = MAX_RETRIES): Promise<unknown> {
  let lastErr: Error | undefined;

  for (let i = 0; i <= retries; i++) {
    try {
      const controller = new AbortController();
      const tid = setTimeout(() => controller.abort(), DEFAULT_TIMEOUT);

      const res = await fetch(url, {
        headers: {
          Authorization: `Bearer ${token}`,
          'Content-Type': 'application/json',
          'User-Agent': '@zvault/hono/0.1.0',
        },
        signal: controller.signal,
      });

      clearTimeout(tid);

      if (res.ok) return res.json();

      lastErr = new Error(`HTTP ${res.status}`);
      if (res.status < 500 && res.status !== 429) throw lastErr;
    } catch (err) {
      lastErr = err instanceof Error ? err : new Error(String(err));
    }

    if (i < retries) {
      await new Promise((r) => setTimeout(r, 300 * 2 ** i));
    }
  }

  throw lastErr ?? new Error('ZVault request failed');
}

async function fetchSecrets(
  baseUrl: string,
  token: string,
  orgId: string,
  projectId: string,
  envSlug: string,
): Promise<Record<string, string>> {
  const keysUrl = `${baseUrl}/v1/cloud/orgs/${orgId}/projects/${projectId}/envs/${envSlug}/secrets`;
  const keysRes = keysUrl ? await fetchWithRetry(keysUrl, token) : { keys: [] };
  const { keys } = keysRes as { keys: Array<{ key: string }> };

  const result: Record<string, string> = {};
  const batchSize = 20;

  for (let i = 0; i < keys.length; i += batchSize) {
    const batch = keys.slice(i, i + batchSize);
    const settled = await Promise.allSettled(
      batch.map((k) =>
        fetchWithRetry(
          `${baseUrl}/v1/cloud/orgs/${orgId}/projects/${projectId}/envs/${envSlug}/secrets/${encodeURIComponent(k.key)}`,
          token,
        ),
      ),
    );

    for (const entry of settled) {
      if (entry.status === 'fulfilled') {
        const s = (entry.value as { secret: { key: string; value: string } }).secret;
        result[s.key] = s.value;
      }
    }
  }

  return result;
}

/**
 * Hono middleware that fetches secrets and sets them on `c.get('secrets')`.
 *
 * Secrets are cached in-memory and refreshed on TTL expiry.
 * On failure, serves stale cache or empty object (graceful degradation).
 */
export function zvault(config?: ZVaultHonoConfig): MiddlewareHandler {
  const token = config?.token ?? envVar('ZVAULT_TOKEN') ?? '';
  const orgId = config?.orgId ?? envVar('ZVAULT_ORG_ID') ?? '';
  const projectId = config?.projectId ?? envVar('ZVAULT_PROJECT_ID') ?? '';
  const envSlug = config?.env ?? envVar('ZVAULT_ENV') ?? 'production';
  const baseUrl = (config?.url ?? envVar('ZVAULT_URL') ?? DEFAULT_BASE_URL).replace(/\/+$/, '');
  const ttl = config?.cacheTtl ?? DEFAULT_CACHE_TTL;

  if (!token || !orgId || !projectId) {
    console.warn('[zvault] Missing config — middleware will pass through with empty secrets.');
    return async (c: Context, next) => {
      c.set('secrets', {});
      await next();
    };
  }

  return async (c: Context, next) => {
    try {
      if (cached && cached.expiresAt > Date.now()) {
        c.set('secrets', cached.data);
        await next();
        return;
      }

      const secrets = await fetchSecrets(baseUrl, token, orgId, projectId, envSlug);
      cached = { data: secrets, expiresAt: Date.now() + ttl };
      c.set('secrets', secrets);
      await next();
    } catch (err) {
      c.set('secrets', cached?.data ?? {});
      console.warn(
        `[zvault] Failed to fetch secrets: ${err instanceof Error ? err.message : String(err)}`,
      );
      await next();
    }
  };
}

/**
 * One-shot: fetch all secrets and inject into process.env.
 * Call at app startup before routes are registered.
 */
export async function inject(config?: ZVaultHonoConfig): Promise<number> {
  const token = config?.token ?? envVar('ZVAULT_TOKEN') ?? '';
  const orgId = config?.orgId ?? envVar('ZVAULT_ORG_ID') ?? '';
  const projectId = config?.projectId ?? envVar('ZVAULT_PROJECT_ID') ?? '';
  const envSlug = config?.env ?? envVar('ZVAULT_ENV') ?? 'production';
  const baseUrl = (config?.url ?? envVar('ZVAULT_URL') ?? DEFAULT_BASE_URL).replace(/\/+$/, '');

  if (!token || !orgId || !projectId) {
    console.warn('[zvault] Missing config — skipping injection.');
    return 0;
  }

  const secrets = await fetchSecrets(baseUrl, token, orgId, projectId, envSlug);
  let count = 0;

  for (const [k, v] of Object.entries(secrets)) {
    if (typeof process !== 'undefined' && process.env[k] === undefined) {
      process.env[k] = v;
      count++;
    }
  }

  console.log(`[zvault] Injected ${count} secrets from "${envSlug}"`);
  return count;
}

export type { ZVaultHonoConfig as Config };

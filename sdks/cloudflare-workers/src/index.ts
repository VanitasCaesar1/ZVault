/**
 * @zvault/cloudflare-workers — ZVault integration for Cloudflare Workers.
 *
 * Fetches secrets from ZVault Cloud and caches them in Workers KV.
 *
 * @example
 * ```ts
 * import { ZVault } from '@zvault/cloudflare-workers';
 *
 * export default {
 *   async fetch(request, env) {
 *     const vault = new ZVault(env);
 *     const dbUrl = await vault.get('DATABASE_URL');
 *     return new Response('ok');
 *   },
 * };
 * ```
 */

export interface ZVaultEnv {
  ZVAULT_TOKEN: string;
  ZVAULT_ORG_ID: string;
  ZVAULT_PROJECT_ID: string;
  ZVAULT_ENV?: string;
  ZVAULT_URL?: string;
  /** Optional KV namespace for caching */
  ZVAULT_KV?: KVNamespace;
}

const DEFAULT_BASE_URL = 'https://api.zvault.cloud';
const DEFAULT_CACHE_TTL = 300; // 5 minutes
const MAX_RETRIES = 2;

/** In-memory cache for the lifetime of the Worker isolate */
let memCache: { data: Record<string, string>; expiresAt: number } | null = null;

export class ZVault {
  private token: string;
  private orgId: string;
  private projectId: string;
  private envSlug: string;
  private baseUrl: string;
  private kv?: KVNamespace;

  constructor(env: ZVaultEnv) {
    this.token = env.ZVAULT_TOKEN || '';
    this.orgId = env.ZVAULT_ORG_ID || '';
    this.projectId = env.ZVAULT_PROJECT_ID || '';
    this.envSlug = env.ZVAULT_ENV || 'production';
    this.baseUrl = (env.ZVAULT_URL || DEFAULT_BASE_URL).replace(/\/+$/, '');
    this.kv = env.ZVAULT_KV;
  }

  /** Fetch all secrets for the configured environment. */
  async getAll(): Promise<Record<string, string>> {
    // Check in-memory cache
    if (memCache && memCache.expiresAt > Date.now()) {
      return memCache.data;
    }

    // Check KV cache
    if (this.kv) {
      const cached = await this.kv.get('zvault:secrets', 'json');
      if (cached) {
        const data = cached as Record<string, string>;
        memCache = { data, expiresAt: Date.now() + DEFAULT_CACHE_TTL * 1000 };
        return data;
      }
    }

    // Fetch from API
    const url = `${this.baseUrl}/v1/cloud/orgs/${this.orgId}/projects/${this.projectId}/envs/${this.envSlug}/secrets`;
    const body = await this.fetchWithRetry(url);
    const secrets: Record<string, string> = {};

    for (const s of (body as { secrets: Array<{ key: string; value: string }> }).secrets || []) {
      secrets[s.key] = s.value;
    }

    // Cache in memory
    memCache = { data: secrets, expiresAt: Date.now() + DEFAULT_CACHE_TTL * 1000 };

    // Cache in KV (async, don't block)
    if (this.kv) {
      await this.kv.put('zvault:secrets', JSON.stringify(secrets), {
        expirationTtl: DEFAULT_CACHE_TTL,
      });
    }

    return secrets;
  }

  /** Fetch a single secret by key. */
  async get(key: string): Promise<string> {
    const all = await this.getAll();
    if (!(key in all)) {
      throw new Error(`Secret not found: ${key}`);
    }
    return all[key];
  }

  private async fetchWithRetry(url: string): Promise<unknown> {
    let lastErr: Error | undefined;

    for (let i = 0; i <= MAX_RETRIES; i++) {
      try {
        const res = await fetch(url, {
          headers: {
            Authorization: `Bearer ${this.token}`,
            'Content-Type': 'application/json',
            'User-Agent': '@zvault/cloudflare-workers/0.1.0',
          },
        });

        if (res.ok) return res.json();

        lastErr = new Error(`HTTP ${res.status}`);
        if (res.status < 500 && res.status !== 429) throw lastErr;
      } catch (err) {
        lastErr = err instanceof Error ? err : new Error(String(err));
      }

      if (i < MAX_RETRIES) {
        await new Promise((r) => setTimeout(r, 300 * 2 ** i));
      }
    }

    throw lastErr ?? new Error('ZVault request failed');
  }
}

export default { ZVault };

import type {
  ZVaultConfig,
  SecretEntry,
  SecretKey,
  SecretResponse,
  SecretKeysResponse,
  ApiErrorBody,
  HealthStatus,
} from './types.js';
import {
  ZVaultApiError,
  ZVaultAuthError,
  ZVaultConfigError,
  ZVaultNotFoundError,
  ZVaultTimeoutError,
} from './errors.js';
import { SecretCache } from './cache.js';

const DEFAULT_BASE_URL = 'https://api.zvault.cloud';
const DEFAULT_CACHE_TTL = 300_000; // 5 minutes
const DEFAULT_TIMEOUT = 10_000; // 10 seconds
const DEFAULT_MAX_RETRIES = 3;
const RETRY_BASE_DELAY = 500; // ms

/**
 * ZVault SDK client.
 *
 * Fetches secrets from ZVault Cloud at runtime. Caches in-memory,
 * auto-refreshes on TTL, and gracefully degrades if the API is unreachable.
 *
 * @example
 * ```typescript
 * const vault = new ZVault({ token: process.env.ZVAULT_TOKEN });
 * const secrets = await vault.getAll('production');
 * const dbUrl = secrets.get('DATABASE_URL');
 * ```
 */
export class ZVault {
  private readonly token: string;
  private readonly baseUrl: string;
  private readonly orgId: string;
  private readonly projectId: string;
  private readonly defaultEnv: string;
  private readonly timeout: number;
  private readonly maxRetries: number;
  private readonly debug: boolean;
  private readonly autoRefresh: boolean;
  private readonly cache: SecretCache;
  private refreshTimers = new Map<string, ReturnType<typeof setInterval>>();
  private lastRefresh: Date | null = null;

  constructor(config: ZVaultConfig = {}) {
    this.token = config.token ?? env('ZVAULT_TOKEN') ?? '';
    this.baseUrl = (config.baseUrl ?? env('ZVAULT_URL') ?? DEFAULT_BASE_URL).replace(/\/+$/, '');
    this.orgId = config.orgId ?? env('ZVAULT_ORG_ID') ?? '';
    this.projectId = config.projectId ?? env('ZVAULT_PROJECT_ID') ?? '';
    this.defaultEnv = config.defaultEnv ?? env('ZVAULT_ENV') ?? 'development';
    this.timeout = config.timeout ?? DEFAULT_TIMEOUT;
    this.maxRetries = config.maxRetries ?? DEFAULT_MAX_RETRIES;
    this.debug = config.debug ?? false;
    this.autoRefresh = config.autoRefresh ?? true;

    const ttl = config.cacheTtl ?? DEFAULT_CACHE_TTL;
    this.cache = new SecretCache(ttl);

    if (!this.token) {
      throw new ZVaultConfigError(
        'Missing token. Set ZVAULT_TOKEN env var or pass { token } in config.',
      );
    }
  }

  /**
   * Fetch all secrets for an environment in a single HTTP call.
   *
   * Returns a `Map<string, string>` of key → value. Results are cached
   * in-memory and auto-refreshed on TTL if `autoRefresh` is enabled.
   *
   * On network failure, returns last-known cached values (graceful degradation).
   */
  async getAll(envSlug?: string): Promise<Map<string, string>> {
    const env = envSlug ?? this.defaultEnv;
    this.requireProjectConfig();

    try {
      // Fetch key list first
      const keysRes = await this.request<SecretKeysResponse>(
        'GET',
        `/orgs/${this.orgId}/projects/${this.projectId}/envs/${env}/secrets`,
      );

      // Fetch each secret value in parallel (batched)
      const secrets = new Map<string, string>();
      const batchSize = 20;
      const keys = keysRes.keys;

      for (let i = 0; i < keys.length; i += batchSize) {
        const batch = keys.slice(i, i + batchSize);
        const results = await Promise.allSettled(
          batch.map((k) =>
            this.request<SecretResponse>(
              'GET',
              `/orgs/${this.orgId}/projects/${this.projectId}/envs/${env}/secrets/${encodeURIComponent(k.key)}`,
            ),
          ),
        );

        for (const result of results) {
          if (result.status === 'fulfilled') {
            secrets.set(result.value.secret.key, result.value.secret.value);
          }
        }
      }

      // Update cache
      this.cache.setAll(env, secrets);
      this.lastRefresh = new Date();

      // Start auto-refresh if enabled and not already running
      if (this.autoRefresh && !this.refreshTimers.has(env)) {
        this.startAutoRefresh(env);
      }

      this.log(`Fetched ${secrets.size} secrets for env "${env}"`);
      return secrets;
    } catch (err) {
      // Graceful degradation: return cached values on failure
      const cached = this.cache.getAll(env);
      if (cached.size > 0) {
        this.log(`API unreachable, serving ${cached.size} cached secrets for "${env}"`);
        return cached;
      }
      throw err;
    }
  }

  /**
   * Fetch a single secret by key.
   *
   * Checks cache first, then fetches from API. Returns the secret value string.
   *
   * @throws {ZVaultNotFoundError} if the secret doesn't exist.
   */
  async get(key: string, envSlug?: string): Promise<string> {
    const env = envSlug ?? this.defaultEnv;
    this.requireProjectConfig();

    // Check cache first
    const cached = this.cache.get(`${env}:${key}`);
    if (cached !== undefined) {
      this.log(`Cache hit for "${key}" in "${env}"`);
      return cached;
    }

    try {
      const res = await this.request<SecretResponse>(
        'GET',
        `/orgs/${this.orgId}/projects/${this.projectId}/envs/${env}/secrets/${encodeURIComponent(key)}`,
      );

      this.cache.set(`${env}:${key}`, res.secret.value);
      return res.secret.value;
    } catch (err) {
      if (err instanceof ZVaultApiError && err.statusCode === 404) {
        throw new ZVaultNotFoundError(key, env);
      }
      throw err;
    }
  }

  /**
   * List secret keys (no values) for an environment.
   */
  async listKeys(envSlug?: string): Promise<SecretKey[]> {
    const env = envSlug ?? this.defaultEnv;
    this.requireProjectConfig();

    const res = await this.request<SecretKeysResponse>(
      'GET',
      `/orgs/${this.orgId}/projects/${this.projectId}/envs/${env}/secrets`,
    );
    return res.keys;
  }

  /**
   * Set a secret value. Requires a token with write permission.
   */
  async set(key: string, value: string, envSlug?: string, comment?: string): Promise<SecretEntry> {
    const env = envSlug ?? this.defaultEnv;
    this.requireProjectConfig();

    const res = await this.request<SecretResponse>(
      'PUT',
      `/orgs/${this.orgId}/projects/${this.projectId}/envs/${env}/secrets/${encodeURIComponent(key)}`,
      { value, comment: comment ?? '' },
    );

    // Update cache
    this.cache.set(`${env}:${key}`, value);
    return res.secret;
  }

  /**
   * Delete a secret. Requires a token with write permission.
   */
  async delete(key: string, envSlug?: string): Promise<void> {
    const env = envSlug ?? this.defaultEnv;
    this.requireProjectConfig();

    await this.request(
      'DELETE',
      `/orgs/${this.orgId}/projects/${this.projectId}/envs/${env}/secrets/${encodeURIComponent(key)}`,
    );

    // Remove from cache
    this.cache.set(`${env}:${key}`, ''); // will expire
  }

  /**
   * Inject all secrets into `process.env` for the given environment.
   *
   * Existing env vars are NOT overwritten unless `overwrite` is true.
   */
  async injectIntoEnv(envSlug?: string, overwrite = false): Promise<number> {
    const secrets = await this.getAll(envSlug);
    let injected = 0;

    for (const [key, value] of secrets) {
      if (overwrite || process.env[key] === undefined) {
        process.env[key] = value;
        injected++;
      }
    }

    this.log(`Injected ${injected} secrets into process.env`);
    return injected;
  }

  /**
   * Check if the ZVault API is reachable and the token is valid.
   */
  async healthy(): Promise<HealthStatus> {
    const start = Date.now();
    try {
      await this.request('GET', '/me');
      return {
        ok: true,
        latencyMs: Date.now() - start,
        cachedSecrets: this.cache.size,
        lastRefresh: this.lastRefresh,
      };
    } catch {
      return {
        ok: false,
        latencyMs: Date.now() - start,
        cachedSecrets: this.cache.size,
        lastRefresh: this.lastRefresh,
      };
    }
  }

  /**
   * Stop all background refresh timers and clear the cache.
   *
   * Call this when shutting down to prevent leaked timers.
   */
  destroy(): void {
    for (const timer of this.refreshTimers.values()) {
      clearInterval(timer);
    }
    this.refreshTimers.clear();
    this.cache.clear();
    this.log('Client destroyed');
  }

  // --- Private methods ---

  private requireProjectConfig(): void {
    if (!this.orgId) {
      throw new ZVaultConfigError(
        'Missing orgId. Set ZVAULT_ORG_ID env var or pass { orgId } in config.',
      );
    }
    if (!this.projectId) {
      throw new ZVaultConfigError(
        'Missing projectId. Set ZVAULT_PROJECT_ID env var or pass { projectId } in config.',
      );
    }
  }

  private startAutoRefresh(envSlug: string): void {
    // Refresh at 80% of cache TTL to avoid serving stale data
    const interval = Math.max(10_000, Math.floor((this.cache as any).ttlMs * 0.8));
    const timer = setInterval(() => {
      this.getAll(envSlug).catch((err) => {
        this.log(`Auto-refresh failed for "${envSlug}": ${err instanceof Error ? err.message : String(err)}`);
      });
    }, interval);

    // Unref so the timer doesn't prevent Node.js from exiting
    if (typeof timer === 'object' && 'unref' in timer) {
      timer.unref();
    }

    this.refreshTimers.set(envSlug, timer);
    this.log(`Auto-refresh started for "${envSlug}" every ${interval}ms`);
  }

  private async request<T = unknown>(
    method: string,
    path: string,
    body?: unknown,
  ): Promise<T> {
    const url = `${this.baseUrl}/v1/cloud${path}`;
    let lastError: Error | undefined;

    for (let attempt = 0; attempt <= this.maxRetries; attempt++) {
      try {
        const controller = new AbortController();
        const timeoutId = setTimeout(() => controller.abort(), this.timeout);

        const headers: Record<string, string> = {
          Authorization: `Bearer ${this.token}`,
          'Content-Type': 'application/json',
          'User-Agent': '@zvault/sdk-node/0.1.0',
        };

        const res = await fetch(url, {
          method,
          headers,
          body: body ? JSON.stringify(body) : undefined,
          signal: controller.signal,
        });

        clearTimeout(timeoutId);

        if (res.ok) {
          // Handle 204 No Content
          if (res.status === 204) return undefined as T;
          return (await res.json()) as T;
        }

        // Parse error body
        const errorBody = await res.json().catch(() => null) as ApiErrorBody | null;
        const message = errorBody?.error?.message ?? `HTTP ${res.status}`;

        // Auth errors — don't retry
        if (res.status === 401 || res.status === 403) {
          throw new ZVaultAuthError(message);
        }

        // Not found — don't retry
        if (res.status === 404) {
          throw new ZVaultApiError(res.status, message);
        }

        // Retryable errors: 429, 500, 502, 503, 504
        if ([429, 500, 502, 503, 504].includes(res.status)) {
          lastError = new ZVaultApiError(res.status, message);

          if (attempt < this.maxRetries) {
            const delay = RETRY_BASE_DELAY * Math.pow(2, attempt);
            const jitter = Math.random() * delay * 0.3;
            this.log(`Retry ${attempt + 1}/${this.maxRetries} after ${Math.round(delay + jitter)}ms (${res.status})`);
            await sleep(delay + jitter);
            continue;
          }
        }

        throw new ZVaultApiError(res.status, message);
      } catch (err) {
        if (err instanceof ZVaultAuthError || err instanceof ZVaultNotFoundError) {
          throw err;
        }
        if (err instanceof ZVaultApiError) {
          if (![429, 500, 502, 503, 504].includes(err.statusCode)) {
            throw err;
          }
          lastError = err;
        } else if (err instanceof DOMException && err.name === 'AbortError') {
          lastError = new ZVaultTimeoutError(this.timeout);
          if (attempt < this.maxRetries) {
            const delay = RETRY_BASE_DELAY * Math.pow(2, attempt);
            this.log(`Retry ${attempt + 1}/${this.maxRetries} after timeout`);
            await sleep(delay);
            continue;
          }
        } else {
          lastError = err instanceof Error ? err : new Error(String(err));
          if (attempt < this.maxRetries) {
            const delay = RETRY_BASE_DELAY * Math.pow(2, attempt);
            this.log(`Retry ${attempt + 1}/${this.maxRetries} after network error`);
            await sleep(delay);
            continue;
          }
        }
      }
    }

    throw lastError ?? new ZVaultApiError(0, 'Unknown error');
  }

  private log(message: string): void {
    if (this.debug) {
      process.stderr.write(`[zvault-sdk] ${message}\n`);
    }
  }
}

/** Read an environment variable, returning undefined if not set or empty. */
function env(name: string): string | undefined {
  const val = process.env[name];
  return val && val.length > 0 ? val : undefined;
}

/** Sleep for the given number of milliseconds. */
function sleep(ms: number): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

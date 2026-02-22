/** In-memory secret cache with TTL-based expiration. */
export class SecretCache {
  private store = new Map<string, { value: string; expiresAt: number }>();
  private ttlMs: number;

  constructor(ttlMs: number) {
    this.ttlMs = ttlMs;
  }

  /** Get a cached value. Returns undefined if missing or expired. */
  get(key: string): string | undefined {
    const entry = this.store.get(key);
    if (!entry) return undefined;
    if (Date.now() > entry.expiresAt) {
      this.store.delete(key);
      return undefined;
    }
    return entry.value;
  }

  /** Set a value in the cache. */
  set(key: string, value: string): void {
    this.store.set(key, {
      value,
      expiresAt: Date.now() + this.ttlMs,
    });
  }

  /** Bulk-set all secrets for an environment. Clears stale entries for that env prefix. */
  setAll(env: string, secrets: Map<string, string>): void {
    // Remove old entries for this env
    const prefix = `${env}:`;
    for (const k of this.store.keys()) {
      if (k.startsWith(prefix)) {
        this.store.delete(k);
      }
    }
    // Set new entries
    for (const [key, value] of secrets) {
      this.set(`${env}:${key}`, value);
    }
  }

  /** Get all cached secrets for an environment. Returns empty map if none cached. */
  getAll(env: string): Map<string, string> {
    const result = new Map<string, string>();
    const prefix = `${env}:`;
    const now = Date.now();
    for (const [k, entry] of this.store) {
      if (k.startsWith(prefix) && now <= entry.expiresAt) {
        result.set(k.slice(prefix.length), entry.value);
      }
    }
    return result;
  }

  /** Number of non-expired entries in the cache. */
  get size(): number {
    const now = Date.now();
    let count = 0;
    for (const entry of this.store.values()) {
      if (now <= entry.expiresAt) count++;
    }
    return count;
  }

  /** Clear all cached entries. */
  clear(): void {
    this.store.clear();
  }
}

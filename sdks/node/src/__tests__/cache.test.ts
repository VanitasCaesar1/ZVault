import { describe, it, expect, beforeEach } from 'vitest';
import { SecretCache } from '../cache.js';

describe('SecretCache', () => {
  let cache: SecretCache;

  beforeEach(() => {
    cache = new SecretCache(60_000); // 1 minute TTL
  });

  it('stores and retrieves values', () => {
    cache.set('prod:DB_URL', 'postgres://...');
    expect(cache.get('prod:DB_URL')).toBe('postgres://...');
  });

  it('returns undefined for missing keys', () => {
    expect(cache.get('nonexistent')).toBeUndefined();
  });

  it('expires entries after TTL', () => {
    const shortCache = new SecretCache(1); // 1ms TTL
    shortCache.set('key', 'value');

    // Wait for expiry
    return new Promise<void>((resolve) => {
      setTimeout(() => {
        expect(shortCache.get('key')).toBeUndefined();
        resolve();
      }, 10);
    });
  });

  it('setAll replaces all entries for an env', () => {
    cache.set('prod:OLD_KEY', 'old');
    cache.setAll('prod', new Map([['NEW_KEY', 'new']]));

    expect(cache.get('prod:OLD_KEY')).toBeUndefined();
    expect(cache.get('prod:NEW_KEY')).toBe('new');
  });

  it('getAll returns all entries for an env', () => {
    cache.set('prod:A', '1');
    cache.set('prod:B', '2');
    cache.set('staging:C', '3');

    const prodSecrets = cache.getAll('prod');
    expect(prodSecrets.size).toBe(2);
    expect(prodSecrets.get('A')).toBe('1');
    expect(prodSecrets.get('B')).toBe('2');
  });

  it('tracks size correctly', () => {
    expect(cache.size).toBe(0);
    cache.set('a', '1');
    cache.set('b', '2');
    expect(cache.size).toBe(2);
  });

  it('clear removes all entries', () => {
    cache.set('a', '1');
    cache.set('b', '2');
    cache.clear();
    expect(cache.size).toBe(0);
  });
});

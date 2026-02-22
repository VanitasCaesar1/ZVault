import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { ZVault } from '../client.js';
import { ZVaultConfigError, ZVaultNotFoundError, ZVaultAuthError } from '../errors.js';

// Mock fetch globally
const mockFetch = vi.fn();
vi.stubGlobal('fetch', mockFetch);

function jsonResponse(data: unknown, status = 200) {
  return {
    ok: status >= 200 && status < 300,
    status,
    json: () => Promise.resolve(data),
  };
}

describe('ZVault', () => {
  beforeEach(() => {
    mockFetch.mockReset();
    process.env.ZVAULT_TOKEN = 'zvt_test_token_123';
    process.env.ZVAULT_ORG_ID = 'org-123';
    process.env.ZVAULT_PROJECT_ID = 'proj-456';
  });

  afterEach(() => {
    delete process.env.ZVAULT_TOKEN;
    delete process.env.ZVAULT_ORG_ID;
    delete process.env.ZVAULT_PROJECT_ID;
  });

  it('throws ZVaultConfigError when token is missing', () => {
    delete process.env.ZVAULT_TOKEN;
    expect(() => new ZVault()).toThrow(ZVaultConfigError);
  });

  it('creates client from env vars', () => {
    const vault = new ZVault();
    vault.destroy();
    // No error means success
  });

  it('creates client from explicit config', () => {
    delete process.env.ZVAULT_TOKEN;
    const vault = new ZVault({
      token: 'zvt_explicit',
      orgId: 'org-1',
      projectId: 'proj-1',
    });
    vault.destroy();
  });

  describe('get()', () => {
    it('fetches a single secret', async () => {
      mockFetch.mockResolvedValueOnce(
        jsonResponse({
          secret: {
            key: 'DB_URL',
            value: 'postgres://localhost/mydb',
            version: 1,
            comment: '',
            created_at: '2026-01-01T00:00:00Z',
            updated_at: '2026-01-01T00:00:00Z',
          },
        }),
      );

      const vault = new ZVault({ defaultEnv: 'production' });
      const value = await vault.get('DB_URL');
      vault.destroy();

      expect(value).toBe('postgres://localhost/mydb');
      expect(mockFetch).toHaveBeenCalledOnce();

      const [url, opts] = mockFetch.mock.calls[0];
      expect(url).toContain('/envs/production/secrets/DB_URL');
      expect(opts.headers.Authorization).toBe('Bearer zvt_test_token_123');
    });

    it('returns cached value on second call', async () => {
      mockFetch.mockResolvedValueOnce(
        jsonResponse({
          secret: {
            key: 'API_KEY',
            value: 'sk_live_123',
            version: 1,
            comment: '',
            created_at: '2026-01-01T00:00:00Z',
            updated_at: '2026-01-01T00:00:00Z',
          },
        }),
      );

      const vault = new ZVault();
      await vault.get('API_KEY', 'dev');
      const cached = await vault.get('API_KEY', 'dev');
      vault.destroy();

      expect(cached).toBe('sk_live_123');
      expect(mockFetch).toHaveBeenCalledOnce(); // Only 1 fetch, second was cache hit
    });

    it('throws ZVaultNotFoundError on 404', async () => {
      mockFetch.mockResolvedValueOnce(
        jsonResponse({ error: { code: 404, message: 'not found' } }, 404),
      );

      const vault = new ZVault();
      await expect(vault.get('MISSING', 'prod')).rejects.toThrow(ZVaultNotFoundError);
      vault.destroy();
    });

    it('throws ZVaultAuthError on 401', async () => {
      mockFetch.mockResolvedValueOnce(
        jsonResponse({ error: { code: 401, message: 'unauthorized' } }, 401),
      );

      const vault = new ZVault();
      await expect(vault.get('KEY', 'prod')).rejects.toThrow(ZVaultAuthError);
      vault.destroy();
    });
  });

  describe('getAll()', () => {
    it('fetches all secrets for an environment', async () => {
      // First call: list keys
      mockFetch.mockResolvedValueOnce(
        jsonResponse({
          keys: [
            { key: 'DB_URL', version: 1, comment: '', updated_at: '2026-01-01T00:00:00Z' },
            { key: 'API_KEY', version: 1, comment: '', updated_at: '2026-01-01T00:00:00Z' },
          ],
        }),
      );

      // Second + third calls: individual secrets
      mockFetch.mockResolvedValueOnce(
        jsonResponse({
          secret: { key: 'DB_URL', value: 'postgres://...', version: 1, comment: '', created_at: '', updated_at: '' },
        }),
      );
      mockFetch.mockResolvedValueOnce(
        jsonResponse({
          secret: { key: 'API_KEY', value: 'sk_123', version: 1, comment: '', created_at: '', updated_at: '' },
        }),
      );

      const vault = new ZVault({ autoRefresh: false });
      const secrets = await vault.getAll('staging');
      vault.destroy();

      expect(secrets.size).toBe(2);
      expect(secrets.get('DB_URL')).toBe('postgres://...');
      expect(secrets.get('API_KEY')).toBe('sk_123');
    });
  });

  describe('set()', () => {
    it('sets a secret value', async () => {
      mockFetch.mockResolvedValueOnce(
        jsonResponse({
          secret: {
            key: 'NEW_KEY',
            value: 'new_value',
            version: 1,
            comment: 'test',
            created_at: '2026-01-01T00:00:00Z',
            updated_at: '2026-01-01T00:00:00Z',
          },
        }),
      );

      const vault = new ZVault();
      const entry = await vault.set('NEW_KEY', 'new_value', 'dev', 'test');
      vault.destroy();

      expect(entry.key).toBe('NEW_KEY');
      expect(entry.version).toBe(1);

      const [, opts] = mockFetch.mock.calls[0];
      expect(opts.method).toBe('PUT');
      expect(JSON.parse(opts.body)).toEqual({ value: 'new_value', comment: 'test' });
    });
  });

  describe('healthy()', () => {
    it('returns ok:true when API is reachable', async () => {
      mockFetch.mockResolvedValueOnce(
        jsonResponse({ user_id: 'u-1', email: 'test@test.com' }),
      );

      const vault = new ZVault();
      const health = await vault.healthy();
      vault.destroy();

      expect(health.ok).toBe(true);
      expect(health.latencyMs).toBeGreaterThanOrEqual(0);
    });

    it('returns ok:false when API is unreachable', async () => {
      mockFetch.mockRejectedValueOnce(new Error('network error'));

      const vault = new ZVault({ maxRetries: 0 });
      const health = await vault.healthy();
      vault.destroy();

      expect(health.ok).toBe(false);
    });
  });

  describe('injectIntoEnv()', () => {
    it('injects secrets into process.env', async () => {
      mockFetch.mockResolvedValueOnce(
        jsonResponse({ keys: [{ key: 'INJECTED_VAR', version: 1, comment: '', updated_at: '' }] }),
      );
      mockFetch.mockResolvedValueOnce(
        jsonResponse({
          secret: { key: 'INJECTED_VAR', value: 'injected_value', version: 1, comment: '', created_at: '', updated_at: '' },
        }),
      );

      const vault = new ZVault({ autoRefresh: false });
      const count = await vault.injectIntoEnv('dev');
      vault.destroy();

      expect(count).toBe(1);
      expect(process.env.INJECTED_VAR).toBe('injected_value');

      // Cleanup
      delete process.env.INJECTED_VAR;
    });

    it('does not overwrite existing env vars by default', async () => {
      process.env.EXISTING_VAR = 'original';

      mockFetch.mockResolvedValueOnce(
        jsonResponse({ keys: [{ key: 'EXISTING_VAR', version: 1, comment: '', updated_at: '' }] }),
      );
      mockFetch.mockResolvedValueOnce(
        jsonResponse({
          secret: { key: 'EXISTING_VAR', value: 'new_value', version: 1, comment: '', created_at: '', updated_at: '' },
        }),
      );

      const vault = new ZVault({ autoRefresh: false });
      await vault.injectIntoEnv('dev');
      vault.destroy();

      expect(process.env.EXISTING_VAR).toBe('original');
      delete process.env.EXISTING_VAR;
    });
  });
});

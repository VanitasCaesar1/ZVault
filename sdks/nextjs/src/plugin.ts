/**
 * withZVault — Next.js config wrapper that injects secrets at build time.
 *
 * @example
 * ```js
 * // next.config.js
 * const { withZVault } = require('@zvault/next/plugin');
 *
 * module.exports = withZVault({
 *   env: 'production',
 *   publicKeys: ['NEXT_PUBLIC_STRIPE_KEY'],
 * })({
 *   reactStrictMode: true,
 * });
 * ```
 */

import type { ZVaultNextConfig } from './types';

const DEFAULT_BASE_URL = 'https://api.zvault.cloud';
const DEFAULT_TIMEOUT = 15_000;

interface NextConfig {
  env?: Record<string, string>;
  [key: string]: unknown;
}

/**
 * Wraps your Next.js config to inject ZVault secrets into `process.env`
 * at build time. Only keys listed in `publicKeys` become `NEXT_PUBLIC_*`
 * (browser-safe). Everything else stays server-only.
 */
export function withZVault(zvaultConfig?: ZVaultNextConfig) {
  const token = zvaultConfig?.token ?? process.env.ZVAULT_TOKEN ?? '';
  const orgId = zvaultConfig?.orgId ?? process.env.ZVAULT_ORG_ID ?? '';
  const projectId = zvaultConfig?.projectId ?? process.env.ZVAULT_PROJECT_ID ?? '';
  const envSlug = zvaultConfig?.env ?? process.env.ZVAULT_ENV ?? 'production';
  const baseUrl = (zvaultConfig?.url ?? process.env.ZVAULT_URL ?? DEFAULT_BASE_URL).replace(/\/+$/, '');
  const publicKeys = new Set(zvaultConfig?.publicKeys ?? []);

  return (nextConfig: NextConfig = {}): NextConfig => {
    if (!token || !orgId || !projectId) {
      console.warn(
        '[zvault] Missing ZVAULT_TOKEN, ZVAULT_ORG_ID, or ZVAULT_PROJECT_ID — skipping secret injection.',
      );
      return nextConfig;
    }

    // Use a synchronous-looking approach via top-level await in next.config.mjs
    // or the webpack hook for CJS configs.
    const originalWebpack = nextConfig.webpack as
      | ((config: any, options: any) => any)
      | undefined;

    return {
      ...nextConfig,
      webpack(config: any, options: any) {
        // Only inject during server compilation (not client)
        if (options.isServer && !options.dev) {
          // Secrets are already injected into process.env by the time webpack runs
          // if the user called withZVault in an async next.config.mjs
        }
        return originalWebpack ? originalWebpack(config, options) : config;
      },

      // Async env loading — Next.js 13.4+ supports async config
      async rewrites() {
        // Side-effect: fetch and inject secrets into process.env
        await injectSecrets(baseUrl, token, orgId, projectId, envSlug, publicKeys);

        // Delegate to original rewrites if any
        const original = nextConfig.rewrites as (() => Promise<any>) | undefined;
        if (typeof original === 'function') {
          return original();
        }
        return [];
      },
    };
  };
}

async function injectSecrets(
  baseUrl: string,
  token: string,
  orgId: string,
  projectId: string,
  envSlug: string,
  publicKeys: Set<string>,
): Promise<void> {
  try {
    const keysUrl = `${baseUrl}/v1/cloud/orgs/${orgId}/projects/${projectId}/envs/${envSlug}/secrets`;

    const controller = new AbortController();
    const tid = setTimeout(() => controller.abort(), DEFAULT_TIMEOUT);

    const keysRes = await fetch(keysUrl, {
      headers: {
        Authorization: `Bearer ${token}`,
        'User-Agent': '@zvault/next-plugin/0.1.0',
      },
      signal: controller.signal,
    });

    clearTimeout(tid);

    if (!keysRes.ok) {
      console.warn(`[zvault] Failed to fetch secret keys: HTTP ${keysRes.status}`);
      return;
    }

    const { keys } = (await keysRes.json()) as { keys: Array<{ key: string }> };
    let injected = 0;

    // Fetch values in parallel
    const results = await Promise.allSettled(
      keys.map(async (k) => {
        const url = `${baseUrl}/v1/cloud/orgs/${orgId}/projects/${projectId}/envs/${envSlug}/secrets/${encodeURIComponent(k.key)}`;
        const res = await fetch(url, {
          headers: {
            Authorization: `Bearer ${token}`,
            'User-Agent': '@zvault/next-plugin/0.1.0',
          },
        });
        if (!res.ok) return null;
        const data = (await res.json()) as { secret: { key: string; value: string } };
        return data.secret;
      }),
    );

    for (const result of results) {
      if (result.status !== 'fulfilled' || !result.value) continue;
      const { key, value } = result.value;

      // Inject as server-side env var
      if (process.env[key] === undefined) {
        process.env[key] = value;
        injected++;
      }

      // If listed in publicKeys, also set NEXT_PUBLIC_ variant
      if (publicKeys.has(key) && process.env[`NEXT_PUBLIC_${key}`] === undefined) {
        process.env[`NEXT_PUBLIC_${key}`] = value;
      }
    }

    console.log(`[zvault] Injected ${injected} secrets from "${envSlug}" environment`);
  } catch (err) {
    console.warn(
      `[zvault] Failed to inject secrets: ${err instanceof Error ? err.message : String(err)}`,
    );
  }
}

export type { ZVaultNextConfig };

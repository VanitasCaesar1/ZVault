/**
 * @zvault/next — ZVault SDK for Next.js
 *
 * Build-time and runtime secret injection for Next.js applications.
 *
 * Usage in next.config.js:
 * ```js
 * const { withZVault } = require('@zvault/next/plugin');
 * module.exports = withZVault({
 *   env: 'production',
 * })({
 *   // your next config
 * });
 * ```
 *
 * Usage in server components / API routes:
 * ```ts
 * import { getSecret, getAllSecrets } from '@zvault/next';
 * const dbUrl = await getSecret('DATABASE_URL');
 * ```
 */

export { getSecret, getAllSecrets, getZVaultClient } from './client';
export type { ZVaultNextConfig } from './types';

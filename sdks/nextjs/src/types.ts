/** Configuration for the ZVault Next.js integration. */
export interface ZVaultNextConfig {
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
  /**
   * Keys to expose as NEXT_PUBLIC_ env vars (available in browser).
   * Only listed keys are exposed — everything else stays server-only.
   */
  publicKeys?: string[];
  /** Cache TTL in ms. Default: 300000 (5 min). */
  cacheTtl?: number;
}

export interface SecretEntry {
  key: string;
  value: string;
  version: number;
  comment: string;
  created_at: string;
  updated_at: string;
}

export interface SecretKey {
  key: string;
  version: number;
  comment: string;
  updated_at: string;
}

/** Configuration for the ZVault SDK client. */
export interface ZVaultConfig {
  /** Service token (zvt_...) or cloud auth token. Falls back to ZVAULT_TOKEN env var. */
  token?: string;

  /** ZVault Cloud API base URL. Falls back to ZVAULT_URL env var, then https://api.zvault.cloud. */
  baseUrl?: string;

  /** Organization ID. Falls back to ZVAULT_ORG_ID env var. */
  orgId?: string;

  /** Project ID. Falls back to ZVAULT_PROJECT_ID env var. */
  projectId?: string;

  /** Default environment slug (e.g. "production"). Falls back to ZVAULT_ENV env var. */
  defaultEnv?: string;

  /** Cache TTL in milliseconds. Default: 300_000 (5 minutes). Set to 0 to disable caching. */
  cacheTtl?: number;

  /** Enable automatic background refresh. Default: true. */
  autoRefresh?: boolean;

  /** Request timeout in milliseconds. Default: 10_000 (10 seconds). */
  timeout?: number;

  /** Maximum retry attempts on transient errors (429, 503). Default: 3. */
  maxRetries?: number;

  /** Enable debug logging to stderr. Default: false. Never logs secret values. */
  debug?: boolean;
}

/** A single secret entry returned by the API. */
export interface SecretEntry {
  key: string;
  value: string;
  version: number;
  comment: string;
  created_at: string;
  updated_at: string;
}

/** A secret key (no value) returned by list operations. */
export interface SecretKey {
  key: string;
  version: number;
  comment: string;
  updated_at: string;
}

/** API response for a single secret. */
export interface SecretResponse {
  secret: SecretEntry;
}

/** API response for listing secret keys. */
export interface SecretKeysResponse {
  keys: SecretKey[];
}

/** API error response shape from ZVault Cloud. */
export interface ApiErrorBody {
  error?: {
    code?: number;
    message?: string;
  };
}

/** Health check result. */
export interface HealthStatus {
  ok: boolean;
  latencyMs: number;
  cachedSecrets: number;
  lastRefresh: Date | null;
}

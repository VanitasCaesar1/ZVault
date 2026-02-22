/** Base error class for all ZVault SDK errors. */
export class ZVaultError extends Error {
  constructor(message: string) {
    super(message);
    this.name = 'ZVaultError';
  }
}

/** Thrown when the API returns an HTTP error. */
export class ZVaultApiError extends ZVaultError {
  public readonly statusCode: number;
  public readonly apiMessage: string;

  constructor(statusCode: number, apiMessage: string) {
    super(`ZVault API error ${statusCode}: ${apiMessage}`);
    this.name = 'ZVaultApiError';
    this.statusCode = statusCode;
    this.apiMessage = apiMessage;
  }
}

/** Thrown when required configuration is missing. */
export class ZVaultConfigError extends ZVaultError {
  constructor(message: string) {
    super(message);
    this.name = 'ZVaultConfigError';
  }
}

/** Thrown when a secret is not found. */
export class ZVaultNotFoundError extends ZVaultError {
  public readonly key: string;
  public readonly env: string;

  constructor(key: string, env: string) {
    super(`Secret "${key}" not found in environment "${env}"`);
    this.name = 'ZVaultNotFoundError';
    this.key = key;
    this.env = env;
  }
}

/** Thrown when authentication fails (401/403). */
export class ZVaultAuthError extends ZVaultError {
  constructor(message: string) {
    super(message);
    this.name = 'ZVaultAuthError';
  }
}

/** Thrown when the request times out. */
export class ZVaultTimeoutError extends ZVaultError {
  constructor(timeoutMs: number) {
    super(`Request timed out after ${timeoutMs}ms`);
    this.name = 'ZVaultTimeoutError';
  }
}

# @zvault/sdk

Official Node.js SDK for [ZVault Cloud](https://zvault.cloud) — fetch secrets at runtime from ZVault.

## Install

```bash
npm install @zvault/sdk
```

## Quick Start

```typescript
import { ZVault } from '@zvault/sdk';

const vault = new ZVault({
  token: process.env.ZVAULT_TOKEN,
  orgId: 'my-org-id',
  projectId: 'my-project-id',
});

// Fetch all secrets for an environment
const secrets = await vault.getAll('production');
const dbUrl = secrets.get('DATABASE_URL');

// Or fetch a single secret
const stripeKey = await vault.get('STRIPE_KEY', 'production');

// Inject all secrets into process.env
await vault.injectIntoEnv('production');
```

## Configuration

| Option | Env Var | Default | Description |
|--------|---------|---------|-------------|
| `token` | `ZVAULT_TOKEN` | — | Service token or auth token (required) |
| `baseUrl` | `ZVAULT_URL` | `https://api.zvault.cloud` | API base URL |
| `orgId` | `ZVAULT_ORG_ID` | — | Organization ID |
| `projectId` | `ZVAULT_PROJECT_ID` | — | Project ID |
| `defaultEnv` | `ZVAULT_ENV` | `development` | Default environment |
| `cacheTtl` | — | `300000` (5 min) | Cache TTL in ms |
| `autoRefresh` | — | `true` | Background refresh |
| `timeout` | — | `10000` (10s) | Request timeout in ms |
| `maxRetries` | — | `3` | Retry attempts on 429/5xx |
| `debug` | — | `false` | Debug logging to stderr |

## Features

- **Single-call bootstrap** — `getAll()` fetches all secrets in one flow
- **In-memory cache** — secrets cached in process memory, never written to disk
- **Auto-refresh** — background refresh on configurable TTL
- **Graceful degradation** — serves cached values if API is unreachable
- **Retry with backoff** — exponential backoff on 429, 503
- **Env injection** — `injectIntoEnv()` sets secrets as process env vars
- **Health check** — `healthy()` for readiness probes
- **Zero dependencies** — uses native `fetch` (Node 18+)

## API

### `new ZVault(config?)`

Create a client. Reads from env vars if config options aren't provided.

### `vault.getAll(env?): Promise<Map<string, string>>`

Fetch all secrets for an environment. Cached and auto-refreshed.

### `vault.get(key, env?): Promise<string>`

Fetch a single secret. Cache-first.

### `vault.set(key, value, env?, comment?): Promise<SecretEntry>`

Set a secret (requires write permission).

### `vault.delete(key, env?): Promise<void>`

Delete a secret (requires write permission).

### `vault.listKeys(env?): Promise<SecretKey[]>`

List secret keys (no values).

### `vault.injectIntoEnv(env?, overwrite?): Promise<number>`

Inject secrets into `process.env`. Returns count of injected vars.

### `vault.healthy(): Promise<HealthStatus>`

Check API connectivity and token validity.

### `vault.destroy(): void`

Stop background refresh timers and clear cache. Call on shutdown.

## Error Handling

```typescript
import { ZVaultNotFoundError, ZVaultAuthError } from '@zvault/sdk';

try {
  const secret = await vault.get('MISSING_KEY');
} catch (err) {
  if (err instanceof ZVaultNotFoundError) {
    console.log(`Secret "${err.key}" not found in "${err.env}"`);
  }
  if (err instanceof ZVaultAuthError) {
    console.log('Token expired or invalid');
  }
}
```

## License

MIT

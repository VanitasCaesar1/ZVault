# @zvault/next

ZVault SDK for Next.js — build-time and runtime secret injection.

## Install

```bash
npm install @zvault/next
```

## Setup

Set environment variables (or pass in config):

```bash
ZVAULT_TOKEN=zvt_your_service_token
ZVAULT_ORG_ID=org_xxx
ZVAULT_PROJECT_ID=proj_xxx
ZVAULT_ENV=production
```

## Build-Time Injection (next.config.js)

Injects all secrets into `process.env` at build time:

```js
// next.config.mjs
import { withZVault } from '@zvault/next/plugin';

export default withZVault({
  env: 'production',
  publicKeys: ['STRIPE_PUBLISHABLE_KEY'], // exposed as NEXT_PUBLIC_*
})({
  reactStrictMode: true,
});
```

Only keys listed in `publicKeys` are exposed to the browser. Everything else stays server-only.

## Runtime Usage (Server Components / API Routes)

```ts
import { getSecret, getAllSecrets } from '@zvault/next';

// Single secret
const dbUrl = await getSecret('DATABASE_URL');

// All secrets as object
const secrets = await getAllSecrets({ env: 'staging' });
console.log(secrets.STRIPE_KEY);
```

## Client Instance

```ts
import { getZVaultClient } from '@zvault/next';

const vault = getZVaultClient({ env: 'production' });
const key = await vault.get('API_KEY');
const all = await vault.getAll();
```

## Configuration

| Option | Env Var | Default |
|--------|---------|---------|
| `token` | `ZVAULT_TOKEN` | — (required) |
| `orgId` | `ZVAULT_ORG_ID` | — (required) |
| `projectId` | `ZVAULT_PROJECT_ID` | — (required) |
| `env` | `ZVAULT_ENV` | `production` |
| `url` | `ZVAULT_URL` | `https://api.zvault.cloud` |
| `publicKeys` | — | `[]` |
| `cacheTtl` | — | `300000` (5 min) |

## How It Works

1. **Build time**: `withZVault()` fetches secrets and injects into `process.env` before Next.js compiles
2. **Runtime**: `getSecret()` / `getAllSecrets()` fetch on-demand with in-memory caching
3. **Security**: Secrets never reach the browser unless explicitly listed in `publicKeys`

## License

MIT

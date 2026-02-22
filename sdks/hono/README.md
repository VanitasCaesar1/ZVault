# @zvault/hono

ZVault middleware for [Hono](https://hono.dev) — auto-inject secrets into context.

Works on Node.js, Bun, Deno, Cloudflare Workers, and any runtime Hono supports.

## Install

```bash
npm install @zvault/hono
```

## Quick Start

```ts
import { Hono } from 'hono';
import { zvault } from '@zvault/hono';

const app = new Hono();

// Middleware — attaches secrets to every request
app.use(zvault({ env: 'production' }));

app.get('/', (c) => {
  const dbUrl = c.get('secrets').DATABASE_URL;
  return c.json({ ok: true });
});

export default app;
```

### One-shot injection at startup

```ts
import { inject } from '@zvault/hono';

await inject({ env: 'production' });
// Now process.env.DATABASE_URL is set
```

## Environment Variables

```bash
ZVAULT_TOKEN=zvt_your_service_token
ZVAULT_ORG_ID=org_xxx
ZVAULT_PROJECT_ID=proj_xxx
ZVAULT_ENV=production
```

## Configuration

| Option | Env Var | Default |
|--------|---------|---------|
| `token` | `ZVAULT_TOKEN` | — (required) |
| `orgId` | `ZVAULT_ORG_ID` | — (required) |
| `projectId` | `ZVAULT_PROJECT_ID` | — (required) |
| `env` | `ZVAULT_ENV` | `production` |
| `url` | `ZVAULT_URL` | `https://api.zvault.cloud` |
| `cacheTtl` | — | `300000` (5 min) |

## Graceful Degradation

If the ZVault API is unreachable, the middleware serves stale cached secrets and logs a warning. Your app keeps running.

## License

MIT

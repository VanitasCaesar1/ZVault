# @zvault/cloudflare-workers

ZVault integration for Cloudflare Workers — fetch secrets with optional KV caching.

## Install

```bash
npm install @zvault/cloudflare-workers
```

## Quick Start

```ts
import { ZVault } from '@zvault/cloudflare-workers';

export default {
  async fetch(request, env) {
    const vault = new ZVault(env);
    const dbUrl = await vault.get('DATABASE_URL');
    return new Response('ok');
  },
};
```

## wrangler.toml

```toml
[vars]
ZVAULT_TOKEN = "zvt_your_token"
ZVAULT_ORG_ID = "org_xxx"
ZVAULT_PROJECT_ID = "proj_xxx"
ZVAULT_ENV = "production"

# Optional: KV namespace for caching
[[kv_namespaces]]
binding = "ZVAULT_KV"
id = "your-kv-namespace-id"
```

## License

MIT

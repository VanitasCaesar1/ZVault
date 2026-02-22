# ZVault × Fly.io

Replace `fly secrets` with ZVault Cloud for centralized secret management.

## Setup

### 1. Set ZVault credentials as Fly secrets

```bash
fly secrets set ZVAULT_TOKEN=zvt_your_service_token
fly secrets set ZVAULT_ORG_ID=org_xxx
fly secrets set ZVAULT_PROJECT_ID=proj_xxx
```

### 2. Use Docker entrypoint pattern

```dockerfile
# fly.toml
[build]
  dockerfile = "Dockerfile"

# Dockerfile
RUN curl -fsSL https://zvault.cloud/install.sh | sh
ENTRYPOINT ["zvault", "run", "--env", "production", "--"]
CMD ["node", "server.js"]
```

### 3. Or use the SDK in your app

```typescript
import { ZVault } from '@zvault/sdk';

const vault = new ZVault();
await vault.injectIntoEnv('production');
```

### 4. fly.toml integration

```toml
# fly.toml
[env]
  ZVAULT_ENV = "production"

[processes]
  app = "zvault run --env production -- node server.js"
```

## Migration from fly secrets

```bash
# Export current fly secrets
fly secrets list --json | jq -r '.[] | "\(.Name)=\(.Digest)"'

# Import into ZVault (manual — fly doesn't expose values)
# Re-set each secret in ZVault dashboard or CLI
zvault cloud set DATABASE_URL "postgres://..." --env production
```

## License

MIT

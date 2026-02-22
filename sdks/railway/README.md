# ZVault × Railway

Inject ZVault Cloud secrets into Railway deployments.

## Setup

### 1. Add ZVault service token to Railway

```bash
railway variables set ZVAULT_TOKEN=zvt_your_service_token
railway variables set ZVAULT_ORG_ID=org_xxx
railway variables set ZVAULT_PROJECT_ID=proj_xxx
railway variables set ZVAULT_ENV=production
```

### 2. Use Docker entrypoint pattern

```dockerfile
# Install ZVault CLI
RUN curl -fsSL https://zvault.cloud/install.sh | sh

# Use zvault as entrypoint
ENTRYPOINT ["zvault", "run", "--env", "production", "--"]
CMD ["node", "server.js"]
```

### 3. Or use the Node.js SDK

```typescript
// At the top of your entry file
import { ZVault } from '@zvault/sdk';

const vault = new ZVault();
await vault.injectIntoEnv('production');

// Now process.env has all your secrets
```

### 4. Or use the init script pattern

```dockerfile
COPY zvault-init.sh /app/
RUN chmod +x /app/zvault-init.sh
ENTRYPOINT ["/app/zvault-init.sh"]
CMD ["node", "server.js"]
```

```bash
#!/bin/sh
# zvault-init.sh
eval $(zvault cloud pull --env ${ZVAULT_ENV:-production} --format shell)
exec "$@"
```

## Migration from Railway Variables

```bash
# Export Railway variables to ZVault
zvault migrate railway --project my-app --env production
```

## License

MIT

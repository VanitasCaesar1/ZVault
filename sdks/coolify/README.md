# ZVault + Coolify

Inject secrets from ZVault Cloud into Coolify deployments.

## Setup

### Option 1: Dockerfile Entrypoint (Recommended)

Use ZVault's Docker init pattern:

```dockerfile
FROM node:22-alpine

# Install ZVault CLI
RUN curl -fsSL https://zvault.cloud/install.sh | bash

WORKDIR /app
COPY . .
RUN npm ci --production

# ZVault as entrypoint — injects secrets then exec's your app
ENTRYPOINT ["zvault", "run", "--env", "production", "--"]
CMD ["node", "server.js"]
```

Set `ZVAULT_TOKEN` in Coolify's environment variables UI.

### Option 2: Build-Time Script

Add to your Coolify build command:

```bash
curl -fsSL https://zvault.cloud/install.sh | bash && \
  export PATH="$HOME/.zvault/bin:$PATH" && \
  zvault cloud pull --env production --output .env --format env
```

### Option 3: Runtime SDK

```typescript
import { ZVault } from '@zvault/sdk';

const vault = new ZVault({ token: process.env.ZVAULT_TOKEN });
const secrets = await vault.getAll({ env: 'production' });
```

## Environment Variables

Set these in Coolify's UI:

| Variable | Required | Description |
|----------|----------|-------------|
| `ZVAULT_TOKEN` | Yes | Service token |

# ZVault + Render

Sync secrets from ZVault Cloud to Render environment groups.

## Setup

### Option 1: Build-Time Injection (Recommended)

Add a pre-build script that pulls secrets from ZVault:

```bash
# render-build.sh
#!/usr/bin/env bash
set -euo pipefail

# Install ZVault CLI
curl -fsSL https://zvault.cloud/install.sh | bash
export PATH="$HOME/.zvault/bin:$PATH"

# Pull secrets into .env
zvault cloud pull --env production --output .env --format env

# Source secrets for the build
set -a
source .env
set +a

# Run your build
npm ci
npm run build
```

In `render.yaml`:
```yaml
services:
  - type: web
    name: my-app
    buildCommand: bash render-build.sh
    startCommand: node server.js
    envVars:
      - key: ZVAULT_TOKEN
        sync: false  # Set manually in Render dashboard
```

### Option 2: Runtime SDK

Use the Node.js SDK in your app:

```typescript
import { ZVault } from '@zvault/sdk';

const vault = new ZVault({ token: process.env.ZVAULT_TOKEN });
const secrets = await vault.getAll({ env: 'production' });

// Use secrets
const dbUrl = secrets.DATABASE_URL;
```

### Option 3: Sync Script

Sync ZVault secrets to Render environment groups via Render API:

```bash
#!/usr/bin/env bash
# sync-to-render.sh — Push ZVault secrets to Render env group
RENDER_API_KEY="${RENDER_API_KEY:?Set RENDER_API_KEY}"
ENV_GROUP_ID="${RENDER_ENV_GROUP_ID:?Set RENDER_ENV_GROUP_ID}"

zvault cloud pull --env production --format json | \
  jq -r 'to_entries[] | select(.key != "_meta") | "\(.key)=\(.value)"' | \
  while IFS='=' read -r key value; do
    curl -s -X PUT "https://api.render.com/v1/env-groups/${ENV_GROUP_ID}/env-vars/${key}" \
      -H "Authorization: Bearer ${RENDER_API_KEY}" \
      -H "Content-Type: application/json" \
      -d "{\"value\": \"${value}\"}"
  done
```

## Environment Variables

| Variable | Required | Description |
|----------|----------|-------------|
| `ZVAULT_TOKEN` | Yes | Service token |

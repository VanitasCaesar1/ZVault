# ZVault Docker Init

Use ZVault as your container entrypoint to inject secrets at startup.

## Usage

```dockerfile
# Install zvault CLI
RUN curl -fsSL https://zvault.cloud/install.sh | sh

# Use zvault as entrypoint — it fetches secrets, exports them, then exec's your app
ENTRYPOINT ["zvault", "run", "--env", "production", "--"]
CMD ["node", "server.js"]
```

## How It Works

1. Container starts with `zvault run` as PID 1
2. ZVault fetches secrets from the cloud API using `ZVAULT_TOKEN`
3. Secrets are injected as environment variables
4. ZVault `exec`s your actual command (replacing itself as PID 1)
5. Your app starts with all secrets available in `process.env` / `os.environ` / etc.

## Docker Compose

```yaml
services:
  api:
    build: .
    environment:
      - ZVAULT_TOKEN=${ZVAULT_TOKEN}
      - ZVAULT_ORG_ID=${ZVAULT_ORG_ID}
      - ZVAULT_PROJECT_ID=${ZVAULT_PROJECT_ID}
      - ZVAULT_ENV=production
    entrypoint: ["zvault", "run", "--env", "production", "--"]
    command: ["node", "server.js"]
```

## Environment Variables

| Variable | Required | Default | Description |
|----------|----------|---------|-------------|
| `ZVAULT_TOKEN` | Yes | — | Service token (`zvt_...`) |
| `ZVAULT_ORG_ID` | Yes | — | Organization ID |
| `ZVAULT_PROJECT_ID` | Yes | — | Project ID |
| `ZVAULT_ENV` | No | `production` | Environment slug |
| `ZVAULT_URL` | No | `https://api.zvault.cloud` | API base URL |

## Multi-Stage Build

```dockerfile
FROM node:20-alpine AS builder
WORKDIR /app
COPY package*.json ./
RUN npm ci --production

FROM node:20-alpine
RUN curl -fsSL https://zvault.cloud/install.sh | sh
WORKDIR /app
COPY --from=builder /app/node_modules ./node_modules
COPY . .
ENTRYPOINT ["zvault", "run", "--env", "production", "--"]
CMD ["node", "server.js"]
```

## License

MIT

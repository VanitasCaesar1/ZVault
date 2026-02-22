# ZVault + Drone CI

Inject secrets from ZVault Cloud into Drone CI pipelines.

## .drone.yml

```yaml
kind: pipeline
type: docker
name: default

steps:
  - name: inject-secrets
    image: alpine:3.20
    environment:
      ZVAULT_TOKEN:
        from_secret: zvault_token
    commands:
      - apk add --no-cache curl bash
      - curl -fsSL https://zvault.cloud/install.sh | bash
      - export PATH="$HOME/.zvault/bin:$PATH"
      - zvault cloud pull --env staging --output .env --format env
      - echo "✅ Secrets pulled"

  - name: test
    image: node:22-alpine
    commands:
      - source .env
      - npm ci
      - npm test

  - name: build
    image: node:22-alpine
    commands:
      - source .env
      - npm run build
```

## Setup

1. Add `zvault_token` as a Drone secret:
   ```bash
   drone secret add --repository org/repo --name zvault_token --data zvt_xxx
   ```

2. Reference it in your pipeline via `from_secret`

## Plugin (Docker)

For a cleaner approach, use the ZVault Drone plugin:

```yaml
steps:
  - name: secrets
    image: zvault/drone-plugin:latest
    settings:
      token:
        from_secret: zvault_token
      environment: staging
```

## Environment Variables

| Variable | Required | Description |
|----------|----------|-------------|
| `ZVAULT_TOKEN` | Yes | Service token (from Drone secrets) |
| `ZVAULT_ENV` | No | Environment name (default: `staging`) |

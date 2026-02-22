# ZVault + AWS CodeBuild

Inject secrets from ZVault Cloud into AWS CodeBuild builds.

## Setup

1. Add `ZVAULT_TOKEN` to your CodeBuild project environment variables (mark as "Secret")
2. Use the helper script in your `buildspec.yml`

## buildspec.yml

```yaml
version: 0.2

env:
  variables:
    ZVAULT_ENV: staging

phases:
  install:
    commands:
      - curl -fsSL https://zvault.cloud/install.sh | bash
      - export PATH="$HOME/.zvault/bin:$PATH"

  pre_build:
    commands:
      - source ./buildspec-helper.sh

  build:
    commands:
      - echo "DATABASE_URL is set: ${DATABASE_URL:+yes}"
      - npm test
      - npm run build
```

## Alternative: Direct CLI

```yaml
phases:
  pre_build:
    commands:
      - curl -fsSL https://zvault.cloud/install.sh | bash
      - export PATH="$HOME/.zvault/bin:$PATH"
      - eval "$(zvault cloud pull --env staging --format env)"
```

## Environment Variables

| Variable | Required | Description |
|----------|----------|-------------|
| `ZVAULT_TOKEN` | Yes | Service token (mark as Secret in CodeBuild) |
| `ZVAULT_ENV` | No | Environment name (default: `staging`) |

## Security

- Store `ZVAULT_TOKEN` as a CodeBuild "Secret" environment variable (encrypted at rest)
- Use scoped service tokens (one per project + environment)
- Secrets are fetched at build time, never stored in build artifacts

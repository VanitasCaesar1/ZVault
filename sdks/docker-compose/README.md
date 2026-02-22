# ZVault Docker Compose Integration

Init container pattern for injecting ZVault secrets into Docker Compose services.

## Quick Start

```yaml
# docker-compose.yml
services:
  app:
    build: .
    depends_on:
      zvault-init:
        condition: service_completed_successfully
    volumes:
      - zvault-secrets:/run/secrets:ro
    env_file:
      - /run/secrets/zvault.env

  zvault-init:
    extends:
      file: docker-compose.zvault.yml
      service: zvault-init

volumes:
  zvault-secrets:
```

## Environment Variables

```bash
# .env
ZVAULT_TOKEN=zvt_your_token
ZVAULT_ORG_ID=org_xxx
ZVAULT_PROJECT_ID=proj_xxx
ZVAULT_ENV=production
```

## How It Works

1. `zvault-init` runs first, fetches all secrets from ZVault Cloud
2. Writes them to `/run/secrets/zvault.env` in a shared volume
3. Your app service starts after init completes, reads the env file
4. Secrets are available as environment variables in your app

## License

MIT

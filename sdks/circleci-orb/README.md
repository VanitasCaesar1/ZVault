# ZVault CircleCI Orb

Inject secrets from ZVault Cloud into your CircleCI pipelines.

## Usage

```yaml
# .circleci/config.yml
version: 2.1

orbs:
  zvault: zvault/secrets@1.0

jobs:
  test:
    docker:
      - image: cimg/node:20.0
    steps:
      - checkout
      - zvault/inject-secrets:
          env: staging
      - run: npm test  # All secrets available as env vars

workflows:
  build-and-test:
    jobs:
      - test
```

## Configuration

Set these as CircleCI environment variables (Project Settings → Environment Variables):

| Variable | Required | Description |
|----------|----------|-------------|
| `ZVAULT_TOKEN` | Yes | Service token (`zvt_...`) |
| `ZVAULT_ORG_ID` | Yes | Organization ID |
| `ZVAULT_PROJECT_ID` | Yes | Project ID |

### Orb Parameters

| Parameter | Default | Description |
|-----------|---------|-------------|
| `env` | `production` | Environment slug |
| `url` | `https://api.zvault.cloud` | API base URL |
| `keys` | — | Comma-separated keys (empty = all) |

## Security

- Store `ZVAULT_TOKEN` as a CircleCI environment variable (never in config)
- Use CircleCI contexts to share tokens across projects
- Restrict contexts to specific branches for production secrets

## License

MIT

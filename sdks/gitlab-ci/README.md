# ZVault GitLab CI Component

Inject secrets from ZVault Cloud into your GitLab CI/CD pipelines.

## Usage

```yaml
# .gitlab-ci.yml
include:
  - component: gitlab.com/zvault/gitlab-ci/inject-secrets@v1

stages:
  - test

test:
  stage: test
  variables:
    ZVAULT_TOKEN: $ZVAULT_TOKEN
    ZVAULT_ORG_ID: $ZVAULT_ORG_ID
    ZVAULT_PROJECT_ID: $ZVAULT_PROJECT_ID
    ZVAULT_ENV: staging
  before_script:
    - source <(zvault-inject)
  script:
    - npm test
```

## Direct Script Usage

If you prefer not to use the CI component, use the standalone script:

```yaml
test:
  image: node:20-alpine
  variables:
    ZVAULT_TOKEN: $ZVAULT_TOKEN
    ZVAULT_ORG_ID: $ZVAULT_ORG_ID
    ZVAULT_PROJECT_ID: $ZVAULT_PROJECT_ID
    ZVAULT_ENV: production
  before_script:
    - apk add --no-cache curl
    - curl -fsSL https://zvault.cloud/install.sh | sh
    - eval "$(zvault cloud secrets --env $ZVAULT_ENV --format export)"
  script:
    - echo "All ZVault secrets are now available as env vars"
    - npm test
```

## Configuration

| Variable | Required | Default | Description |
|----------|----------|---------|-------------|
| `ZVAULT_TOKEN` | Yes | — | Service token (`zvt_...`) |
| `ZVAULT_ORG_ID` | Yes | — | Organization ID |
| `ZVAULT_PROJECT_ID` | Yes | — | Project ID |
| `ZVAULT_ENV` | No | `production` | Environment slug |
| `ZVAULT_URL` | No | `https://api.zvault.cloud` | API base URL |
| `ZVAULT_KEYS` | No | — | Comma-separated keys to fetch (empty = all) |
| `ZVAULT_MASK` | No | `true` | Mask values in job logs |

## How It Works

1. The script fetches secrets from ZVault Cloud API using your service token
2. Each secret is exported as an environment variable
3. Values are masked in GitLab CI job logs (when `ZVAULT_MASK=true`)
4. Your application reads secrets from environment variables as usual

## Security

- Store `ZVAULT_TOKEN` as a [GitLab CI/CD variable](https://docs.gitlab.com/ee/ci/variables/) (masked + protected)
- Use protected branches to limit which pipelines can access production secrets
- Service tokens can be scoped to specific environments

## License

MIT

# ZVault Bitbucket Pipe

Inject ZVault secrets into your Bitbucket Pipeline builds.

## Usage

```yaml
# bitbucket-pipelines.yml
pipelines:
  default:
    - step:
        name: Build
        script:
          - pipe: zvault/inject-secrets:0.1.0
            variables:
              ZVAULT_TOKEN: $ZVAULT_TOKEN
              ZVAULT_ORG_ID: "org_xxx"
              ZVAULT_PROJECT_ID: "proj_xxx"
              ZVAULT_ENV: "production"
          - source /tmp/zvault_env.sh
          - npm test
```

## Variables

| Variable | Required | Default | Description |
|----------|----------|---------|-------------|
| `ZVAULT_TOKEN` | Yes | — | Service token |
| `ZVAULT_ORG_ID` | Yes | — | Organization ID |
| `ZVAULT_PROJECT_ID` | Yes | — | Project ID |
| `ZVAULT_ENV` | No | `production` | Environment |
| `ZVAULT_MASK` | No | `true` | Mask values in logs |

## License

MIT

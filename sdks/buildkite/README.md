# ZVault Buildkite Plugin

Inject ZVault secrets into Buildkite build steps.

## Usage

```yaml
# .buildkite/pipeline.yml
steps:
  - label: ":rocket: Deploy"
    plugins:
      - zvault/secrets#v0.1.0:
          org-id: "org_xxx"
          project-id: "proj_xxx"
          env: "production"
    command: npm run deploy
```

Set `ZVAULT_TOKEN` in your Buildkite agent environment or pipeline settings.

## Configuration

| Option | Default | Description |
|--------|---------|-------------|
| `org-id` | — (required) | Organization ID |
| `project-id` | — (required) | Project ID |
| `env` | `production` | Environment slug |
| `token-env` | `ZVAULT_TOKEN` | Env var name holding the token |
| `url` | `https://api.zvault.cloud` | API URL |
| `mask` | `true` | Mask values in logs |

## License

MIT

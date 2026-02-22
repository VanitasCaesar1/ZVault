# ZVault Setup Action

Inject secrets from [ZVault Cloud](https://zvault.cloud) into your GitHub Actions workflows.

## Usage

```yaml
steps:
  - uses: zvault/setup-action@v1
    with:
      token: ${{ secrets.ZVAULT_TOKEN }}
      org-id: ${{ vars.ZVAULT_ORG_ID }}
      project-id: ${{ vars.ZVAULT_PROJECT_ID }}
      env: staging

  - run: npm test
    # All secrets are now available as environment variables
```

### Fetch specific keys only

```yaml
  - uses: zvault/setup-action@v1
    with:
      token: ${{ secrets.ZVAULT_TOKEN }}
      org-id: ${{ vars.ZVAULT_ORG_ID }}
      project-id: ${{ vars.ZVAULT_PROJECT_ID }}
      env: production
      keys: DATABASE_URL,STRIPE_KEY,REDIS_URL
```

## Inputs

| Input | Required | Default | Description |
|-------|----------|---------|-------------|
| `token` | Yes | — | ZVault service token (`zvt_...`) |
| `org-id` | Yes | — | Organization ID |
| `project-id` | Yes | — | Project ID |
| `env` | No | `production` | Environment slug |
| `url` | No | `https://api.zvault.cloud` | API base URL |
| `keys` | No | — | Comma-separated keys to fetch (all if empty) |
| `export-env` | No | `true` | Export as env vars |
| `mask` | No | `true` | Mask values in logs |

## Outputs

| Output | Description |
|--------|-------------|
| `count` | Number of secrets injected |

## Security

- Secret values are automatically masked in workflow logs
- Use GitHub's encrypted secrets for the `ZVAULT_TOKEN`
- Service tokens should be scoped to the specific project + environment
- Create read-only tokens for CI (no write permission needed)

## License

MIT

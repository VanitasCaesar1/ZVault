# ZVault for VS Code

Inline secret peek, autocomplete, and management for ZVault.

## Features

- **Secret Autocomplete** — type `process.env.` and get suggestions from your ZVault project
- **Hover Peek** — hover over `DATABASE_URL` to see metadata (version, description, last updated)
- **Peek Value** — right-click or use command palette to peek at masked secret values
- **Environment Switcher** — click status bar to switch between dev/staging/prod
- **Quick Copy** — list all secrets and copy keys to clipboard

## Setup

1. Install the extension
2. Configure in VS Code settings or `.env`:

```json
{
  "zvault.token": "zvt_your_service_token",
  "zvault.orgId": "org_xxx",
  "zvault.projectId": "proj_xxx",
  "zvault.env": "development"
}
```

Or set environment variables: `ZVAULT_TOKEN`, `ZVAULT_ORG_ID`, `ZVAULT_PROJECT_ID`, `ZVAULT_ENV`.

## Commands

| Command | Description |
|---------|-------------|
| `ZVault: List Secrets` | Browse and copy secret keys |
| `ZVault: Peek Secret Value` | View masked value of secret under cursor |
| `ZVault: Refresh Secret Cache` | Re-fetch secret list from API |
| `ZVault: Switch Environment` | Change active environment |

## Security

- Secret values are never stored on disk
- Values are masked by default (first 4 + last 4 chars shown)
- Hover shows metadata only (not values)
- All API calls use your service token

## License

MIT

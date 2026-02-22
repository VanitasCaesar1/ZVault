# ZVault + Slack / Discord / Teams

Real-time notifications for secret changes, access events, and security alerts.

## Slack

### Setup

1. Create a Slack app or use an incoming webhook
2. Configure in ZVault Cloud: Settings → Integrations → Slack

### Webhook URL

```
https://hooks.slack.com/services/TXXXXXXXXX/BXXXXXXXXX/your-webhook-token-here
```

### Events

| Event | Channel | Description |
|-------|---------|-------------|
| `secret.created` | #secrets | New secret added |
| `secret.updated` | #secrets | Secret value changed |
| `secret.deleted` | #secrets | Secret removed |
| `secret.rotated` | #secrets | Secret auto-rotated |
| `token.created` | #security | New service token |
| `token.revoked` | #security | Token revoked |
| `auth.failure` | #security | Failed authentication |
| `access.anomaly` | #security | Unusual access pattern |

### Message Format

```
🔐 ZVault: Secret Updated
Project: my-saas
Environment: production
Key: DATABASE_URL
By: john@company.com
Time: 2026-02-21 14:30 UTC
```

## Discord

### Setup

1. Create a Discord webhook in your channel settings
2. Configure in ZVault Cloud: Settings → Integrations → Discord

### Webhook URL

```
https://discord.com/api/webhooks/000000000000000000/XXXXXXXXXXXXXXXXXXXX
```

## Microsoft Teams

### Setup

1. Create an Incoming Webhook connector in your Teams channel
2. Configure in ZVault Cloud: Settings → Integrations → Teams

### Webhook URL

```
https://outlook.office.com/webhook/xxx/IncomingWebhook/yyy/zzz
```

## Configuration

```json
{
  "notifications": {
    "slack": {
      "webhook_url": "https://hooks.slack.com/services/...",
      "channel": "#secrets",
      "events": ["secret.updated", "secret.rotated", "auth.failure"]
    },
    "discord": {
      "webhook_url": "https://discord.com/api/webhooks/...",
      "events": ["secret.updated", "auth.failure"]
    },
    "teams": {
      "webhook_url": "https://outlook.office.com/webhook/...",
      "events": ["auth.failure", "access.anomaly"]
    }
  }
}
```

## Per-Project Routing

Route different projects to different channels:

```json
{
  "routing": {
    "my-saas/production": { "channel": "#prod-alerts" },
    "my-saas/staging": { "channel": "#staging" },
    "*": { "channel": "#secrets-general" }
  }
}
```

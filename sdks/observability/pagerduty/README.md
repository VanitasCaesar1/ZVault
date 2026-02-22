# ZVault + PagerDuty

Alert on ZVault security events via PagerDuty.

## Setup

1. Create a PagerDuty service for ZVault alerts
2. Get the Integration Key (Events API v2)
3. Configure in ZVault Cloud dashboard: Settings → Integrations → PagerDuty

## Alert Types

| Event | Severity | Description |
|-------|----------|-------------|
| Unauthorized Access | Critical | Failed auth attempts exceeding threshold |
| Rotation Failure | High | Secret rotation failed |
| Token Expiry | Warning | Service token expiring within 7 days |
| Unusual Access Pattern | Warning | Secret accessed from new IP/region |
| Quota Exceeded | Low | API rate limit exceeded |

## Webhook Configuration

```json
{
  "integration": "pagerduty",
  "routing_key": "your-pagerduty-integration-key",
  "events": [
    "auth.failure.threshold",
    "rotation.failed",
    "token.expiring",
    "access.anomaly"
  ],
  "severity_map": {
    "auth.failure.threshold": "critical",
    "rotation.failed": "error",
    "token.expiring": "warning",
    "access.anomaly": "warning"
  }
}
```

## Manual Setup via Webhooks

If you prefer manual webhook configuration:

```bash
# ZVault fires webhooks to your endpoint
# Your endpoint forwards to PagerDuty Events API v2

POST https://events.pagerduty.com/v2/enqueue
{
  "routing_key": "your-key",
  "event_action": "trigger",
  "payload": {
    "summary": "ZVault: Unauthorized access attempt on production secrets",
    "source": "zvault-cloud",
    "severity": "critical"
  }
}
```

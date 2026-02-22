# ZVault + OpsGenie

Create incidents in OpsGenie for ZVault security events.

## Setup

1. Create an OpsGenie API integration
2. Get the API key
3. Configure in ZVault Cloud: Settings → Integrations → OpsGenie

## Alert Types

| Event | Priority | Description |
|-------|----------|-------------|
| Unauthorized Access Spike | P1 | >10 failed auth attempts in 1 min |
| Rotation Failure | P2 | Secret rotation failed |
| Service Degradation | P2 | API error rate >5% |
| Token Expiry | P3 | Service token expiring soon |

## Configuration

```json
{
  "integration": "opsgenie",
  "api_key": "your-opsgenie-api-key",
  "events": [
    "auth.failure.threshold",
    "rotation.failed",
    "api.error_rate_high",
    "token.expiring"
  ]
}
```

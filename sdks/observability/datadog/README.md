# ZVault + Datadog

Send ZVault Cloud metrics and events to Datadog.

## Setup

### Option 1: Datadog Integration (Recommended)

Install the ZVault integration from the Datadog Integrations page:

1. Go to Integrations → Search "ZVault"
2. Enter your ZVault service token
3. Metrics and events flow automatically

### Option 2: Custom Metrics via DogStatsD

Use the ZVault SDK's built-in metrics export:

```typescript
import { ZVault } from '@zvault/sdk';

const vault = new ZVault({
  token: process.env.ZVAULT_TOKEN,
  metrics: {
    enabled: true,
    exporter: 'dogstatsd',
    host: 'localhost',
    port: 8125,
    prefix: 'zvault.',
  },
});
```

### Metrics Sent

| Metric | Type | Description |
|--------|------|-------------|
| `zvault.secret.fetch.count` | Count | Secret fetch operations |
| `zvault.secret.fetch.latency` | Histogram | Fetch latency (ms) |
| `zvault.cache.hit` | Count | Cache hits |
| `zvault.cache.miss` | Count | Cache misses |
| `zvault.auth.failure` | Count | Auth failures |

### Events

ZVault sends Datadog events for:
- Secret rotation completed
- Service token created/revoked
- Unauthorized access attempts
- Rotation failures

## Dashboard

Import the pre-built dashboard:

```bash
curl -X POST "https://api.datadoghq.com/api/v1/dashboard" \
  -H "DD-API-KEY: $DD_API_KEY" \
  -H "DD-APPLICATION-KEY: $DD_APP_KEY" \
  -d @zvault-datadog-dashboard.json
```

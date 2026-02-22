# ZVault Prometheus Exporter

Expose ZVault Cloud metrics for Prometheus scraping.

## Metrics Endpoint

The ZVault Cloud API exposes a `/metrics` endpoint (Pro+ plans):

```
https://api.zvault.cloud/v1/metrics
```

### Available Metrics

| Metric | Type | Description |
|--------|------|-------------|
| `zvault_secret_reads_total` | Counter | Total secret read operations |
| `zvault_secret_writes_total` | Counter | Total secret write operations |
| `zvault_secret_deletes_total` | Counter | Total secret delete operations |
| `zvault_api_requests_total` | Counter | Total API requests by status code |
| `zvault_api_request_duration_seconds` | Histogram | API request latency |
| `zvault_cache_hits_total` | Counter | SDK cache hit count |
| `zvault_cache_misses_total` | Counter | SDK cache miss count |
| `zvault_token_auth_total` | Counter | Token authentication attempts |
| `zvault_token_auth_failures_total` | Counter | Failed authentication attempts |
| `zvault_secrets_count` | Gauge | Total secrets per project/env |
| `zvault_service_tokens_count` | Gauge | Active service tokens |
| `zvault_rotation_due_count` | Gauge | Secrets due for rotation |

### Labels

All metrics include:
- `org` — Organization slug
- `project` — Project name
- `environment` — Environment name

## Prometheus Configuration

```yaml
# prometheus.yml
scrape_configs:
  - job_name: zvault
    scheme: https
    bearer_token: "zvt_xxx"  # Service token with metrics scope
    static_configs:
      - targets: ["api.zvault.cloud"]
    metrics_path: /v1/metrics
    scrape_interval: 30s
```

## Self-Hosted Exporter

For self-hosted ZVault instances:

```yaml
# docker-compose.yml
services:
  zvault-exporter:
    image: zvault/prometheus-exporter:latest
    environment:
      ZVAULT_URL: http://zvault:8200
      ZVAULT_TOKEN: hvs.xxx
    ports:
      - "9090:9090"
```

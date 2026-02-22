# ZVault Grafana Dashboard

Pre-built Grafana dashboard for monitoring ZVault Cloud.

## Installation

Import `zvault-dashboard.json` into Grafana:

1. Go to Dashboards → Import
2. Upload `zvault-dashboard.json` or paste the JSON
3. Select your Prometheus data source
4. Click Import

## Panels

### Overview Row
- **Total Secrets**: Gauge showing total secrets across all projects
- **API Requests/min**: Rate of API requests
- **Error Rate**: Percentage of 4xx/5xx responses
- **P99 Latency**: 99th percentile API latency

### Access Patterns Row
- **Secret Reads Over Time**: Time series of read operations by project
- **Secret Writes Over Time**: Time series of write operations
- **Top Accessed Secrets**: Table of most-read secret keys
- **Auth Failures**: Time series of failed authentication attempts

### SDK Performance Row
- **Cache Hit Rate**: Percentage of requests served from SDK cache
- **SDK Fetch Latency**: Histogram of SDK-to-cloud fetch times
- **Active Service Tokens**: Gauge of active tokens per project

### Rotation Row
- **Secrets Due for Rotation**: Gauge of overdue rotations
- **Rotation History**: Time series of completed rotations
- **Rotation Failures**: Alert panel for failed rotations

## Alerts

The dashboard includes pre-configured alert rules:

| Alert | Condition | Severity |
|-------|-----------|----------|
| High Error Rate | >5% 5xx responses for 5min | Critical |
| Auth Failures Spike | >10 failures/min | Warning |
| Rotation Overdue | Secret past rotation deadline | Warning |
| API Latency High | P99 > 2s for 5min | Warning |

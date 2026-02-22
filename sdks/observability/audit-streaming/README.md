# ZVault Audit Log Streaming

Stream ZVault audit events to external logging and SIEM platforms.

## Supported Destinations

| Destination | Format | Protocol |
|-------------|--------|----------|
| Amazon S3 | JSON | S3 API |
| CloudWatch Logs | JSON | CloudWatch API |
| Elasticsearch | JSON | Bulk API |
| Splunk | JSON/CEF | HEC (HTTP Event Collector) |
| Datadog Logs | JSON | Datadog Logs API |
| Sumo Logic | JSON | HTTP Source |
| Azure Sentinel | CEF | Log Analytics API |
| IBM QRadar | LEEF | Syslog |
| Generic Webhook | JSON | HTTP POST |

## Configuration

### S3

```json
{
  "audit_stream": {
    "type": "s3",
    "bucket": "my-audit-logs",
    "prefix": "zvault/",
    "region": "us-east-1",
    "format": "json",
    "batch_size": 100,
    "flush_interval": "60s"
  }
}
```

### Splunk HEC

```json
{
  "audit_stream": {
    "type": "splunk",
    "hec_url": "https://splunk.example.com:8088",
    "hec_token": "your-hec-token",
    "index": "zvault",
    "source": "zvault-cloud",
    "format": "json"
  }
}
```

### Elasticsearch

```json
{
  "audit_stream": {
    "type": "elasticsearch",
    "url": "https://es.example.com:9200",
    "index": "zvault-audit",
    "api_key": "your-api-key",
    "format": "json"
  }
}
```

## Audit Event Schema

```json
{
  "timestamp": "2026-02-21T14:30:00Z",
  "event_type": "secret.read",
  "org_id": "org_xxx",
  "project": "my-saas",
  "environment": "production",
  "actor": {
    "type": "service_token",
    "id": "tok_xxx",
    "name": "railway-deploy"
  },
  "resource": {
    "type": "secret",
    "key": "DATABASE_URL"
  },
  "source_ip": "203.0.113.42",
  "user_agent": "zvault-sdk-node/0.1.0",
  "result": "success"
}
```

## SIEM Integration (CEF/LEEF)

For SIEM platforms that require CEF or LEEF format:

### CEF (Common Event Format)

```
CEF:0|ZVault|Cloud|1.0|secret.read|Secret Read|3|src=203.0.113.42 duser=tok_xxx cs1=my-saas cs2=production cs3=DATABASE_URL
```

### LEEF (Log Event Extended Format)

```
LEEF:2.0|ZVault|Cloud|1.0|secret.read|src=203.0.113.42	usrName=tok_xxx	project=my-saas	env=production	key=DATABASE_URL
```

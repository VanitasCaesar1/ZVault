# ZVault Kubernetes Operator

Sync secrets from ZVault Cloud to Kubernetes Secrets automatically.

## Quick Start

```bash
# Install via Helm
helm repo add zvault https://charts.zvault.cloud
helm install zvault-operator zvault/operator \
  --set token=zvt_your_service_token \
  --set orgId=your_org_id
```

## Custom Resource

```yaml
apiVersion: zvault.cloud/v1alpha1
kind: VaultSecret
metadata:
  name: app-secrets
  namespace: default
spec:
  project: my-saas
  environment: production
  target:
    name: app-secrets
    type: Opaque
  refreshInterval: 5m
```

This creates a Kubernetes Secret named `app-secrets` with all secrets from the
`production` environment of the `my-saas` project, refreshed every 5 minutes.

## Selective Sync

```yaml
apiVersion: zvault.cloud/v1alpha1
kind: VaultSecret
metadata:
  name: db-secrets
spec:
  project: my-saas
  environment: production
  keys:
    - DATABASE_URL
    - REDIS_URL
  target:
    name: db-secrets
    type: Opaque
  refreshInterval: 1m
```

## TLS Secrets

```yaml
apiVersion: zvault.cloud/v1alpha1
kind: VaultSecret
metadata:
  name: tls-cert
spec:
  project: my-saas
  environment: production
  target:
    name: tls-cert
    type: kubernetes.io/tls
  keyMapping:
    TLS_CERT: tls.crt
    TLS_KEY: tls.key
  refreshInterval: 1h
```

## Configuration

### Helm Values

| Value | Default | Description |
|-------|---------|-------------|
| `token` | — | ZVault service token (required) |
| `orgId` | — | Organization ID (required) |
| `apiUrl` | `https://api.zvault.cloud` | API base URL |
| `replicas` | 1 | Operator replicas |
| `resources.requests.memory` | `64Mi` | Memory request |
| `resources.requests.cpu` | `50m` | CPU request |
| `resources.limits.memory` | `128Mi` | Memory limit |

## License

MIT

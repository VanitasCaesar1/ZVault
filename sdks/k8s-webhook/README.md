# ZVault Kubernetes Mutating Webhook

Auto-inject ZVault secrets into pods via annotations — no code changes needed.

## How It Works

1. Annotate your pods with `zvault.cloud/inject: "true"`
2. The webhook intercepts pod creation
3. Adds an init container that fetches secrets from ZVault Cloud
4. Secrets are written to a shared volume as env files
5. Your app container sources the env file at startup

## Installation

```bash
helm repo add zvault https://charts.zvault.cloud
helm install zvault-webhook zvault/mutating-webhook \
  --namespace zvault-system \
  --create-namespace
```

## Usage

### Annotate Your Deployment

```yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: my-app
spec:
  template:
    metadata:
      annotations:
        zvault.cloud/inject: "true"
        zvault.cloud/project: "my-saas"
        zvault.cloud/environment: "production"
        zvault.cloud/token-secret: "zvault-token"  # K8s Secret name
    spec:
      containers:
        - name: app
          image: my-app:latest
          # Secrets automatically injected as env vars
```

### Create Token Secret

```bash
kubectl create secret generic zvault-token \
  --from-literal=token=zvt_xxx
```

## What the Webhook Injects

The webhook mutates the pod spec to add:

1. **Init container**: Fetches secrets and writes `.env` file
2. **Shared volume**: tmpfs volume for the `.env` file
3. **Env source**: `envFrom` referencing the generated ConfigMap

## Annotations

| Annotation | Required | Default | Description |
|------------|----------|---------|-------------|
| `zvault.cloud/inject` | Yes | — | Enable injection (`"true"`) |
| `zvault.cloud/project` | Yes | — | ZVault project name |
| `zvault.cloud/environment` | No | `production` | Environment |
| `zvault.cloud/token-secret` | No | `zvault-token` | K8s Secret with token |
| `zvault.cloud/refresh` | No | `"false"` | Enable sidecar refresh |

## Sidecar Mode

For long-running pods that need secret refresh:

```yaml
annotations:
  zvault.cloud/inject: "true"
  zvault.cloud/refresh: "true"
  zvault.cloud/refresh-interval: "5m"
```

This adds a sidecar container that periodically refreshes secrets.

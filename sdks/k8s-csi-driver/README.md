# ZVault Kubernetes CSI Driver

Mount ZVault secrets as files in Kubernetes pods using the Container Storage Interface.

## Overview

The CSI driver mounts secrets as a tmpfs volume in your pods. Secrets appear as files — one file per secret key. This is an alternative to environment variables for apps that read config from files.

## Installation

```bash
helm repo add zvault https://charts.zvault.cloud
helm install zvault-csi zvault/csi-driver \
  --namespace zvault-system \
  --create-namespace \
  --set zvault.token=zvt_xxx
```

## Usage

### SecretProviderClass

```yaml
apiVersion: secrets-store.csi.x-k8s.io/v1
kind: SecretProviderClass
metadata:
  name: app-secrets
spec:
  provider: zvault
  parameters:
    project: my-saas
    environment: production
    # Optional: only mount specific keys
    keys: |
      - key: DATABASE_URL
        path: db-url
      - key: STRIPE_KEY
        path: stripe-key
```

### Pod Spec

```yaml
apiVersion: v1
kind: Pod
metadata:
  name: my-app
spec:
  containers:
    - name: app
      image: my-app:latest
      volumeMounts:
        - name: secrets
          mountPath: /mnt/secrets
          readOnly: true
  volumes:
    - name: secrets
      csi:
        driver: secrets-store.csi.k8s.io
        readOnly: true
        volumeAttributes:
          secretProviderClass: app-secrets
```

### Reading Secrets

```bash
# Inside the pod
cat /mnt/secrets/db-url        # → postgres://...
cat /mnt/secrets/stripe-key    # → sk_live_...
```

## Sync to K8s Secrets

Optionally sync mounted secrets to native Kubernetes Secrets:

```yaml
apiVersion: secrets-store.csi.x-k8s.io/v1
kind: SecretProviderClass
metadata:
  name: app-secrets
spec:
  provider: zvault
  parameters:
    project: my-saas
    environment: production
  secretObjects:
    - secretName: app-k8s-secrets
      type: Opaque
      data:
        - objectName: DATABASE_URL
          key: db-url
```

## Auto-Rotation

The CSI driver polls ZVault Cloud for changes:

```yaml
# Helm values
rotation:
  enabled: true
  interval: 2m  # Check every 2 minutes
```

## Requirements

- Kubernetes 1.24+
- Secrets Store CSI Driver v1.3+

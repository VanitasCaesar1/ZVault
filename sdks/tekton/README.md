# ZVault + Tekton

Inject secrets from ZVault Cloud into Tekton pipelines.

## Tekton Task

```yaml
apiVersion: tekton.dev/v1
kind: Task
metadata:
  name: zvault-inject-secrets
  labels:
    app.kubernetes.io/version: "0.1"
spec:
  description: >-
    Fetches secrets from ZVault Cloud and writes them to a shared workspace
    as a .env file for downstream tasks.
  params:
    - name: environment
      type: string
      default: staging
      description: ZVault environment to pull secrets from
    - name: format
      type: string
      default: env
      description: Output format (env, json, yaml)
  workspaces:
    - name: secrets
      description: Workspace to write secrets file to
  steps:
    - name: fetch-secrets
      image: alpine:3.20
      env:
        - name: ZVAULT_TOKEN
          valueFrom:
            secretKeyRef:
              name: zvault-credentials
              key: token
      script: |
        #!/usr/bin/env sh
        set -eu
        apk add --no-cache curl bash >/dev/null 2>&1
        curl -fsSL https://zvault.cloud/install.sh | bash
        export PATH="$HOME/.zvault/bin:$PATH"
        zvault cloud pull \
          --env "$(params.environment)" \
          --format "$(params.format)" \
          --output "$(workspaces.secrets.path)/.env"
        echo "✅ Secrets written to workspace"
```

## Pipeline Usage

```yaml
apiVersion: tekton.dev/v1
kind: Pipeline
metadata:
  name: build-and-deploy
spec:
  workspaces:
    - name: shared-secrets
  tasks:
    - name: fetch-secrets
      taskRef:
        name: zvault-inject-secrets
      params:
        - name: environment
          value: production
      workspaces:
        - name: secrets
          workspace: shared-secrets

    - name: build
      runAfter: [fetch-secrets]
      taskRef:
        name: build-app
      workspaces:
        - name: secrets
          workspace: shared-secrets
```

## Setup

1. Create a Kubernetes secret with your ZVault token:
   ```bash
   kubectl create secret generic zvault-credentials \
     --from-literal=token=zvt_xxx
   ```

2. Apply the Task:
   ```bash
   kubectl apply -f zvault-task.yaml
   ```

## Environment Variables

| Variable | Required | Description |
|----------|----------|-------------|
| `ZVAULT_TOKEN` | Yes | Service token (from K8s Secret) |

# ZVault + Google Cloud Run

Inject secrets from ZVault Cloud into Cloud Run services.

## Option 1: Entrypoint Wrapper (Recommended)

```dockerfile
FROM node:22-alpine

# Install ZVault CLI
RUN curl -fsSL https://zvault.cloud/install.sh | bash

WORKDIR /app
COPY . .
RUN npm ci --production

# ZVault as entrypoint
ENTRYPOINT ["zvault", "run", "--env", "production", "--"]
CMD ["node", "server.js"]
```

Deploy:
```bash
gcloud run deploy my-service \
  --image gcr.io/my-project/my-app \
  --set-env-vars ZVAULT_TOKEN=zvt_xxx
```

## Option 2: Runtime SDK

```typescript
import { ZVault } from '@zvault/sdk';

const vault = new ZVault({ token: process.env.ZVAULT_TOKEN });
const secrets = await vault.getAll({ env: 'production' });

// Use secrets
const app = express();
app.listen(process.env.PORT || 8080);
```

## Option 3: Sidecar (Cloud Run v2)

```yaml
# service.yaml
apiVersion: serving.knative.dev/v1
kind: Service
metadata:
  name: my-app
spec:
  template:
    spec:
      containers:
        - image: gcr.io/my-project/my-app
          env:
            - name: ZVAULT_TOKEN
              valueFrom:
                secretKeyRef:
                  name: zvault-token
                  key: latest
```

## Store Token in Secret Manager

```bash
echo -n "zvt_xxx" | gcloud secrets create zvault-token --data-file=-
gcloud run services update my-service \
  --set-secrets=ZVAULT_TOKEN=zvault-token:latest
```

## Environment Variables

| Variable | Description |
|----------|-------------|
| `ZVAULT_TOKEN` | Service token (from GCP Secret Manager) |

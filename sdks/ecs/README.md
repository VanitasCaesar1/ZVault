# ZVault + AWS ECS

Inject secrets from ZVault Cloud into ECS tasks using the init container pattern.

## Task Definition

```json
{
  "family": "my-app",
  "containerDefinitions": [
    {
      "name": "zvault-init",
      "image": "zvault/init:latest",
      "essential": false,
      "environment": [
        { "name": "ZVAULT_ENV", "value": "production" }
      ],
      "secrets": [
        {
          "name": "ZVAULT_TOKEN",
          "valueFrom": "arn:aws:secretsmanager:us-east-1:123456:secret:zvault-token"
        }
      ],
      "mountPoints": [
        { "sourceVolume": "secrets", "containerPath": "/secrets" }
      ],
      "command": [
        "sh", "-c",
        "zvault cloud pull --env $ZVAULT_ENV --output /secrets/.env --format env"
      ]
    },
    {
      "name": "app",
      "image": "my-app:latest",
      "essential": true,
      "dependsOn": [
        { "containerName": "zvault-init", "condition": "SUCCESS" }
      ],
      "mountPoints": [
        { "sourceVolume": "secrets", "containerPath": "/secrets", "readOnly": true }
      ],
      "command": ["sh", "-c", "source /secrets/.env && node server.js"]
    }
  ],
  "volumes": [
    { "name": "secrets" }
  ]
}
```

## Alternative: SDK at Runtime

```typescript
import { ZVault } from '@zvault/sdk';

const vault = new ZVault({ token: process.env.ZVAULT_TOKEN });
const secrets = await vault.getAll({ env: 'production' });
```

Store `ZVAULT_TOKEN` in AWS Secrets Manager and reference via `secrets` in the task definition.

## Fargate

Same pattern works with Fargate — use the init container approach or the runtime SDK.

## Environment Variables

| Variable | Source | Description |
|----------|--------|-------------|
| `ZVAULT_TOKEN` | AWS Secrets Manager | Service token |
| `ZVAULT_ENV` | Task definition | Environment name |

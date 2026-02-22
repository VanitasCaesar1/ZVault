# ZVault Pulumi Provider

Manage ZVault Cloud secrets as Pulumi resources.

## Installation

```bash
# TypeScript/JavaScript
npm install @zvault/pulumi

# Python
pip install pulumi-zvault

# Go
go get github.com/ArcadeLabsInc/pulumi-zvault/sdk/go/zvault

# C#
dotnet add package Pulumi.ZVault
```

## Usage

### TypeScript

```typescript
import * as zvault from "@zvault/pulumi";

const secret = new zvault.Secret("db-url", {
  project: "my-saas",
  environment: "production",
  key: "DATABASE_URL",
  value: dbInstance.connectionString,
});

// Read existing secret
const stripeKey = zvault.Secret.get("stripe", {
  project: "my-saas",
  environment: "production",
  key: "STRIPE_KEY",
});

export const dbUrl = secret.key;
```

### Python

```python
import pulumi_zvault as zvault

secret = zvault.Secret("db-url",
    project="my-saas",
    environment="production",
    key="DATABASE_URL",
    value=db_instance.connection_string,
)
```

### Go

```go
secret, err := zvault.NewSecret(ctx, "db-url", &zvault.SecretArgs{
    Project:     pulumi.String("my-saas"),
    Environment: pulumi.String("production"),
    Key:         pulumi.String("DATABASE_URL"),
    Value:       dbInstance.ConnectionString,
})
```

## Resources

| Resource | Description |
|----------|-------------|
| `zvault.Secret` | Create/update a secret in ZVault Cloud |
| `zvault.Project` | Create a ZVault project |
| `zvault.Environment` | Create a custom environment |
| `zvault.ServiceToken` | Create a scoped service token |

## Configuration

```bash
pulumi config set zvault:token zvt_xxx --secret
pulumi config set zvault:org my-company
```

Or via environment variables:

| Variable | Description |
|----------|-------------|
| `ZVAULT_TOKEN` | Service token |
| `ZVAULT_ORG` | Organization slug |

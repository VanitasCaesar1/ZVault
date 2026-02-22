# ZVault Secret Rotation

Automatic secret rotation with zero-downtime deployment patterns.

## Rotation Types

### Database Credential Rotation

Auto-rotate PostgreSQL, MySQL, and MongoDB passwords on schedule.

```bash
# Configure rotation for a database secret
zvault cloud rotation set DATABASE_URL \
  --env production \
  --interval 30d \
  --type postgres \
  --connection-string "postgres://admin:xxx@db.example.com:5432/myapp"
```

ZVault uses the dual-user pattern:
1. Creates a new user with a random password
2. Grants same permissions as the old user
3. Updates the secret value
4. Fires webhooks so apps refresh
5. Drops the old user after grace period

### AWS IAM Rotation

Generate short-lived AWS credentials via STS AssumeRole:

```bash
zvault cloud rotation set AWS_CREDENTIALS \
  --env production \
  --type aws-iam \
  --role-arn "arn:aws:iam::123456:role/my-app-role" \
  --ttl 1h
```

### GCP Service Account

Generate short-lived GCP OAuth2 tokens:

```bash
zvault cloud rotation set GCP_TOKEN \
  --env production \
  --type gcp-sa \
  --service-account "my-app@project.iam.gserviceaccount.com" \
  --ttl 1h
```

### Azure AD

Generate short-lived Azure service principal tokens:

```bash
zvault cloud rotation set AZURE_TOKEN \
  --env production \
  --type azure-ad \
  --tenant-id "xxx" \
  --client-id "yyy" \
  --ttl 1h
```

### Stripe Key Rotation

Zero-downtime Stripe API key rotation using the dual-key pattern:

```bash
zvault cloud rotation set STRIPE_KEY \
  --env production \
  --type stripe \
  --interval 90d
```

1. Creates a new restricted key in Stripe
2. Updates the secret to the new key
3. Fires webhooks
4. Deletes the old key after 24h grace period

## Rotation Policies

```bash
# Set a rotation policy
zvault cloud rotation policy set \
  --env production \
  --key DATABASE_URL \
  --interval 30d \
  --max-age 90d \
  --notify-before 7d

# List policies
zvault cloud rotation policy list --env production

# View rotation history
zvault cloud rotation history --env production --key DATABASE_URL
```

## Webhooks on Rotation

Fire webhooks when any secret rotates:

```json
{
  "event": "secret.rotated",
  "project": "my-saas",
  "environment": "production",
  "key": "DATABASE_URL",
  "rotated_at": "2026-02-21T14:30:00Z",
  "next_rotation": "2026-03-23T14:30:00Z"
}
```

Configure webhook endpoints in the dashboard or via CLI:

```bash
zvault cloud webhook set \
  --url "https://my-app.com/webhooks/zvault" \
  --events "secret.rotated" \
  --secret "whsec_xxx"
```

## SDK Integration

SDKs automatically pick up rotated secrets via background refresh:

```typescript
const vault = new ZVault({
  token: process.env.ZVAULT_TOKEN,
  refreshInterval: 60_000, // Check every 60s
  onRefresh: (changed) => {
    console.log('Secrets refreshed:', changed);
    // Reconnect database, refresh API clients, etc.
  },
});
```

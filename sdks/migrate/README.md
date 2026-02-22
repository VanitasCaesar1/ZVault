# ZVault Migration Tools

Import secrets from other platforms into ZVault Cloud.

## Supported Sources

| Source | Command | Status |
|--------|---------|--------|
| `.env` files | `zvault import .env` | ✅ Built-in |
| AWS Secrets Manager | `zvault migrate aws-sm --region us-east-1` | ✅ |
| Doppler | `zvault migrate doppler --project my-app` | ✅ |
| Infisical | `zvault migrate infisical --workspace my-ws` | ✅ |
| HashiCorp Vault | `zvault migrate hcv --addr https://vault.example.com` | ✅ |
| 1Password | `zvault migrate 1password --vault Development` | ✅ |
| Vercel | `zvault migrate vercel --project my-app` | ✅ |
| Railway | `zvault migrate railway --project my-app` | ✅ |

## Usage

All migration commands follow the same pattern:

```bash
# 1. Authenticate with ZVault
export ZVAULT_TOKEN=zvt_your_token
export ZVAULT_ORG_ID=org_xxx
export ZVAULT_PROJECT_ID=proj_xxx

# 2. Run migration
zvault migrate <source> [flags] --env production

# 3. Verify
zvault cloud list --env production
```

## AWS Secrets Manager

```bash
# Requires AWS credentials (AWS_ACCESS_KEY_ID, AWS_SECRET_ACCESS_KEY)
zvault migrate aws-sm \
  --region us-east-1 \
  --prefix "myapp/" \
  --env production
```

Imports all secrets matching the prefix. Secret names are normalized:
`myapp/database/url` → `DATABASE_URL`

## Doppler

```bash
# Requires DOPPLER_TOKEN
zvault migrate doppler \
  --project my-app \
  --config prd \
  --env production
```

## Infisical

```bash
# Requires INFISICAL_TOKEN
zvault migrate infisical \
  --workspace my-workspace \
  --environment prod \
  --env production
```

## HashiCorp Vault

```bash
# Requires VAULT_TOKEN and VAULT_ADDR
zvault migrate hcv \
  --addr https://vault.example.com \
  --path secret/data/myapp \
  --env production
```

## 1Password

```bash
# Requires 1Password CLI (op) authenticated
zvault migrate 1password \
  --vault Development \
  --env production
```

## Vercel

```bash
# Requires VERCEL_TOKEN
zvault migrate vercel \
  --project my-app \
  --vercel-env production \
  --env production
```

## Railway

```bash
# Requires RAILWAY_TOKEN
zvault migrate railway \
  --project my-app \
  --railway-env production \
  --env production
```

## Dry Run

All commands support `--dry-run` to preview what would be imported:

```bash
zvault migrate aws-sm --region us-east-1 --dry-run
```

## Implementation

Migration scripts are shell-based and use the ZVault CLI + each platform's CLI/API.
See individual scripts in `scripts/` for implementation details.

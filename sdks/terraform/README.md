# ZVault Terraform Provider

Manage ZVault Cloud secrets as Terraform resources.

## Usage

```hcl
terraform {
  required_providers {
    zvault = {
      source  = "zvault/zvault"
      version = "~> 0.1"
    }
  }
}

provider "zvault" {
  token      = var.zvault_token  # or ZVAULT_TOKEN env var
  org_id     = var.zvault_org_id # or ZVAULT_ORG_ID env var
}

# Read a secret
data "zvault_secret" "db_url" {
  project     = "my-saas"
  environment = "production"
  key         = "DATABASE_URL"
}

# Create/update a secret
resource "zvault_secret" "stripe_key" {
  project     = "my-saas"
  environment = "production"
  key         = "STRIPE_KEY"
  value       = var.stripe_key
  comment     = "Managed by Terraform"
}

# Use the secret value
output "db_url" {
  value     = data.zvault_secret.db_url.value
  sensitive = true
}
```

## Resources

### `zvault_secret`

Manages a secret in ZVault Cloud.

| Argument | Required | Description |
|----------|----------|-------------|
| `project` | Yes | Project ID or slug |
| `environment` | Yes | Environment slug |
| `key` | Yes | Secret key name |
| `value` | Yes | Secret value (sensitive) |
| `comment` | No | Optional comment |

### `zvault_project`

Manages a project.

| Argument | Required | Description |
|----------|----------|-------------|
| `name` | Yes | Project name |
| `slug` | No | URL-friendly slug (auto-generated if omitted) |

## Data Sources

### `data.zvault_secret`

Reads a secret value.

| Argument | Required | Description |
|----------|----------|-------------|
| `project` | Yes | Project ID or slug |
| `environment` | Yes | Environment slug |
| `key` | Yes | Secret key name |

| Attribute | Description |
|-----------|-------------|
| `value` | The secret value (sensitive) |
| `version` | Secret version number |
| `updated_at` | Last update timestamp |

### `data.zvault_secrets`

Reads all secrets for an environment.

| Argument | Required | Description |
|----------|----------|-------------|
| `project` | Yes | Project ID or slug |
| `environment` | Yes | Environment slug |

| Attribute | Description |
|-----------|-------------|
| `secrets` | Map of key → value (sensitive) |

## Provider Configuration

```hcl
provider "zvault" {
  # Token (required) — can also use ZVAULT_TOKEN env var
  token = var.zvault_token

  # Organization ID (required) — can also use ZVAULT_ORG_ID env var
  org_id = var.zvault_org_id

  # API URL (optional)
  url = "https://api.zvault.cloud"
}
```

## Import

```bash
terraform import zvault_secret.stripe_key my-saas/production/STRIPE_KEY
```

## License

MIT

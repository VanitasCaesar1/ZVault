# ZVault GitHub App

GitHub App that scans PRs for hardcoded secrets and suggests `zvault://` references.

## Features

- **PR Secret Scanning**: Detects hardcoded API keys, tokens, passwords in diffs
- **Inline Suggestions**: Comments on the exact line with a `zvault://` replacement
- **Status Checks**: Blocks merge if secrets are detected (configurable)
- **Auto-fix PRs**: Optionally creates a fix commit replacing secrets with references

## Installation

1. Install from GitHub Marketplace: [ZVault Secret Scanner](https://github.com/marketplace/zvault-secret-scanner)
2. Grant access to your repositories
3. Add `.zvault-scan.yml` to your repo (optional)

## Configuration

```yaml
# .zvault-scan.yml
enabled: true

# Severity level to block PRs: "error" (block), "warning" (comment only), "off"
level: error

# Patterns to scan for (built-in patterns always active)
custom_patterns:
  - name: internal-api-key
    pattern: "INTERNAL_[A-Z]+_KEY=['\"]?[a-zA-Z0-9]{32,}"
    severity: error

# Files to exclude from scanning
exclude:
  - "**/*.test.*"
  - "**/*.spec.*"
  - "**/fixtures/**"
  - "**/__mocks__/**"

# Auto-fix: create a commit replacing secrets with zvault:// URIs
auto_fix: false
```

## Built-in Patterns

| Pattern | Examples |
|---------|----------|
| AWS Keys | `AKIA...`, `aws_secret_access_key` |
| Stripe | `sk_live_...`, `sk_test_...`, `pk_live_...` |
| GitHub Tokens | `ghp_...`, `gho_...`, `ghs_...` |
| Database URLs | `postgres://user:pass@...`, `mysql://...` |
| JWT Secrets | `eyJ...` (long base64 strings) |
| Private Keys | `-----BEGIN RSA PRIVATE KEY-----` |
| Generic Secrets | `password=`, `secret=`, `api_key=` (with values) |
| Slack Tokens | `xoxb-...`, `xoxp-...` |
| SendGrid | `SG....` |
| Twilio | `SK...` |

## PR Comment Example

```
🔐 ZVault: Hardcoded secret detected

Line 42 in `src/config.ts`:
- const STRIPE_KEY = "sk_live_abc123...";
+ const STRIPE_KEY = process.env.STRIPE_KEY; // Use: zvault://my-project/STRIPE_KEY

Suggestion: Store this in ZVault Cloud and reference via environment variable.
```

## Webhook Events

The app listens for:
- `pull_request.opened`
- `pull_request.synchronize`
- `push` (for default branch scanning)

## Self-Hosted

For enterprise users who can't use the marketplace app:

```yaml
# docker-compose.yml
services:
  zvault-github-app:
    image: zvault/github-app:latest
    environment:
      GITHUB_APP_ID: "12345"
      GITHUB_PRIVATE_KEY_PATH: /secrets/github-app.pem
      GITHUB_WEBHOOK_SECRET: your-webhook-secret
    ports:
      - "3000:3000"
```

## Privacy

- The app only reads PR diffs — it never clones your repository
- No code is stored or transmitted outside of GitHub
- Secret patterns are matched locally in the webhook handler
- The app does not have write access to your code (only PR comments and checks)

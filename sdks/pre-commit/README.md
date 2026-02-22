# ZVault Pre-Commit Hook

Scan for hardcoded secrets before they reach your repository.

## Using pre-commit framework

```yaml
# .pre-commit-config.yaml
repos:
  - repo: https://github.com/zvault/zvault
    rev: v0.3.0
    hooks:
      - id: zvault-scan
```

```bash
pre-commit install
```

## Manual Installation

```bash
cp zvault-scan.sh .git/hooks/pre-commit
chmod +x .git/hooks/pre-commit
```

## What It Detects

- Stripe API keys (`sk_live_*`, `pk_live_*`)
- AWS credentials (`AKIA*`, `aws_secret_access_key`)
- Private keys (RSA, EC, OpenSSH)
- Database URLs with embedded credentials
- JWT tokens
- ZVault service tokens (should use env vars)
- Generic `password=`, `secret=`, `api_key=`, `token=` patterns

## Bypassing

```bash
git commit --no-verify  # Not recommended
```

## License

MIT

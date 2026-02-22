#!/usr/bin/env bash
# ZVault helper for AWS CodeBuild
# Fetches secrets from ZVault Cloud and exports as environment variables.
#
# Usage in buildspec.yml:
#   phases:
#     install:
#       commands:
#         - curl -fsSL https://zvault.cloud/install.sh | bash
#     pre_build:
#       commands:
#         - source <(zvault cloud pull --env "$ZVAULT_ENV" --format env)
#     build:
#       commands:
#         - npm test  # All secrets available as env vars
#
# Required environment variables (set in CodeBuild project):
#   ZVAULT_TOKEN  — service token scoped to project + environment
#   ZVAULT_ENV    — environment name (e.g., "staging")
set -euo pipefail

ZVAULT_ENV="${ZVAULT_ENV:-staging}"

echo "🔐 ZVault: Fetching secrets for environment '${ZVAULT_ENV}'..."

# Install ZVault CLI if not present
if ! command -v zvault &>/dev/null; then
  curl -fsSL https://zvault.cloud/install.sh | bash
  export PATH="$HOME/.zvault/bin:$PATH"
fi

# Pull secrets and export as env vars
zvault cloud pull --env "${ZVAULT_ENV}" --output /tmp/.zvault-env --format env

# Export each line as an environment variable
set -a
# shellcheck disable=SC1091
source /tmp/.zvault-env
set +a

# Clean up
rm -f /tmp/.zvault-env

echo "✅ ZVault: Secrets loaded for '${ZVAULT_ENV}'"

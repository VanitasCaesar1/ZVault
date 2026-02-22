#!/usr/bin/env bash
# zvault migrate infisical — Import from Infisical
# Requires: curl, jq, zvault CLI, INFISICAL_TOKEN
set -euo pipefail

WORKSPACE="${1:?Usage: migrate-infisical.sh <workspace-id> [environment] [zvault-env]}"
INF_ENV="${2:-prod}"
ZVAULT_ENV="${3:-production}"
DRY_RUN="${DRY_RUN:-false}"
INFISICAL_TOKEN="${INFISICAL_TOKEN:?INFISICAL_TOKEN is required}"
INFISICAL_URL="${INFISICAL_URL:-https://app.infisical.com}"

echo "🔐 Migrating from Infisical (workspace: ${WORKSPACE}, env: ${INF_ENV})..."

SECRETS=$(curl -sf \
  -H "Authorization: Bearer ${INFISICAL_TOKEN}" \
  "${INFISICAL_URL}/api/v3/secrets/raw?workspaceId=${WORKSPACE}&environment=${INF_ENV}" \
  | jq -r '.secrets[] | "\(.secretKey)=\(.secretValue)"') || {
  echo "❌ Failed to fetch Infisical secrets."
  exit 1
}

COUNT=0

while IFS='=' read -r KEY VALUE; do
  [ -z "${KEY}" ] && continue

  if [ "${DRY_RUN}" = "true" ]; then
    echo "  [dry-run] ${KEY}"
    COUNT=$((COUNT + 1))
    continue
  fi

  zvault cloud set "${KEY}" "${VALUE}" --env "${ZVAULT_ENV}" --comment "Migrated from Infisical" 2>/dev/null || {
    echo "  ⚠️  Failed to set ${KEY}"
    continue
  }

  echo "  ✅ ${KEY}"
  COUNT=$((COUNT + 1))
done <<< "${SECRETS}"

echo ""
echo "✅ Migrated ${COUNT} secrets to ZVault (env: ${ZVAULT_ENV})"

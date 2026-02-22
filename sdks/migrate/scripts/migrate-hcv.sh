#!/usr/bin/env bash
# zvault migrate hcv — Import from HashiCorp Vault
# Requires: curl, jq, zvault CLI, VAULT_TOKEN, VAULT_ADDR
set -euo pipefail

VAULT_PATH="${1:?Usage: migrate-hcv.sh <path> [zvault-env]}"
ZVAULT_ENV="${2:-production}"
DRY_RUN="${DRY_RUN:-false}"
VAULT_ADDR="${VAULT_ADDR:?VAULT_ADDR is required}"
VAULT_TOKEN="${VAULT_TOKEN:?VAULT_TOKEN is required}"

echo "🔐 Migrating from HashiCorp Vault (${VAULT_ADDR}/${VAULT_PATH})..."

# Fetch secrets (KV v2)
RESPONSE=$(curl -sf \
  -H "X-Vault-Token: ${VAULT_TOKEN}" \
  "${VAULT_ADDR}/v1/${VAULT_PATH}" 2>/dev/null) || {
  echo "❌ Failed to fetch from HashiCorp Vault."
  exit 1
}

# KV v2 wraps data in .data.data, KV v1 in .data
SECRETS=$(echo "${RESPONSE}" | jq -r '
  if .data.data then .data.data
  elif .data then .data
  else empty end
  | to_entries[] | "\(.key)=\(.value)"') || {
  echo "❌ Failed to parse Vault response."
  exit 1
}

COUNT=0

while IFS='=' read -r KEY VALUE; do
  [ -z "${KEY}" ] && continue

  # Normalize: uppercase
  NORM_KEY=$(echo "${KEY}" | tr '[:lower:]' '[:upper:]' | tr '-' '_')

  if [ "${DRY_RUN}" = "true" ]; then
    echo "  [dry-run] ${KEY} → ${NORM_KEY}"
    COUNT=$((COUNT + 1))
    continue
  fi

  zvault cloud set "${NORM_KEY}" "${VALUE}" --env "${ZVAULT_ENV}" --comment "Migrated from HCV: ${VAULT_PATH}/${KEY}" 2>/dev/null || {
    echo "  ⚠️  Failed to set ${NORM_KEY}"
    continue
  }

  echo "  ✅ ${KEY} → ${NORM_KEY}"
  COUNT=$((COUNT + 1))
done <<< "${SECRETS}"

echo ""
echo "✅ Migrated ${COUNT} secrets to ZVault (env: ${ZVAULT_ENV})"

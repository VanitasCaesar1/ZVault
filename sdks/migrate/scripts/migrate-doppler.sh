#!/usr/bin/env bash
# zvault migrate doppler — Import from Doppler
# Requires: curl, jq, zvault CLI, DOPPLER_TOKEN
set -euo pipefail

PROJECT="${1:?Usage: migrate-doppler.sh <project> [config] [zvault-env]}"
CONFIG="${2:-prd}"
ZVAULT_ENV="${3:-production}"
DRY_RUN="${DRY_RUN:-false}"
DOPPLER_TOKEN="${DOPPLER_TOKEN:?DOPPLER_TOKEN is required}"

echo "🔐 Migrating from Doppler (${PROJECT}/${CONFIG})..."

# Fetch all secrets
SECRETS=$(curl -sf \
  -H "Authorization: Bearer ${DOPPLER_TOKEN}" \
  "https://api.doppler.com/v3/configs/config/secrets?project=${PROJECT}&config=${CONFIG}" \
  | jq -r '.secrets | to_entries[] | select(.value.raw != null) | "\(.key)=\(.value.raw)"') || {
  echo "❌ Failed to fetch Doppler secrets. Check DOPPLER_TOKEN."
  exit 1
}

COUNT=0

while IFS='=' read -r KEY VALUE; do
  [ -z "${KEY}" ] && continue

  # Skip Doppler internal keys
  [[ "${KEY}" == DOPPLER_* ]] && continue

  if [ "${DRY_RUN}" = "true" ]; then
    echo "  [dry-run] ${KEY}"
    COUNT=$((COUNT + 1))
    continue
  fi

  zvault cloud set "${KEY}" "${VALUE}" --env "${ZVAULT_ENV}" --comment "Migrated from Doppler: ${PROJECT}/${CONFIG}" 2>/dev/null || {
    echo "  ⚠️  Failed to set ${KEY}"
    continue
  }

  echo "  ✅ ${KEY}"
  COUNT=$((COUNT + 1))
done <<< "${SECRETS}"

echo ""
echo "✅ Migrated ${COUNT} secrets to ZVault (env: ${ZVAULT_ENV})"

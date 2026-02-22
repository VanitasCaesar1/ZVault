#!/usr/bin/env bash
# zvault migrate 1password — Import from 1Password vault
# Requires: op CLI (1Password CLI) authenticated, jq, zvault CLI
set -euo pipefail

VAULT="${1:?Usage: migrate-1password.sh <vault-name> [zvault-env]}"
ZVAULT_ENV="${2:-production}"
DRY_RUN="${DRY_RUN:-false}"

echo "🔐 Migrating from 1Password (vault: ${VAULT})..."

# List items in vault
ITEMS=$(op item list --vault "${VAULT}" --format json 2>/dev/null | jq -r '.[].id') || {
  echo "❌ Failed to list 1Password items. Is 'op' authenticated?"
  exit 1
}

COUNT=0

while IFS= read -r ITEM_ID; do
  [ -z "${ITEM_ID}" ] && continue

  # Get item details
  ITEM=$(op item get "${ITEM_ID}" --vault "${VAULT}" --format json 2>/dev/null) || continue

  TITLE=$(echo "${ITEM}" | jq -r '.title')
  CATEGORY=$(echo "${ITEM}" | jq -r '.category')

  # Only import password/secure note/API credential items
  case "${CATEGORY}" in
    PASSWORD|SECURE_NOTE|API_CREDENTIAL) ;;
    *) continue ;;
  esac

  # Normalize title to env var key
  KEY=$(echo "${TITLE}" | tr '[:lower:]' '[:upper:]' | tr ' ' '_' | tr '-' '_' | sed 's/[^A-Z0-9_]//g')

  # Extract value based on category
  VALUE=""
  case "${CATEGORY}" in
    PASSWORD)
      VALUE=$(echo "${ITEM}" | jq -r '.fields[] | select(.id == "password") | .value // empty')
      ;;
    SECURE_NOTE)
      VALUE=$(echo "${ITEM}" | jq -r '.fields[] | select(.id == "notesPlain") | .value // empty')
      ;;
    API_CREDENTIAL)
      VALUE=$(echo "${ITEM}" | jq -r '.fields[] | select(.id == "credential") | .value // empty')
      ;;
  esac

  [ -z "${VALUE}" ] && continue

  if [ "${DRY_RUN}" = "true" ]; then
    echo "  [dry-run] ${TITLE} → ${KEY}"
    COUNT=$((COUNT + 1))
    continue
  fi

  zvault cloud set "${KEY}" "${VALUE}" --env "${ZVAULT_ENV}" --comment "Migrated from 1Password: ${VAULT}/${TITLE}" 2>/dev/null || {
    echo "  ⚠️  Failed to set ${KEY}"
    continue
  }

  echo "  ✅ ${TITLE} → ${KEY}"
  COUNT=$((COUNT + 1))
done <<< "${ITEMS}"

echo ""
echo "✅ Migrated ${COUNT} secrets to ZVault (env: ${ZVAULT_ENV})"

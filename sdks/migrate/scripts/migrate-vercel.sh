#!/usr/bin/env bash
# zvault migrate vercel — Import from Vercel environment variables
# Requires: curl, jq, zvault CLI, VERCEL_TOKEN
set -euo pipefail

PROJECT="${1:?Usage: migrate-vercel.sh <project-id> [vercel-env] [zvault-env]}"
VERCEL_ENV="${2:-production}"
ZVAULT_ENV="${3:-production}"
DRY_RUN="${DRY_RUN:-false}"
VERCEL_TOKEN="${VERCEL_TOKEN:?VERCEL_TOKEN is required}"

echo "🔐 Migrating from Vercel (project: ${PROJECT}, env: ${VERCEL_ENV})..."

SECRETS=$(curl -sf \
  -H "Authorization: Bearer ${VERCEL_TOKEN}" \
  "https://api.vercel.com/v9/projects/${PROJECT}/env" \
  | jq -r --arg env "${VERCEL_ENV}" \
    '.envs[] | select(.target[] == $env) | "\(.key)=\(.value // "")"') || {
  echo "❌ Failed to fetch Vercel env vars."
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

  zvault cloud set "${KEY}" "${VALUE}" --env "${ZVAULT_ENV}" --comment "Migrated from Vercel" 2>/dev/null || {
    echo "  ⚠️  Failed to set ${KEY}"
    continue
  }

  echo "  ✅ ${KEY}"
  COUNT=$((COUNT + 1))
done <<< "${SECRETS}"

echo ""
echo "✅ Migrated ${COUNT} secrets to ZVault (env: ${ZVAULT_ENV})"

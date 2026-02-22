#!/usr/bin/env bash
# zvault migrate aws-sm — Import from AWS Secrets Manager
# Requires: aws CLI, jq, zvault CLI
set -euo pipefail

REGION="${1:?Usage: migrate-aws-sm.sh <region> [prefix] [zvault-env]}"
PREFIX="${2:-}"
ZVAULT_ENV="${3:-production}"
DRY_RUN="${DRY_RUN:-false}"

echo "🔐 Migrating from AWS Secrets Manager (${REGION})..."
[ -n "${PREFIX}" ] && echo "   Prefix filter: ${PREFIX}"

# List secrets
SECRETS=$(aws secretsmanager list-secrets \
  --region "${REGION}" \
  --query 'SecretList[].Name' \
  --output json 2>/dev/null | jq -r '.[]') || {
  echo "❌ Failed to list AWS secrets. Check AWS credentials."
  exit 1
}

COUNT=0
TOTAL=$(echo "${SECRETS}" | grep -c . || true)

while IFS= read -r NAME; do
  [ -z "${NAME}" ] && continue

  # Apply prefix filter
  if [ -n "${PREFIX}" ] && [[ "${NAME}" != "${PREFIX}"* ]]; then
    continue
  fi

  # Normalize key: remove prefix, replace / with _, uppercase
  KEY="${NAME#"${PREFIX}"}"
  KEY=$(echo "${KEY}" | tr '/' '_' | tr '[:lower:]' '[:upper:]' | tr '-' '_')

  if [ "${DRY_RUN}" = "true" ]; then
    echo "  [dry-run] ${NAME} → ${KEY}"
    COUNT=$((COUNT + 1))
    continue
  fi

  # Fetch value
  VALUE=$(aws secretsmanager get-secret-value \
    --region "${REGION}" \
    --secret-id "${NAME}" \
    --query 'SecretString' \
    --output text 2>/dev/null) || {
    echo "  ⚠️  Skipping ${NAME} (failed to fetch)"
    continue
  }

  # Import into ZVault
  zvault cloud set "${KEY}" "${VALUE}" --env "${ZVAULT_ENV}" --comment "Migrated from AWS SM: ${NAME}" 2>/dev/null || {
    echo "  ⚠️  Failed to set ${KEY}"
    continue
  }

  echo "  ✅ ${NAME} → ${KEY}"
  COUNT=$((COUNT + 1))
done <<< "${SECRETS}"

echo ""
echo "✅ Migrated ${COUNT} secrets to ZVault (env: ${ZVAULT_ENV})"

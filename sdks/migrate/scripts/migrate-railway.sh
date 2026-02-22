#!/usr/bin/env bash
# zvault migrate railway — Import from Railway environment variables
# Requires: curl, jq, zvault CLI, RAILWAY_TOKEN
set -euo pipefail

PROJECT="${1:?Usage: migrate-railway.sh <project-id> [railway-env] [zvault-env]}"
RAILWAY_ENV="${2:-production}"
ZVAULT_ENV="${3:-production}"
DRY_RUN="${DRY_RUN:-false}"
RAILWAY_TOKEN="${RAILWAY_TOKEN:?RAILWAY_TOKEN is required}"

echo "🔐 Migrating from Railway (project: ${PROJECT}, env: ${RAILWAY_ENV})..."

# Railway uses GraphQL API
QUERY='{"query":"query { project(id: \"'"${PROJECT}"'\") { environments { edges { node { id name } } } } }"}'

ENV_ID=$(curl -sf \
  -X POST \
  -H "Authorization: Bearer ${RAILWAY_TOKEN}" \
  -H "Content-Type: application/json" \
  -d "${QUERY}" \
  "https://backboard.railway.app/graphql/v2" \
  | jq -r --arg name "${RAILWAY_ENV}" \
    '.data.project.environments.edges[] | select(.node.name == $name) | .node.id') || {
  echo "❌ Failed to find Railway environment '${RAILWAY_ENV}'."
  exit 1
}

if [ -z "${ENV_ID}" ]; then
  echo "❌ Environment '${RAILWAY_ENV}' not found in project."
  exit 1
fi

# Fetch variables
VARS_QUERY='{"query":"query { variables(projectId: \"'"${PROJECT}"'\", environmentId: \"'"${ENV_ID}"'\") }"}'

SECRETS=$(curl -sf \
  -X POST \
  -H "Authorization: Bearer ${RAILWAY_TOKEN}" \
  -H "Content-Type: application/json" \
  -d "${VARS_QUERY}" \
  "https://backboard.railway.app/graphql/v2" \
  | jq -r '.data.variables | to_entries[] | "\(.key)=\(.value)"') || {
  echo "❌ Failed to fetch Railway variables."
  exit 1
}

COUNT=0

while IFS='=' read -r KEY VALUE; do
  [ -z "${KEY}" ] && continue

  # Skip Railway internal vars
  [[ "${KEY}" == RAILWAY_* ]] && continue

  if [ "${DRY_RUN}" = "true" ]; then
    echo "  [dry-run] ${KEY}"
    COUNT=$((COUNT + 1))
    continue
  fi

  zvault cloud set "${KEY}" "${VALUE}" --env "${ZVAULT_ENV}" --comment "Migrated from Railway" 2>/dev/null || {
    echo "  ⚠️  Failed to set ${KEY}"
    continue
  }

  echo "  ✅ ${KEY}"
  COUNT=$((COUNT + 1))
done <<< "${SECRETS}"

echo ""
echo "✅ Migrated ${COUNT} secrets to ZVault (env: ${ZVAULT_ENV})"

#!/usr/bin/env bash
# ZVault Bitbucket Pipe — Injects secrets as environment variables
# Usage in bitbucket-pipelines.yml:
#   - pipe: zvault/inject-secrets:0.1.0
#     variables:
#       ZVAULT_TOKEN: $ZVAULT_TOKEN
#       ZVAULT_ORG_ID: "org_xxx"
#       ZVAULT_PROJECT_ID: "proj_xxx"
#       ZVAULT_ENV: "production"

set -euo pipefail

: "${ZVAULT_TOKEN:?'ZVAULT_TOKEN is required'}"
: "${ZVAULT_ORG_ID:?'ZVAULT_ORG_ID is required'}"
: "${ZVAULT_PROJECT_ID:?'ZVAULT_PROJECT_ID is required'}"
ZVAULT_ENV="${ZVAULT_ENV:-production}"
ZVAULT_URL="${ZVAULT_URL:-https://api.zvault.cloud}"
ZVAULT_MASK="${ZVAULT_MASK:-true}"

BASE="${ZVAULT_URL}/v1/cloud/orgs/${ZVAULT_ORG_ID}/projects/${ZVAULT_PROJECT_ID}/envs/${ZVAULT_ENV}/secrets"

echo "🔐 ZVault: Fetching secrets for env '${ZVAULT_ENV}'..."

# Fetch key list
KEYS_JSON=$(curl -sf -H "Authorization: Bearer ${ZVAULT_TOKEN}" \
  -H "User-Agent: zvault-bitbucket-pipe/0.1.0" \
  "${BASE}" 2>/dev/null) || {
  echo "❌ Failed to fetch secret keys from ZVault"
  exit 1
}

KEYS=$(echo "${KEYS_JSON}" | python3 -c "
import sys, json
data = json.load(sys.stdin)
for k in data.get('keys', []):
    print(k['key'])
" 2>/dev/null) || {
  echo "❌ Failed to parse secret keys"
  exit 1
}

COUNT=0
EXPORT_FILE="${BITBUCKET_PIPE_SHARED_STORAGE_DIR:-/tmp}/zvault_env.sh"
: > "${EXPORT_FILE}"

while IFS= read -r KEY; do
  [ -z "${KEY}" ] && continue

  VALUE=$(curl -sf -H "Authorization: Bearer ${ZVAULT_TOKEN}" \
    -H "User-Agent: zvault-bitbucket-pipe/0.1.0" \
    "${BASE}/$(python3 -c "import urllib.parse; print(urllib.parse.quote('${KEY}', safe=''))")" 2>/dev/null \
    | python3 -c "import sys, json; print(json.load(sys.stdin)['secret']['value'])" 2>/dev/null) || continue

  echo "export ${KEY}='${VALUE}'" >> "${EXPORT_FILE}"
  COUNT=$((COUNT + 1))

  # Mask in Bitbucket logs
  if [ "${ZVAULT_MASK}" = "true" ]; then
    echo "::add-mask::${VALUE}" 2>/dev/null || true
  fi
done <<< "${KEYS}"

echo "✅ ZVault: Injected ${COUNT} secrets from '${ZVAULT_ENV}'"
echo "   Source the env file: source ${EXPORT_FILE}"

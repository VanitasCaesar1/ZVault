#!/bin/sh
# ZVault Lambda Extension
#
# This runs as a Lambda extension layer. At cold start, it fetches secrets
# from ZVault Cloud and injects them as environment variables before your
# handler executes.
#
# Usage:
#   1. Package this as a Lambda Layer
#   2. Add ZVAULT_TOKEN, ZVAULT_ORG_ID, ZVAULT_PROJECT_ID as Lambda env vars
#   3. Secrets are available as env vars in your handler

set -e

ZVAULT_ENV="${ZVAULT_ENV:-production}"
ZVAULT_URL="${ZVAULT_URL:-https://api.zvault.cloud}"

if [ -z "$ZVAULT_TOKEN" ] || [ -z "$ZVAULT_ORG_ID" ] || [ -z "$ZVAULT_PROJECT_ID" ]; then
  echo "[zvault] Missing config — skipping secret injection" >&2
  exec "$@"
fi

SECRETS_URL="${ZVAULT_URL}/v1/cloud/orgs/${ZVAULT_ORG_ID}/projects/${ZVAULT_PROJECT_ID}/envs/${ZVAULT_ENV}/secrets"

# Fetch secrets
RESPONSE=$(curl -sf \
  -H "Authorization: Bearer ${ZVAULT_TOKEN}" \
  -H "Content-Type: application/json" \
  -H "User-Agent: zvault-lambda/0.1.0" \
  --max-time 10 \
  "${SECRETS_URL}" 2>/dev/null || echo '{"secrets":[]}')

# Parse and export secrets
# Uses jq if available, falls back to python/node
if command -v jq >/dev/null 2>&1; then
  eval $(echo "$RESPONSE" | jq -r '.secrets[]? | "export \(.key)=\(.value | @sh)"')
elif command -v python3 >/dev/null 2>&1; then
  eval $(python3 -c "
import json, sys, shlex
data = json.loads(sys.stdin.read())
for s in data.get('secrets', []):
    print(f'export {s[\"key\"]}={shlex.quote(s[\"value\"])}')
" <<< "$RESPONSE")
elif command -v node >/dev/null 2>&1; then
  eval $(node -e "
const d = JSON.parse(require('fs').readFileSync('/dev/stdin','utf8'));
(d.secrets||[]).forEach(s => console.log('export ' + s.key + '=' + JSON.stringify(s.value)));
" <<< "$RESPONSE")
else
  echo "[zvault] No JSON parser available (jq, python3, or node required)" >&2
fi

echo "[zvault] Secrets injected for env '${ZVAULT_ENV}'" >&2

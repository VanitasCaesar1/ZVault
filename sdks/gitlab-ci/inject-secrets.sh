#!/usr/bin/env bash
# ZVault GitLab CI Secret Injector
# Fetches secrets from ZVault Cloud and exports them as environment variables.
#
# Usage:
#   source <(./inject-secrets.sh)
#   eval "$(./inject-secrets.sh)"
#
# Required env vars: ZVAULT_TOKEN, ZVAULT_ORG_ID, ZVAULT_PROJECT_ID
# Optional: ZVAULT_ENV (default: production), ZVAULT_URL, ZVAULT_KEYS, ZVAULT_MASK

set -euo pipefail

ZVAULT_URL="${ZVAULT_URL:-https://api.zvault.cloud}"
ZVAULT_ENV="${ZVAULT_ENV:-production}"
ZVAULT_MASK="${ZVAULT_MASK:-true}"
ZVAULT_KEYS="${ZVAULT_KEYS:-}"

# Strip trailing slash
ZVAULT_URL="${ZVAULT_URL%/}"

if [ -z "${ZVAULT_TOKEN:-}" ]; then
  echo "ERROR: ZVAULT_TOKEN is required" >&2
  exit 1
fi

if [ -z "${ZVAULT_ORG_ID:-}" ]; then
  echo "ERROR: ZVAULT_ORG_ID is required" >&2
  exit 1
fi

if [ -z "${ZVAULT_PROJECT_ID:-}" ]; then
  echo "ERROR: ZVAULT_PROJECT_ID is required" >&2
  exit 1
fi

BASE_PATH="/v1/cloud/orgs/${ZVAULT_ORG_ID}/projects/${ZVAULT_PROJECT_ID}/envs/${ZVAULT_ENV}/secrets"

# Fetch key list
KEYS_JSON=$(curl -fsSL \
  -H "Authorization: Bearer ${ZVAULT_TOKEN}" \
  -H "Content-Type: application/json" \
  -H "User-Agent: zvault-gitlab-ci/0.1.0" \
  "${ZVAULT_URL}${BASE_PATH}" 2>/dev/null)

if [ $? -ne 0 ] || [ -z "$KEYS_JSON" ]; then
  echo "ERROR: Failed to fetch secret keys from ZVault" >&2
  exit 1
fi

# Parse keys
ALL_KEYS=$(echo "$KEYS_JSON" | python3 -c "
import sys, json
data = json.load(sys.stdin)
for k in data.get('keys', []):
    print(k['key'])
" 2>/dev/null || echo "$KEYS_JSON" | grep -o '"key":"[^"]*"' | sed 's/"key":"//;s/"//')

# Filter keys if ZVAULT_KEYS is set
if [ -n "$ZVAULT_KEYS" ]; then
  FILTERED=""
  IFS=',' read -ra WANTED <<< "$ZVAULT_KEYS"
  for key in $ALL_KEYS; do
    for wanted in "${WANTED[@]}"; do
      wanted=$(echo "$wanted" | xargs)  # trim whitespace
      if [ "$key" = "$wanted" ]; then
        FILTERED="${FILTERED}${key}\n"
      fi
    done
  done
  ALL_KEYS=$(echo -e "$FILTERED")
fi

COUNT=0

while IFS= read -r key; do
  [ -z "$key" ] && continue

  # URL-encode the key
  ENCODED_KEY=$(python3 -c "import urllib.parse; print(urllib.parse.quote('$key'))" 2>/dev/null || echo "$key")

  # Fetch secret value
  SECRET_JSON=$(curl -fsSL \
    -H "Authorization: Bearer ${ZVAULT_TOKEN}" \
    -H "Content-Type: application/json" \
    -H "User-Agent: zvault-gitlab-ci/0.1.0" \
    "${ZVAULT_URL}${BASE_PATH}/${ENCODED_KEY}" 2>/dev/null)

  if [ -z "$SECRET_JSON" ]; then
    echo "WARNING: Failed to fetch secret '${key}', skipping" >&2
    continue
  fi

  # Parse value
  VALUE=$(echo "$SECRET_JSON" | python3 -c "
import sys, json
data = json.load(sys.stdin)
print(data.get('secret', {}).get('value', ''), end='')
" 2>/dev/null)

  if [ -z "$VALUE" ]; then
    continue
  fi

  # Export the variable
  echo "export ${key}='${VALUE//\'/\'\\\'\'}'"

  # Mask in GitLab CI logs
  if [ "$ZVAULT_MASK" = "true" ] && [ -n "${CI:-}" ]; then
    # GitLab CI doesn't have native masking like GitHub Actions,
    # but we can add the value to the masking list via CI_DEBUG_SERVICES
    # For now, just don't echo the value
    :
  fi

  COUNT=$((COUNT + 1))
done <<< "$ALL_KEYS"

echo "# ZVault: injected ${COUNT} secrets from env '${ZVAULT_ENV}'" >&2

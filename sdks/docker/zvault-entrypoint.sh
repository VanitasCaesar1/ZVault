#!/usr/bin/env sh
# ZVault Docker Entrypoint
# Fetches secrets from ZVault Cloud, exports as env vars, then exec's the command.
#
# Usage in Dockerfile:
#   COPY zvault-entrypoint.sh /usr/local/bin/
#   ENTRYPOINT ["zvault-entrypoint.sh"]
#   CMD ["node", "server.js"]

set -eu

ZVAULT_URL="${ZVAULT_URL:-https://api.zvault.cloud}"
ZVAULT_URL="${ZVAULT_URL%/}"
ZVAULT_ENV="${ZVAULT_ENV:-production}"

if [ -z "${ZVAULT_TOKEN:-}" ]; then
  echo "[zvault] WARNING: ZVAULT_TOKEN not set, skipping secret injection" >&2
  exec "$@"
fi

if [ -z "${ZVAULT_ORG_ID:-}" ] || [ -z "${ZVAULT_PROJECT_ID:-}" ]; then
  echo "[zvault] WARNING: ZVAULT_ORG_ID or ZVAULT_PROJECT_ID not set, skipping" >&2
  exec "$@"
fi

BASE_PATH="/v1/cloud/orgs/${ZVAULT_ORG_ID}/projects/${ZVAULT_PROJECT_ID}/envs/${ZVAULT_ENV}/secrets"

echo "[zvault] Fetching secrets (env: ${ZVAULT_ENV})..." >&2

# Fetch key list
KEYS_JSON=$(curl -fsSL \
  -H "Authorization: Bearer ${ZVAULT_TOKEN}" \
  -H "User-Agent: zvault-docker/0.1.0" \
  "${ZVAULT_URL}${BASE_PATH}" 2>/dev/null) || {
  echo "[zvault] WARNING: Failed to reach ZVault API, starting without secrets" >&2
  exec "$@"
}

# Parse keys (POSIX-compatible, no bash arrays)
ALL_KEYS=$(echo "$KEYS_JSON" | grep -o '"key":"[^"]*"' | sed 's/"key":"//;s/"//')

COUNT=0
for key in $ALL_KEYS; do
  [ -z "$key" ] && continue

  SECRET_JSON=$(curl -fsSL \
    -H "Authorization: Bearer ${ZVAULT_TOKEN}" \
    -H "User-Agent: zvault-docker/0.1.0" \
    "${ZVAULT_URL}${BASE_PATH}/${key}" 2>/dev/null) || continue

  VALUE=$(echo "$SECRET_JSON" | grep -o '"value":"[^"]*"' | head -1 | sed 's/"value":"//;s/"//')

  if [ -n "$VALUE" ]; then
    export "${key}=${VALUE}"
    COUNT=$((COUNT + 1))
  fi
done

echo "[zvault] Injected ${COUNT} secrets, starting application..." >&2

# Replace this process with the actual command
exec "$@"

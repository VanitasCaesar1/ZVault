#!/usr/bin/env bash
# ZVault pre-commit hook — scan for leaked secrets before commit
# Install: cp this to .git/hooks/pre-commit (or use pre-commit framework)
set -euo pipefail

# Patterns that indicate hardcoded secrets
PATTERNS=(
  # API keys
  'sk_live_[a-zA-Z0-9]{24,}'
  'sk_test_[a-zA-Z0-9]{24,}'
  'pk_live_[a-zA-Z0-9]{24,}'
  'pk_test_[a-zA-Z0-9]{24,}'
  # AWS
  'AKIA[0-9A-Z]{16}'
  'aws_secret_access_key\s*=\s*[A-Za-z0-9/+=]{40}'
  # Generic secrets
  'password\s*=\s*["\x27][^"\x27]{8,}["\x27]'
  'secret\s*=\s*["\x27][^"\x27]{8,}["\x27]'
  'api_key\s*=\s*["\x27][^"\x27]{8,}["\x27]'
  'token\s*=\s*["\x27][^"\x27]{8,}["\x27]'
  # Private keys
  '-----BEGIN (RSA |EC |DSA )?PRIVATE KEY-----'
  '-----BEGIN OPENSSH PRIVATE KEY-----'
  # Database URLs with credentials
  'postgres(ql)?://[^:]+:[^@]+@'
  'mysql://[^:]+:[^@]+@'
  'mongodb(\+srv)?://[^:]+:[^@]+@'
  # JWT tokens
  'eyJ[a-zA-Z0-9_-]{10,}\.[a-zA-Z0-9_-]{10,}\.[a-zA-Z0-9_-]{10,}'
  # ZVault tokens (should use env vars, not hardcode)
  'zvt_[a-zA-Z0-9]{32,}'
)

# Files to skip
SKIP_PATTERNS=(
  '\.lock$'
  '\.sum$'
  'node_modules/'
  'vendor/'
  'dist/'
  'build/'
  '\.min\.'
  '\.map$'
  'package-lock\.json'
  'yarn\.lock'
  'pnpm-lock\.yaml'
  '\.env\.example'
  'CHANGELOG'
  'LICENSE'
)

# Get staged files
STAGED_FILES=$(git diff --cached --name-only --diff-filter=ACM 2>/dev/null || true)

if [ -z "${STAGED_FILES}" ]; then
  exit 0
fi

FOUND=0

for FILE in ${STAGED_FILES}; do
  # Skip binary and excluded files
  SKIP=false
  for SKIP_PAT in "${SKIP_PATTERNS[@]}"; do
    if echo "${FILE}" | grep -qE "${SKIP_PAT}"; then
      SKIP=true
      break
    fi
  done
  [ "${SKIP}" = "true" ] && continue

  # Check file exists
  [ -f "${FILE}" ] || continue

  # Scan for patterns
  for PATTERN in "${PATTERNS[@]}"; do
    MATCHES=$(grep -nE "${PATTERN}" "${FILE}" 2>/dev/null || true)
    if [ -n "${MATCHES}" ]; then
      if [ "${FOUND}" -eq 0 ]; then
        echo ""
        echo "🔐 ZVault: Potential secrets detected in staged files!"
        echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
      fi
      FOUND=$((FOUND + 1))
      echo ""
      echo "  ⚠️  ${FILE}:"
      echo "${MATCHES}" | while IFS= read -r LINE; do
        echo "     ${LINE}"
      done
    fi
  done
done

if [ "${FOUND}" -gt 0 ]; then
  echo ""
  echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
  echo "  Found ${FOUND} potential secret(s)."
  echo ""
  echo "  💡 Use ZVault instead of hardcoding secrets:"
  echo "     zvault cloud set MY_SECRET 'value' --env production"
  echo ""
  echo "  To bypass this check (not recommended):"
  echo "     git commit --no-verify"
  echo ""
  exit 1
fi

exit 0

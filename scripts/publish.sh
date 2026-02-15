#!/bin/bash
# Publish all ZVault crates to crates.io in dependency order.
#
# Usage:
#   ./scripts/publish.sh          # Dry run (default)
#   ./scripts/publish.sh --exec   # Actually publish
#
# Prerequisites:
#   cargo login <your-token>
#
# Publish order (dependency chain):
#   1. zvault-storage  (no internal deps)
#   2. zvault-core     (depends on zvault-storage)
#   3. zvault-server   (depends on zvault-core + zvault-storage)
#   4. zvault-cli      (standalone, HTTP client only)

set -euo pipefail

RED='\033[0;31m'
GREEN='\033[0;32m'
CYAN='\033[0;36m'
DIM='\033[2m'
BOLD='\033[1m'
RESET='\033[0m'

CRATES=(
    "zvault-storage"
    "zvault-core"
    "zvault-server"
    "zvault-cli"
)

DRY_RUN=true
if [[ "${1:-}" == "--exec" ]]; then
    DRY_RUN=false
fi

# Get version from workspace Cargo.toml
VERSION=$(grep '^version' Cargo.toml | head -1 | sed 's/.*"\(.*\)"/\1/')
echo -e "${CYAN}${BOLD}ZVault v${VERSION} — crates.io publish${RESET}\n"

if $DRY_RUN; then
    echo -e "${DIM}Dry run mode. Use --exec to actually publish.${RESET}\n"
fi

for crate in "${CRATES[@]}"; do
    echo -e "${CYAN}▸${RESET} ${BOLD}${crate}${RESET}"

    if $DRY_RUN; then
        cargo publish --package "$crate" --dry-run --no-verify 2>&1 | tail -3
    else
        cargo publish --package "$crate" --no-verify
        echo -e "${GREEN}✓${RESET} Published ${crate} v${VERSION}"

        # Wait for crates.io index to update before publishing dependents
        if [[ "$crate" != "zvault-cli" ]]; then
            echo -e "${DIM}  Waiting 30s for crates.io index...${RESET}"
            sleep 30
        fi
    fi
    echo ""
done

if $DRY_RUN; then
    echo -e "${GREEN}✓${RESET} Dry run complete. Run with ${BOLD}--exec${RESET} to publish."
else
    echo -e "${GREEN}✓${RESET} All crates published to crates.io!"
    echo -e "${DIM}  https://crates.io/crates/zvault-cli${RESET}"
fi

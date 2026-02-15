#!/bin/sh
# ZVault installer — https://zvault.cloud
# Usage: curl -fsSL https://zvault.cloud/install.sh | sh
#
# Installs the `zvault` CLI binary to ~/.zvault/bin and adds it to PATH.
# Supports macOS (arm64, x86_64) and Linux (x86_64, aarch64).

set -e

REPO="VanitasCaesar1/zvault"
INSTALL_DIR="$HOME/.zvault/bin"
BINARY_NAME="zvault"

# ── Colors ────────────────────────────────────────────────────────────

RED='\033[0;31m'
GREEN='\033[0;32m'
CYAN='\033[0;36m'
DIM='\033[2m'
BOLD='\033[1m'
RESET='\033[0m'

info()    { printf "${CYAN}${BOLD}▸${RESET} %s\n" "$1"; }
success() { printf "${GREEN}${BOLD}✓${RESET} %s\n" "$1"; }
error()   { printf "${RED}${BOLD}✗${RESET} %s\n" "$1" >&2; exit 1; }

# ── Detect platform ──────────────────────────────────────────────────

detect_platform() {
    OS="$(uname -s)"
    ARCH="$(uname -m)"

    case "$OS" in
        Linux)  OS="linux" ;;
        Darwin) OS="darwin" ;;
        *)      error "Unsupported OS: $OS. ZVault supports Linux and macOS." ;;
    esac

    case "$ARCH" in
        x86_64|amd64)   ARCH="x86_64" ;;
        aarch64|arm64)  ARCH="aarch64" ;;
        *)              error "Unsupported architecture: $ARCH. ZVault supports x86_64 and aarch64." ;;
    esac

    PLATFORM="${OS}-${ARCH}"
}

# ── Get latest release version ────────────────────────────────────────

get_latest_version() {
    if command -v curl >/dev/null 2>&1; then
        VERSION=$(curl -fsSL "https://api.github.com/repos/${REPO}/releases/latest" | grep '"tag_name"' | head -1 | sed 's/.*"tag_name": *"//;s/".*//')
    elif command -v wget >/dev/null 2>&1; then
        VERSION=$(wget -qO- "https://api.github.com/repos/${REPO}/releases/latest" | grep '"tag_name"' | head -1 | sed 's/.*"tag_name": *"//;s/".*//')
    else
        error "Neither curl nor wget found. Please install one and retry."
    fi

    if [ -z "$VERSION" ]; then
        error "Could not determine latest version. Check https://github.com/${REPO}/releases"
    fi
}

# ── Download and install ──────────────────────────────────────────────

download_and_install() {
    TARBALL="zvault-${VERSION}-${PLATFORM}.tar.gz"
    URL="https://github.com/${REPO}/releases/download/${VERSION}/${TARBALL}"

    info "Downloading ZVault ${VERSION} for ${PLATFORM}..."

    TMPDIR=$(mktemp -d)
    trap 'rm -rf "$TMPDIR"' EXIT

    if command -v curl >/dev/null 2>&1; then
        curl -fsSL "$URL" -o "${TMPDIR}/${TARBALL}" || error "Download failed. Release may not exist for your platform yet.\n  URL: ${URL}\n\n  Build from source instead:\n    cargo install --git https://github.com/${REPO} zvault-cli"
    else
        wget -q "$URL" -O "${TMPDIR}/${TARBALL}" || error "Download failed."
    fi

    info "Extracting..."
    tar -xzf "${TMPDIR}/${TARBALL}" -C "$TMPDIR"

    # Find the binary (might be in a subdirectory)
    BINARY=$(find "$TMPDIR" -name "$BINARY_NAME" -type f | head -1)
    if [ -z "$BINARY" ]; then
        error "Binary not found in archive. The release format may have changed."
    fi

    mkdir -p "$INSTALL_DIR"
    mv "$BINARY" "${INSTALL_DIR}/${BINARY_NAME}"
    chmod +x "${INSTALL_DIR}/${BINARY_NAME}"

    success "Installed to ${INSTALL_DIR}/${BINARY_NAME}"
}

# ── Add to PATH ───────────────────────────────────────────────────────

setup_path() {
    case ":$PATH:" in
        *":${INSTALL_DIR}:"*) return ;; # Already in PATH
    esac

    info "Adding ${INSTALL_DIR} to PATH..."

    SHELL_NAME=$(basename "$SHELL" 2>/dev/null || echo "sh")
    EXPORT_LINE="export PATH=\"${INSTALL_DIR}:\$PATH\""

    case "$SHELL_NAME" in
        zsh)
            PROFILE="$HOME/.zshrc"
            ;;
        bash)
            if [ -f "$HOME/.bash_profile" ]; then
                PROFILE="$HOME/.bash_profile"
            else
                PROFILE="$HOME/.bashrc"
            fi
            ;;
        fish)
            FISH_DIR="$HOME/.config/fish"
            mkdir -p "$FISH_DIR"
            PROFILE="$FISH_DIR/config.fish"
            EXPORT_LINE="set -gx PATH ${INSTALL_DIR} \$PATH"
            ;;
        *)
            PROFILE="$HOME/.profile"
            ;;
    esac

    if [ -f "$PROFILE" ] && grep -q "$INSTALL_DIR" "$PROFILE" 2>/dev/null; then
        return # Already added
    fi

    printf '\n# ZVault\n%s\n' "$EXPORT_LINE" >> "$PROFILE"
    success "Added to ${PROFILE}"
}

# ── Main ──────────────────────────────────────────────────────────────

main() {
    printf "\n${CYAN}${BOLD}"
    printf "  ███████╗██╗   ██╗ █████╗ ██╗   ██╗██╗  ████████╗\n"
    printf "  ╚══███╔╝██║   ██║██╔══██╗██║   ██║██║  ╚══██╔══╝\n"
    printf "    ███╔╝ ██║   ██║███████║██║   ██║██║     ██║\n"
    printf "   ███╔╝  ╚██╗ ██╔╝██╔══██║██║   ██║██║     ██║\n"
    printf "  ███████╗ ╚████╔╝ ██║  ██║╚██████╔╝███████╗██║\n"
    printf "  ╚══════╝  ╚═══╝  ╚═╝  ╚═╝ ╚═════╝ ╚══════╝╚═╝\n"
    printf "${RESET}\n"
    printf "  ${DIM}Installer — https://zvault.cloud${RESET}\n\n"

    detect_platform
    get_latest_version
    download_and_install
    setup_path

    printf "\n"
    success "ZVault ${VERSION} installed successfully!"
    printf "\n"
    printf "  ${DIM}Get started:${RESET}\n"
    printf "    ${CYAN}zvault init${RESET}              Initialize a vault\n"
    printf "    ${CYAN}zvault import .env${RESET}       Import secrets from .env\n"
    printf "    ${CYAN}zvault run -- npm dev${RESET}    Run with secrets injected\n"
    printf "\n"
    printf "  ${DIM}Docs:${RESET} https://docs.zvault.cloud\n"
    printf "\n"

    # Hint to reload shell if PATH was just added
    case ":$PATH:" in
        *":${INSTALL_DIR}:"*) ;;
        *) printf "  ${CYAN}→ Restart your terminal or run:${RESET} source ${PROFILE}\n\n" ;;
    esac
}

main

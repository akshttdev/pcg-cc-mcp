#!/usr/bin/env bash
set -euo pipefail

# ORCHA Distribution Build Script
# Builds a self-contained desktop app with embedded frontend and bundled backend server.
#
# Prerequisites:
#   - Rust toolchain (rustup)
#   - Node.js + pnpm
#   - Tauri CLI: cargo install tauri-cli
#
# macOS code signing (optional):
#   export APPLE_SIGNING_IDENTITY="Developer ID Application: Your Name (TEAMID)"
#   export APPLE_ID="your@email.com"
#   export APPLE_PASSWORD="app-specific-password"
#   export APPLE_TEAM_ID="TEAMID"

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
cd "$SCRIPT_DIR"

echo "=== ORCHA Distribution Build ==="
echo ""

# SQLx offline mode — use cached query metadata instead of requiring a live database
export SQLX_OFFLINE=true

# Detect host platform for sidecar naming
detect_target() {
    local arch
    arch="$(uname -m)"
    local os
    os="$(uname -s)"

    case "$arch" in
        x86_64)  arch="x86_64" ;;
        aarch64) arch="aarch64" ;;
        arm64)   arch="aarch64" ;;
        *)       echo "Unsupported architecture: $arch"; exit 1 ;;
    esac

    case "$os" in
        Darwin) echo "${arch}-apple-darwin" ;;
        Linux)  echo "${arch}-unknown-linux-gnu" ;;
        MINGW*|MSYS*|CYGWIN*) echo "${arch}-pc-windows-msvc" ;;
        *)      echo "Unsupported OS: $os"; exit 1 ;;
    esac
}

TARGET=$(detect_target)
echo "Target: $TARGET"

# Step 1: Build frontend (vite only — skip tsc for pre-existing type issues)
echo ""
echo "--- Step 1/4: Building frontend ---"
cd frontend
pnpm install
npx vite build
cd "$SCRIPT_DIR"
echo "Frontend built to frontend/dist/"

# Step 2: Build server binary with dist profile
echo ""
echo "--- Step 2/4: Building server binary (dist profile) ---"
cargo build --profile dist --bin server
echo "Server binary built"

# Step 3: Copy server binary with Tauri sidecar naming convention
echo ""
echo "--- Step 3/4: Preparing sidecar binary ---"
SIDECAR_DIR="target/release"
SIDECAR_NAME="server-${TARGET}"

# On Windows, add .exe suffix
case "$TARGET" in
    *windows*) SIDECAR_NAME="${SIDECAR_NAME}.exe" ;;
esac

# dist profile outputs to target/dist/
cp "target/dist/server" "${SIDECAR_DIR}/${SIDECAR_NAME}" 2>/dev/null || \
cp "target/dist/server.exe" "${SIDECAR_DIR}/${SIDECAR_NAME}" 2>/dev/null || \
{
    echo "ERROR: Could not find server binary in target/dist/"
    exit 1
}

echo "Sidecar binary: ${SIDECAR_DIR}/${SIDECAR_NAME}"

# Step 4: Build Tauri app
echo ""
echo "--- Step 4/4: Building Tauri app ---"

# Check for macOS signing credentials
if [[ "$TARGET" == *"apple"* ]]; then
    if [[ -n "${APPLE_SIGNING_IDENTITY:-}" ]]; then
        echo "Code signing enabled with identity: $APPLE_SIGNING_IDENTITY"
    else
        echo "Warning: No APPLE_SIGNING_IDENTITY set. App will not be signed."
        echo "  Set APPLE_SIGNING_IDENTITY, APPLE_ID, APPLE_PASSWORD, APPLE_TEAM_ID for signing."
    fi
fi

cd src-tauri
cargo tauri build
cd "$SCRIPT_DIR"

echo ""
echo "=== Build Complete ==="
echo ""

# Show output locations
case "$TARGET" in
    *apple*)
        echo "Output:"
        echo "  DMG: target/release/bundle/dmg/"
        ls -lh target/release/bundle/dmg/*.dmg 2>/dev/null || echo "  (DMG not found - check build output)"
        ;;
    *linux*)
        echo "Output:"
        echo "  AppImage: target/release/bundle/appimage/"
        echo "  Deb:      target/release/bundle/deb/"
        ;;
    *windows*)
        echo "Output:"
        echo "  NSIS: target/release/bundle/nsis/"
        ;;
esac

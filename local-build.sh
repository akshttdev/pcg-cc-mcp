#!/bin/bash

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$ROOT_DIR"

map_platform_dir() {
  local os name arch
  os="$(uname -s)"
  arch="$(uname -m)"

  case "$os" in
    Linux)
      case "$arch" in
        x86_64) echo "linux-x64" ;;
        aarch64|arm64) echo "linux-arm64" ;;
        *)
          echo "❌ Unsupported Linux architecture: $arch" >&2
          exit 1
          ;;
      esac
      ;;
    Darwin)
      case "$arch" in
        arm64) echo "macos-arm64" ;;
        x86_64) echo "macos-x64" ;;
        *)
          echo "❌ Unsupported macOS architecture: $arch" >&2
          exit 1
          ;;
      esac
      ;;
    MINGW*|MSYS*|CYGWIN*|Windows_NT)
      case "$arch" in
        x86_64|amd64) echo "windows-x64" ;;
        arm64|aarch64) echo "windows-arm64" ;;
        *)
          echo "❌ Unsupported Windows architecture: $arch" >&2
          exit 1
          ;;
      esac
      ;;
    *)
      echo "❌ Unsupported platform: $os" >&2
      exit 1
      ;;
  esac
}

PLATFORM_DIR="$(map_platform_dir)"
DIST_ROOT="npx-cli/dist"
DIST_DIR="$DIST_ROOT/$PLATFORM_DIR"

echo "🧭 Target platform: $PLATFORM_DIR"
echo "🧹 Cleaning previous builds..."
rm -rf "$DIST_ROOT"
mkdir -p "$DIST_DIR"

APP_BASENAME="pcg-cc"
MCP_BASENAME="pcg-cc-mcp"

echo "🔨 Building frontend..."
(cd frontend && npm run build)

echo "🔨 Building Rust binaries..."
cargo build --release --manifest-path Cargo.toml
cargo build --release --bin mcp_task_server --manifest-path Cargo.toml

package_binary() {
  local source="$1"
  local base="$2"
  local zip_target="$DIST_DIR/${base}.zip"

  cp "$source" "$base"
  zip -q "$base.zip" "$base"
  rm -f "$base"
  mv "$base.zip" "$zip_target"
}

echo "📦 Creating distribution package..."
package_binary "target/release/server" "$APP_BASENAME"
package_binary "target/release/mcp_task_server" "$MCP_BASENAME"

echo "✅ NPM package ready!"
echo "📁 Files created:"
echo "   - $DIST_DIR/${APP_BASENAME}.zip"
echo "   - $DIST_DIR/${MCP_BASENAME}.zip"

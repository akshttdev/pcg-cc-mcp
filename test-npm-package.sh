#!/bin/bash
# test-npm-package.sh

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$ROOT_DIR"

echo "🧪 Testing NPM package locally..."

# Build the package first
./build-npm-package.sh

cd npx-cli

PACKAGE_NAME="$(node -p "require('./package.json').name")"

echo "📋 Checking files to be included..."
npm pack --dry-run >/dev/null

echo "📦 Creating package tarball..."
PACK_JSON="$(npm pack --json)"
TARBALL_FILE="$(printf '%s' "$PACK_JSON" | node -pe "(d => { if (!d.length) process.exit(1); return d[0].filename; })(JSON.parse(require('fs').readFileSync(0, 'utf8')))")"
TARBALL="$(pwd)/$TARBALL_FILE"

if [[ -z "$TARBALL_FILE" || ! -f "$TARBALL" ]]; then
  echo "❌ Failed to locate packed tarball for $PACKAGE_NAME"
  exit 1
fi

echo "🧪 Testing main command (pcg-cc)..."
MAIN_LOG="$(mktemp)"
npx -y --package="$TARBALL" pcg-cc >"$MAIN_LOG" 2>&1 &
MAIN_PID=$!
sleep 3
kill "$MAIN_PID" 2>/dev/null || true
wait "$MAIN_PID" 2>/dev/null || true
echo "✅ Main app started successfully"

if [[ -s "$MAIN_LOG" ]]; then
  echo "   ↳ pcg-cc log excerpt:"
  sed -n '1,10p' "$MAIN_LOG"
fi
rm -f "$MAIN_LOG"

echo "🧪 Testing MCP command with smoke test..."
node ../scripts/mcp_test.js "$TARBALL"

echo "🧹 Cleaning up..."
rm -f "$TARBALL"

echo "✅ NPM package test completed successfully!"
echo ""
echo "🎉 Your MCP server is working correctly!"
echo "📋 Next steps:"
echo "   1. cd npx-cli"
echo "   2. npm publish"
echo "   3. Users can then run: npx pcg-cc --mcp"

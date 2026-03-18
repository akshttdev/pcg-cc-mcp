#!/bin/bash
# Build PCG Dashboard Desktop Application

set -e

echo "╔══════════════════════════════════════════════════════════╗"
echo "║  Building PCG Dashboard Desktop Application             ║"
echo "╚══════════════════════════════════════════════════════════╝"
echo ""

# Navigate to frontend directory
cd "$(dirname "$0")/frontend"

echo "→ Building frontend assets..."
npm run build

echo ""
echo "→ Building Tauri desktop application..."
cd src-tauri
cargo build --release

echo ""
echo "╔══════════════════════════════════════════════════════════╗"
echo "║  ✅ Build Complete!                                     ║"
echo "╠══════════════════════════════════════════════════════════╣"
echo "║  Binary location:                                       ║"
echo "║  ./target/release/pcg-dashboard                         ║"
echo "║                                                          ║"
echo "║  To create installer packages:                          ║"
echo "║  cd frontend && npm run tauri:build                     ║"
echo "╚══════════════════════════════════════════════════════════╝"

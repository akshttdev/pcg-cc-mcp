#!/bin/bash
set -e

echo "🚀 Building PCG Dashboard Desktop Application"
echo "=============================================="
echo ""

# Set proper Rust toolchain
echo "📦 Setting Rust toolchain to stable..."
rustup default stable

# Navigate to project
cd /home/pythia/pcg-cc-mcp

# Set PKG_CONFIG_PATH to find GTK libraries
export PKG_CONFIG_PATH=/usr/lib/x86_64-linux-gnu/pkgconfig:/usr/share/pkgconfig

echo ""
echo "🔧 Building Tauri desktop application..."
echo "   Using our patched zerocopy (AVX-512 fix)"
echo "   Target: Linux x86_64"
echo ""

# Build the desktop app
pnpm tauri build

echo ""
echo "✅ Build complete!"
echo ""
echo "📦 Installers created at:"
echo "   - DEB package:  src-tauri/target/release/bundle/deb/vibertas_*.deb"
echo "   - AppImage:     src-tauri/target/release/bundle/appimage/vibertas_*.AppImage"
echo ""
echo "🎯 To install:"
echo "   sudo dpkg -i src-tauri/target/release/bundle/deb/vibertas_*.deb"
echo ""
echo "   Or run AppImage directly:"
echo "   ./src-tauri/target/release/bundle/appimage/vibertas_*.AppImage"
echo ""

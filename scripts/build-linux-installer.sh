#!/bin/bash
# Build Linux Installer for APN Core
# Run this script to create the AppImage and .deb packages

set -e

echo "════════════════════════════════════════════════════════"
echo "  APN Core - Linux Installer Build Script"
echo "════════════════════════════════════════════════════════"
echo ""

# Check if dependencies are installed
echo "→ Checking dependencies..."
if ! dpkg -l | grep -q libwebkit2gtk-4.1-dev; then
    echo ""
    echo "⚠️  Missing dependencies! Installing..."
    echo ""
    sudo apt-get update
    sudo apt-get install -y \
        libwebkit2gtk-4.1-dev \
        libappindicator3-dev \
        librsvg2-dev \
        patchelf
    echo ""
    echo "✅ Dependencies installed!"
else
    echo "✅ Dependencies already installed"
fi

echo ""
echo "→ Building APN Core..."
echo ""

cd /home/pythia/pcg-cc-mcp/apn-app

# Install npm dependencies
echo "→ Installing npm dependencies..."
npm install

# Build frontend
echo "→ Building frontend..."
npm run build

# Build Tauri app
echo "→ Building Tauri application (this may take 5-10 minutes)..."
npm run tauri build

echo ""
echo "════════════════════════════════════════════════════════"
echo "  ✅ Build Complete!"
echo "════════════════════════════════════════════════════════"
echo ""
echo "Your installers are ready:"
echo ""
echo "📦 AppImage:"
find src-tauri/target/release/bundle/appimage -name "*.AppImage" 2>/dev/null | while read file; do
    size=$(du -h "$file" | cut -f1)
    echo "   $file ($size)"
done
echo ""
echo "📦 Debian Package:"
find src-tauri/target/release/bundle/deb -name "*.deb" 2>/dev/null | while read file; do
    size=$(du -h "$file" | cut -f1)
    echo "   $file ($size)"
done
echo ""
echo "Next steps:"
echo "1. Test the installers"
echo "2. Copy to /var/www/alphaprotocol.network/downloads/linux/"
echo "3. Update the download page links"
echo ""

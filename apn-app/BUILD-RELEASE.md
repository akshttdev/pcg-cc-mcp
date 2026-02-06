# Building APN Core for Distribution

## Overview
This guide covers building cross-platform installers for the Alpha Protocol Network desktop application.

---

## Prerequisites

### All Platforms
- Node.js 18+ and npm
- Rust toolchain (stable)

### Linux
```bash
sudo apt install libwebkit2gtk-4.1-dev \
  build-essential \
  curl \
  wget \
  file \
  libxdo-dev \
  libssl-dev \
  libayatana-appindicator3-dev \
  librsvg2-dev
```

### macOS
```bash
xcode-select --install
```

### Windows
- Visual Studio 2022 Build Tools
- WebView2 (usually pre-installed on Windows 10/11)

---

## Building for Your Platform

### 1. Install Dependencies
```bash
cd /home/pythia/pcg-cc-mcp/apn-app
npm install
```

### 2. Build the Installer
```bash
# Development build
npm run tauri dev

# Production build (creates installer)
npm run tauri build
```

### 3. Find Your Installer

**Linux:**
- AppImage: `src-tauri/target/release/bundle/appimage/apn-app_0.1.0_amd64.AppImage`
- Deb package: `src-tauri/target/release/bundle/deb/apn-app_0.1.0_amd64.deb`

**macOS:**
- DMG: `src-tauri/target/release/bundle/dmg/Alpha Protocol Network_0.1.0_x64.dmg`
- App: `src-tauri/target/release/bundle/macos/Alpha Protocol Network.app`

**Windows:**
- MSI: `src-tauri/target/release/bundle/msi/Alpha Protocol Network_0.1.0_x64_en-US.msi`
- NSIS: `src-tauri/target/release/bundle/nsis/Alpha Protocol Network_0.1.0_x64-setup.exe`

---

## Cross-Platform Build

To build for all platforms, you need to use platform-specific machines or CI/CD:

### Using GitHub Actions (Recommended)

Create `.github/workflows/release.yml`:

```yaml
name: Release Build

on:
  push:
    tags:
      - 'v*'

jobs:
  release:
    strategy:
      fail-fast: false
      matrix:
        platform: [ubuntu-22.04, macos-latest, windows-latest]

    runs-on: ${{ matrix.platform }}

    steps:
      - uses: actions/checkout@v4

      - name: Setup Node
        uses: actions/setup-node@v4
        with:
          node-version: 20

      - name: Setup Rust
        uses: dtolnay/rust-toolchain@stable

      - name: Install Linux dependencies
        if: matrix.platform == 'ubuntu-22.04'
        run: |
          sudo apt-get update
          sudo apt-get install -y libwebkit2gtk-4.1-dev libappindicator3-dev librsvg2-dev

      - name: Install dependencies
        run: |
          cd apn-app
          npm install

      - name: Build
        run: |
          cd apn-app
          npm run tauri build

      - name: Upload artifacts
        uses: actions/upload-artifact@v3
        with:
          name: apn-core-${{ matrix.platform }}
          path: |
            apn-app/src-tauri/target/release/bundle/
```

### Manual Cross-Platform Build

**On Ubuntu (for Linux builds):**
```bash
npm run tauri build
```

**On macOS (for macOS builds):**
```bash
npm run tauri build -- --target universal-apple-darwin
```

**On Windows (for Windows builds):**
```bash
npm run tauri build
```

---

## File Sizes (Approximate)

- **Linux AppImage**: ~60-80 MB
- **Linux .deb**: ~40-60 MB
- **macOS .dmg**: ~50-70 MB
- **Windows .msi**: ~40-60 MB
- **Windows Setup.exe**: ~45-65 MB

---

## Code Signing (Production)

### macOS
```bash
# Set certificate
export APPLE_CERTIFICATE=...
export APPLE_CERTIFICATE_PASSWORD=...
export APPLE_ID=...
export APPLE_PASSWORD=...
export APPLE_TEAM_ID=...

npm run tauri build
```

### Windows
```bash
# Set certificate thumbprint in tauri.conf.json
"windows": {
  "certificateThumbprint": "YOUR_THUMBPRINT"
}
```

---

## Testing the Build

### Linux
```bash
chmod +x src-tauri/target/release/bundle/appimage/*.AppImage
./src-tauri/target/release/bundle/appimage/*.AppImage
```

### macOS
```bash
open src-tauri/target/release/bundle/dmg/*.dmg
```

### Windows
```bash
start src-tauri/target/release/bundle/msi/*.msi
```

---

## Updater Configuration

To enable auto-updates, modify `tauri.conf.json`:

```json
"updater": {
  "active": true,
  "endpoints": [
    "https://alphaprotocol.network/updates/{{target}}/{{current_version}}"
  ],
  "dialog": true,
  "pubkey": "YOUR_PUBLIC_KEY"
}
```

Generate keys:
```bash
npm run tauri signer generate -- -w ~/.tauri/apn.key
```

---

## Distribution Checklist

- [ ] Update version in `package.json`
- [ ] Update version in `src-tauri/Cargo.toml`
- [ ] Update version in `src-tauri/tauri.conf.json`
- [ ] Build for all platforms
- [ ] Test installers on clean systems
- [ ] Generate checksums (SHA256)
- [ ] Upload to distribution server
- [ ] Update website download links
- [ ] Create release notes
- [ ] Announce to community

---

## Quick Build Script

Create `build-all.sh` (run on each platform):

```bash
#!/bin/bash
set -e

echo "Building APN Core v0.1.0..."

cd apn-app
npm install
npm run tauri build

echo "Build complete!"
echo "Installers located in: src-tauri/target/release/bundle/"
```

Make executable: `chmod +x build-all.sh`

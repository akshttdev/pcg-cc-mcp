# APN Core - Distribution Deployment Guide

## Overview

This guide covers deploying APN Core for public distribution via the AlphaProtocol.Network website.

---

## Quick Summary

**What We Have:**
- ✅ Tauri desktop application (cross-platform)
- ✅ Integrated APN node backend (Rust)
- ✅ React frontend UI
- ✅ Auto-start capability
- ✅ System tray integration
- ✅ Downloads page HTML ready

**What You Need:**
- Build installers on each platform (Windows, macOS, Linux)
- Host installers on your web server
- Deploy downloads page to alphaprotocol.network

---

## Building Installers

### Prerequisites by Platform

**Linux** (Ubuntu/Debian):
```bash
sudo apt install libwebkit2gtk-4.1-dev \
  build-essential \
  curl \
  wget \
  file \
  libxdo-dev \
  libssl-dev \
  libayatana-appindicator3-dev \
  librsvg2-dev \
  patchelf
```

**macOS:**
```bash
xcode-select --install
```

**Windows:**
- Visual Studio 2022 Build Tools
- WebView2 (pre-installed on Windows 10/11)

### Build Commands

On each platform:

```bash
cd /path/to/pcg-cc-mcp/apn-app

# Install dependencies
npm install

# Build frontend
npm run build

# Build installer
npm run tauri build
```

### Output Locations

**Linux:**
- `src-tauri/target/release/bundle/appimage/Alpha_Protocol_Network_0.1.0_amd64.AppImage`
- `src-tauri/target/release/bundle/deb/apn-app_0.1.0_amd64.deb`

**macOS:**
- `src-tauri/target/release/bundle/dmg/Alpha Protocol Network_0.1.0_x64.dmg`
- `src-tauri/target/release/bundle/macos/Alpha Protocol Network.app`

**Windows:**
- `src-tauri/target/release/bundle/msi/Alpha Protocol Network_0.1.0_x64_en-US.msi`
- `src-tauri/target/release/bundle/nsis/Alpha Protocol Network_0.1.0_x64-setup.exe`

---

## Hosting the Installers

### 1. Upload to Web Server

Create directory structure:
```
/var/www/alphaprotocol.network/downloads/
├── windows/
│   ├── APN-Core-Setup.exe  (rename from NSIS setup)
│   └── APN-Core.msi
├── macos/
│   └── APN-Core.dmg
└── linux/
    ├── APN-Core.AppImage
    ├── apn-core.deb
    └── apn-core.rpm (optional)
```

### 2. Generate Checksums

```bash
# For each installer
sha256sum APN-Core-Setup.exe > APN-Core-Setup.exe.sha256
sha256sum APN-Core.dmg > APN-Core.dmg.sha256
sha256sum APN-Core.AppImage > APN-Core.AppImage.sha256
```

### 3. Create checksums.json

```json
{
  "version": "0.1.0",
  "released": "2026-02-04",
  "downloads": {
    "windows": {
      "exe": {
        "url": "/downloads/windows/APN-Core-Setup.exe",
        "size": "52428800",
        "sha256": "abc123..."
      },
      "msi": {
        "url": "/downloads/windows/APN-Core.msi",
        "size": "45678900",
        "sha256": "def456..."
      }
    },
    "macos": {
      "dmg": {
        "url": "/downloads/macos/APN-Core.dmg",
        "size": "61234500",
        "sha256": "ghi789..."
      }
    },
    "linux": {
      "appimage": {
        "url": "/downloads/linux/APN-Core.AppImage",
        "size": "72345600",
        "sha256": "jkl012..."
      },
      "deb": {
        "url": "/downloads/linux/apn-core.deb",
        "size": "45678900",
        "sha256": "mno345..."
      }
    }
  }
}
```

---

## Deploying the Downloads Page

### 1. Copy HTML to Website

```bash
# Copy the downloads page
cp apn-app/downloads-page.html /var/www/alphaprotocol.network/download.html

# Or integrate into your existing site
```

### 2. Update Download Links

Edit `download.html` to point to actual hosted files:

```html
<!-- Update these hrefs to match your server structure -->
<a href="/downloads/windows/APN-Core-Setup.exe" class="download-btn">
<a href="/downloads/macos/APN-Core.dmg" class="download-btn">
<a href="/downloads/linux/APN-Core.AppImage" class="download-btn">
```

### 3. Add to Main Website

Add link to navigation:
```html
<nav>
  <a href="/">Home</a>
  <a href="/download">Download</a>
  <a href="/docs">Docs</a>
</nav>
```

---

## Alternative: Use GitHub Releases

### 1. Create GitHub Release

```bash
# Tag the release
git tag -a v0.1.0 -m "APN Core v0.1.0 - Alpha Release"
git push origin v0.1.0
```

### 2. Upload Installers

1. Go to GitHub Releases
2. Create new release from tag v0.1.0
3. Upload installers as release assets
4. Publish release

### 3. Update Download Links

```html
<a href="https://github.com/YourOrg/pcg-cc-mcp/releases/download/v0.1.0/APN-Core-Setup.exe">
```

---

## Using GitHub Actions for Auto-Build

Create `.github/workflows/release.yml`:

```yaml
name: Build and Release

on:
  push:
    tags:
      - 'v*'

jobs:
  build:
    strategy:
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
          sudo apt-get install -y libwebkit2gtk-4.1-dev \
            libappindicator3-dev \
            librsvg2-dev \
            patchelf

      - name: Install dependencies
        run: |
          cd apn-app
          npm install

      - name: Build frontend
        run: |
          cd apn-app
          npm run build

      - name: Build Tauri app
        run: |
          cd apn-app
          npm run tauri build

      - name: Upload Release Assets
        uses: softprops/action-gh-release@v1
        with:
          files: |
            apn-app/src-tauri/target/release/bundle/**/*
```

Push a tag to trigger:
```bash
git tag v0.1.0
git push origin v0.1.0
```

---

## Distribution Checklist

- [ ] Build installers on all platforms
- [ ] Test installers on clean systems
- [ ] Generate SHA256 checksums
- [ ] Upload to web server or GitHub Releases
- [ ] Deploy downloads page
- [ ] Update website navigation
- [ ] Create release notes
- [ ] Test download links
- [ ] Announce release

---

## Release Notes Template

```markdown
# APN Core v0.1.0 - Alpha Release

## What is APN Core?

APN Core is the lightweight desktop client for the Alpha Protocol Network.
Contribute your computational resources and earn VIBE tokens.

## Features

- 🚀 One-click install and setup
- 💰 Passive VIBE token earnings
- 🔒 Sovereign identity with recovery phrase
- ⚡ Auto-start on boot
- 📊 Resource contribution monitoring
- 🎮 GPU support for enhanced earnings

## Downloads

- **Windows**: [APN-Core-Setup.exe](link)
- **macOS**: [APN-Core.dmg](link)
- **Linux**: [APN-Core.AppImage](link)

## System Requirements

- CPU: 2+ cores
- RAM: 4GB+
- Storage: 50GB+
- OS: Windows 10+, macOS 10.15+, Ubuntu 20.04+

## Getting Started

1. Download installer for your OS
2. Run installer
3. Save your 12-word recovery phrase
4. Start earning VIBE!

## Known Issues

- Alpha release - expect bugs
- Report issues to GitHub

## Support

- Documentation: [link]
- Discord: [link]
- GitHub: [link]
```

---

## Updating for New Releases

### 1. Update Version Numbers

```bash
# package.json
"version": "0.2.0"

# src-tauri/Cargo.toml
version = "0.2.0"

# src-tauri/tauri.conf.json
"version": "0.2.0"
```

### 2. Rebuild

```bash
npm run build
npm run tauri build
```

### 3. Redeploy

Upload new installers with version in filename:
- `APN-Core-v0.2.0-Setup.exe`
- `APN-Core-v0.2.0.dmg`
- `APN-Core-v0.2.0.AppImage`

---

## Files Included

- ✅ `downloads-page.html` - Ready-to-deploy downloads page
- ✅ `BUILD-RELEASE.md` - Build instructions
- ✅ `DEPLOYMENT-GUIDE.md` - This file
- ✅ Tauri app source code
- ✅ APN Core backend integration

---

## Next Steps

1. **Build on Each Platform**: Use separate machines or CI/CD
2. **Upload Installers**: To your web server or GitHub
3. **Deploy Downloads Page**: To alphaprotocol.network
4. **Test Downloads**: Verify all links work
5. **Announce**: Share with community

---

**Ready to distribute APN Core to the world!** 🚀

# Quick Start: Distribute APN Core in 3 Steps

## TL;DR

1. Push code to GitHub
2. Create a release tag
3. GitHub builds everything automatically

---

## Step 1: Push to GitHub

```bash
cd /home/pythia/pcg-cc-mcp

# Initialize git (if not already)
git init
git add .
git commit -m "APN Core v0.1.0 - Ready for distribution"

# Add your GitHub remote
git remote add origin https://github.com/YourOrg/pcg-cc-mcp.git

# Push code
git push -u origin main
```

## Step 2: Create Release Tag

```bash
# Tag the release
git tag -a v0.1.0 -m "APN Core v0.1.0 - Alpha Release"

# Push the tag
git push origin v0.1.0
```

## Step 3: Download Built Installers

1. Go to GitHub Actions tab
2. Wait for build to complete (~15 minutes)
3. Download artifacts:
   - `linux-builds.zip`
   - `macos-builds.zip`
   - `windows-builds.zip`
4. Extract and upload to your website

---

## Deploy Downloads Page

```bash
# Copy to your website
scp apn-app/downloads-page.html user@alphaprotocol.network:/var/www/html/download.html

# Or manually upload via FTP/cPanel
```

Update the download links in `download.html` to point to your hosted installers.

---

## Alternative: Manual Build (if needed)

If you need to build locally on each platform:

### On Ubuntu/Linux:
```bash
# Install dependencies
sudo apt-get install -y libwebkit2gtk-4.1-dev libappindicator3-dev librsvg2-dev patchelf

# Build
cd apn-app
npm install
npm run build
npm run tauri build

# Installer at: src-tauri/target/release/bundle/appimage/*.AppImage
```

### On macOS:
```bash
# Build
cd apn-app
npm install
npm run build
npm run tauri build

# Installer at: src-tauri/target/release/bundle/dmg/*.dmg
```

### On Windows:
```bash
# Build
cd apn-app
npm install
npm run build
npm run tauri build

# Installer at: src-tauri/target/release/bundle/nsis/*.exe
```

---

## What You Have Right Now

✅ **Application Code**: Fully functional APN Core with Tauri
✅ **GitHub Workflow**: `.github/workflows/release.yml` ready
✅ **Downloads Page**: `apn-app/downloads-page.html` designed
✅ **Documentation**: Complete build and deployment guides
✅ **Frontend**: Built and ready in `apn-app/dist/`

🔨 **What's Needed**: Just run the GitHub Actions workflow or build manually

---

## File Locations

```
/home/pythia/pcg-cc-mcp/
├── .github/workflows/release.yml    ← GitHub Actions workflow
├── apn-app/
│   ├── downloads-page.html          ← Deploy this to website
│   ├── dist/                        ← Frontend (already built)
│   ├── src/                         ← React source
│   ├── src-tauri/                   ← Rust backend
│   └── package.json
├── DISTRIBUTION-READY.md            ← Full overview
├── DEPLOYMENT-GUIDE.md              ← Detailed deployment steps
└── QUICK-START-DISTRIBUTION.md      ← This file
```

---

## Expected Build Times

- **GitHub Actions**: ~15 minutes for all 3 platforms
- **Local Linux**: ~10 minutes
- **Local macOS**: ~12 minutes
- **Local Windows**: ~15 minutes

---

## After Building

1. **Test installers** on clean systems
2. **Generate checksums**: `sha256sum installer.exe`
3. **Upload to website** or GitHub Releases
4. **Update download links** in downloads-page.html
5. **Announce launch** 🎉

---

## Support

For issues during build:
- Check `BUILD-RELEASE.md` for detailed instructions
- Check `DEPLOYMENT-GUIDE.md` for hosting options
- GitHub Actions logs show detailed error messages

---

**You're 3 commands away from distributing APN Core worldwide!** 🚀

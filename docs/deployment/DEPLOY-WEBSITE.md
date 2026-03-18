# Deploy APN Core Download Page 🚀

## Quick Deploy (5 Minutes)

### Step 1: Build Linux Installer (Optional - Can Skip for Now)

```bash
# From terminal (you'll need to enter sudo password)
cd /home/pythia/pcg-cc-mcp
./build-linux-installer.sh
```

This creates:
- `apn-app/src-tauri/target/release/bundle/appimage/*.AppImage`
- `apn-app/src-tauri/target/release/bundle/deb/*.deb`

### Step 2: Deploy Download Page

The download page is already designed and ready at:
`/home/pythia/pcg-cc-mcp/apn-app/download.html`

**Option A: Deploy to Existing Website**

```bash
# Copy to your web server
scp /home/pythia/pcg-cc-mcp/apn-app/download.html \
    user@alphaprotocol.network:/var/www/html/download.html

# Or if using local server
sudo cp /home/pythia/pcg-cc-mcp/apn-app/download.html \
    /var/www/html/download.html
```

**Option B: Test Locally First**

```bash
# Open in browser to preview
firefox /home/pythia/pcg-cc-mcp/apn-app/download.html
# or
google-chrome /home/pythia/pcg-cc-mcp/apn-app/download.html
```

### Step 3: Upload Linux Binaries (When Ready)

```bash
# After building, upload to your server
mkdir -p /var/www/alphaprotocol.network/downloads/linux/

cp apn-app/src-tauri/target/release/bundle/appimage/*.AppImage \
   /var/www/alphaprotocol.network/downloads/linux/APN-Core.AppImage

cp apn-app/src-tauri/target/release/bundle/deb/*.deb \
   /var/www/alphaprotocol.network/downloads/linux/apn-core.deb

# Generate checksums
cd /var/www/alphaprotocol.network/downloads/linux/
sha256sum APN-Core.AppImage > APN-Core.AppImage.sha256
sha256sum apn-core.deb > apn-core.deb.sha256
```

---

## What's Live Now

### ✅ Ready to Deploy Immediately

1. **Download Page** (`download.html`)
   - Shows GitHub installation (primary method)
   - Has placeholder for binary downloads
   - Copy button for code snippets
   - Responsive design
   - Tab switching between methods

2. **GitHub Installation Works Now**
   - Users can clone and build
   - Full instructions included
   - Cross-platform support

### 🔨 Ready When You Build

3. **Linux Binaries**
   - Run `./build-linux-installer.sh` when ready
   - Upload to `/downloads/linux/`
   - Mark as "Available" in download.html

4. **macOS & Windows**
   - Build on respective platforms
   - Or use GitHub Actions
   - Coming soon badges already in place

---

## Current Status

### Live Distribution Methods:

**Method 1: GitHub (Live Now ✅)**
```bash
git clone https://github.com/KingBodhi/pcg-cc-mcp.git
cd pcg-cc-mcp/apn-app
npm install && npm run build && npm run tauri dev
```

Users can start using this TODAY.

**Method 2: Linux Binary (Ready to Build ⏳)**
- Run build script
- Upload to server
- Update download page

**Method 3: macOS/Windows (Planned 📅)**
- Use GitHub Actions
- Or build on native machines

---

## Testing the Download Page

### Local Preview

```bash
# Open in browser
cd /home/pythia/pcg-cc-mcp/apn-app
firefox download.html
```

### Check:
- [ ] GitHub instructions display correctly
- [ ] Copy buttons work
- [ ] Tabs switch properly
- [ ] Links point to correct GitHub repo
- [ ] Responsive on mobile (resize browser)

---

## Updating the Page

### Add Your GitHub URL

Edit `download.html` and replace:

```html
<!-- Line ~208 -->
<pre>git clone https://github.com/KingBodhi/pcg-cc-mcp.git

<!-- Change to your actual repo URL -->
<pre>git clone https://github.com/YourOrg/your-repo.git
```

### When Linux Build is Ready

Edit `download.html`:

```html
<!-- Line ~290 -->
<div class="download-card coming-soon">  <!-- Remove coming-soon class -->
<div class="download-card">

<!-- Update badge -->
<span class="badge soon">Coming Soon</span>  <!-- Change to: -->
<span class="badge available">Available</span>
```

---

## Integration with Main Site

### Add Navigation Link

On your main website (index.html), add:

```html
<nav>
  <a href="/">Home</a>
  <a href="/download">Get APN Core</a>  <!-- Add this -->
  <a href="/docs">Docs</a>
</nav>
```

### Add CTA Button

```html
<section class="hero">
  <h1>Join the Alpha Protocol Network</h1>
  <p>Contribute compute power, earn VIBE tokens</p>
  <a href="/download" class="cta-button">Download APN Core</a>
</section>
```

---

## File Locations Summary

```
Website Structure:
/var/www/alphaprotocol.network/
├── index.html (your main site)
├── download.html (← deploy this)
└── downloads/
    ├── linux/
    │   ├── APN-Core.AppImage
    │   ├── apn-core.deb
    │   └── checksums.sha256
    ├── macos/ (coming soon)
    │   └── APN-Core.dmg
    └── windows/ (coming soon)
        └── APN-Core-Setup.exe

Source Files:
/home/pythia/pcg-cc-mcp/
├── apn-app/download.html (← source file)
├── build-linux-installer.sh (← run this to build)
└── apn-app/src-tauri/target/release/bundle/ (← builds go here)
```

---

## Quick Commands

### Deploy download page:
```bash
sudo cp /home/pythia/pcg-cc-mcp/apn-app/download.html \
        /var/www/html/download.html
```

### Build Linux binaries:
```bash
/home/pythia/pcg-cc-mcp/build-linux-installer.sh
```

### Test locally:
```bash
firefox /home/pythia/pcg-cc-mcp/apn-app/download.html
```

---

## What Users See Right Now

1. Visit: `https://alphaprotocol.network/download`
2. See two options:
   - **"From GitHub"** tab (default, active)
     - Full installation instructions
     - Copy-paste commands
     - Works for all platforms
   - **"Download Binary"** tab
     - Linux: Available (when you build)
     - macOS: Coming Soon
     - Windows: Coming Soon

3. Click GitHub tab → copy commands → install

**This works TODAY.** No waiting for builds required!

---

## Next Steps (Priority Order)

1. **Immediate (5 min):**
   ```bash
   sudo cp apn-app/download.html /var/www/html/download.html
   ```
   ✅ GitHub installation live immediately

2. **When Ready (30 min):**
   ```bash
   ./build-linux-installer.sh
   # Upload binaries
   # Update download.html
   ```
   ✅ Linux binaries available

3. **Later (Use GitHub Actions):**
   - Push tag to GitHub
   - Auto-build all platforms
   - Download and host

---

**Bottom Line: You can go live with GitHub installation TODAY. Binary downloads are a nice-to-have that you can add later.**

Ready to deploy? 🚀

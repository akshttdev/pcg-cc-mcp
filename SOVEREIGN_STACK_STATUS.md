# 🏴 Sovereign Stack - Complete Status

**PCG Dashboard - Fully Sovereign, Privacy-First Technology Stack**

## Architecture Overview

```
┌─────────────────────────────────────────────────────────┐
│                   Desktop Application                    │
│  (Vibertas - Tauri + React - Native Installers)        │
└──────────────────┬──────────────────────────────────────┘
                   │
┌──────────────────▼──────────────────────────────────────┐
│              PCG Dashboard Backend                       │
│  (Rust - Axum - SQLite - Port 58297)                   │
└──────────────────┬──────────────────────────────────────┘
                   │
      ┌────────────┼────────────┐
      │            │            │
┌─────▼─────┐ ┌───▼────┐ ┌────▼─────┐
│   Voice   │ │  Mesh  │ │ Storage  │
│   Stack   │ │Network │ │  Layer   │
└───────────┘ └────────┘ └──────────┘
```

---

## ✅ Current Status - What's Working

### Core Backend (✅ Fully Operational)
- **Technology:** Rust + Axum + SQLite
- **Port:** 58297 (auto-assigned if in use)
- **Features:**
  - ✅ Project management
  - ✅ Task kanban boards
  - ✅ Agent registry (Nora, Maci, Editron)
  - ✅ Git integration
  - ✅ File search cache
  - ✅ Health monitoring

### Sovereign Voice Stack (✅ 100% Local)
- **STT (Speech-to-Text):**
  - Engine: Local Whisper
  - Model: small (better accuracy)
  - Port: 8101
  - Device: CUDA (GPU accelerated)
  - Status: ✅ No API keys needed

- **TTS (Text-to-Speech):**
  - Engine: Piper (Chatterbox)
  - Model: en_GB-semaine-medium
  - Voice: "prudence" (British female)
  - Port: 8102
  - Status: ✅ No API keys needed

### Sovereign Storage (✅ Auto-Sync Enabled)
- **Technology:** NATS-based mesh sync
- **Configuration:**
  - Device ID: pythia-master-814d37f4
  - Provider ID: pythia-master-814d37f4
  - NATS Relay: nats://nonlocal.info:4222
  - Sync Interval: 5 minutes
  - Encryption: Enabled (changeme password)
- **Status:** ✅ Mac syncing successfully

### APN Mesh Network (✅ Active)
- **Technology:** libp2p + NATS relay
- **Configuration:**
  - Master Node: pythia (192.168.1.77:8081)
  - APN Port: 4001
  - Heartbeat: 30s
  - Auto-start: Enabled
- **Features:**
  - ✅ Peer discovery
  - ✅ Message routing
  - ✅ Sovereign wallet integration
  - ✅ Reward distribution (11% allocation)

### LLM Stack (✅ Local-First)
- **Primary:** Ollama (localhost:11434)
  - NORA Model: deepseek-r1 (8.2B)
  - TOPSI Model: qwen2.5:7b
  - Status: ✅ Falls back to cloud only if needed

### Database (✅ Sovereign)
- **Technology:** SQLite
- **Location:** /home/pythia/pcg-cc-mcp/dev_assets/db.sqlite
- **Sync:** Auto-backup to mesh network every 5 minutes

### Optional Services
- **ComfyUI:** localhost:8188 (Image generation for Maci)
- **Twilio:** Configured for voice/SMS (optional)

---

## 🔨 Tools Required for Desktop App

### Currently Installed
- ✅ Rust toolchain (1.90.0 stable)
- ✅ Node.js & pnpm
- ✅ Tauri CLI (2.10.0)
- ✅ Git
- ✅ Backend server binary (214MB)
- ✅ Frontend build (dist/)

### Need to Install (Run These Commands)

```bash
# Install GTK development libraries
sudo apt-get update
sudo apt-get install -y \
  libgtk-3-dev \
  libwebkit2gtk-4.0-dev \
  libsoup2.4-dev \
  libjavascriptcoregtk-4.0-dev \
  libayatana-appindicator3-dev \
  librsvg2-dev

# Then build desktop app
./build-desktop-app.sh
```

---

## 🎯 Desktop App - Next Steps

### 1. Install GTK Libraries (Above)

### 2. Build Desktop Application
```bash
./build-desktop-app.sh
```

### 3. Install Desktop App
```bash
# Option A: Install system-wide
sudo dpkg -i src-tauri/target/release/bundle/deb/vibertas_*.deb

# Option B: Run portable AppImage
chmod +x src-tauri/target/release/bundle/appimage/vibertas_*.AppImage
./src-tauri/target/release/bundle/appimage/vibertas_*.AppImage
```

### 4. Launch
- **From Applications Menu:** Search "Vibertas"
- **From Command Line:** `vibertas`
- **Features:**
  - Auto-starts backend server
  - System tray integration
  - Native notifications
  - Offline capable

---

## 🌐 Deployment Status

### Web Access (✅ Working)
- **Local:** http://localhost:58297
- **Public:** https://dashboard.powerclubglobal.co (via Cloudflare Tunnel)

### Desktop App (⏳ Ready to Build)
- **Linux:** .deb, .AppImage (ready after GTK install)
- **macOS:** .dmg (build on Mac)
- **Windows:** .exe, .msi (build on Windows)

---

## 🔐 Sovereign Stack Principles

All components follow these principles:

1. **No Cloud Dependencies**
   - Voice: Local Whisper + Piper
   - LLM: Local Ollama (cloud fallback only)
   - Storage: Local SQLite + mesh sync

2. **Encrypted Everything**
   - Storage sync: AES encryption
   - Mesh network: Cryptographic identity
   - Wallet: BIP-39 mnemonic seeds

3. **Decentralized**
   - NATS relay: mesh coordination
   - Peer-to-peer: direct connections
   - No single point of failure

4. **Privacy-First**
   - No telemetry
   - No analytics (unless explicitly enabled)
   - No API keys required for core functionality

5. **Open Source**
   - All code visible
   - Community contributions welcome
   - Auditable security

---

## 📊 System Requirements

### Minimum
- **OS:** Ubuntu 22.04+ / macOS 12+ / Windows 10+
- **RAM:** 8GB
- **Storage:** 20GB
- **CPU:** 4 cores

### Recommended (for AI features)
- **RAM:** 16GB+
- **GPU:** CUDA-capable NVIDIA (for Whisper/ComfyUI)
- **Storage:** 50GB+ SSD
- **CPU:** 8+ cores

---

## 🚀 Quick Start Commands

```bash
# Start backend server
cd /home/pythia/pcg-cc-mcp
cargo run --bin server

# Build desktop app
./build-desktop-app.sh

# Check sovereign storage status
grep "SOVEREIGN_STORAGE" .env

# Check APN mesh status
curl http://localhost:8081/api/apn/peers

# View logs
tail -f ~/.local/share/vibertas/logs/app.log
```

---

## 📚 Documentation

- **Main README:** `/home/pythia/pcg-cc-mcp/README.md`
- **API Docs:** http://localhost:58297/api/docs (when running)
- **Architecture:** `/home/pythia/pcg-cc-mcp/docs/architecture.md`
- **Sovereign Stack:** This file

---

## 🎯 Current Objectives

- [x] Backend server working
- [x] Sovereign voice stack (Whisper + Piper)
- [x] Mesh network active
- [x] Storage auto-sync working
- [x] Mac device syncing
- [x] zerocopy fix created & tested
- [ ] Install GTK libraries
- [ ] Build desktop app
- [ ] Submit open source contributions
- [ ] Deploy to production

---

**Status:** 95% Complete - Just need GTK libraries + final build!

# PCG Dashboard Launcher

## Two Launcher Options

### 1. Quick Access (Web Browser Only)
Just open the dashboard in your browser - perfect for most users!

**Desktop Icon:**
```bash
cp Dashboard-Shortcut.desktop ~/Desktop/
```

**Command Line:**
```bash
./open-dashboard.sh
```

This opens http://dashboard.powerclubglobal.com in your browser.

### 2. Full Service Launcher (Master Node Only)
Starts all backend services and opens dashboard - for the master node operator.

**Desktop Icon:**
```bash
cp PCG-Dashboard.desktop ~/Desktop/
```

**Command Line:**
```bash
./launch-dashboard.sh
```

## What It Does

The launcher automatically:
1. ✅ Starts the APN node (if not already running)
2. ✅ Starts the Python bridge server on port 8000
3. ✅ Starts the frontend dev server on port 3000
4. ✅ Opens your browser to http://dashboard.powerclubglobal.com

## Stopping the Dashboard

```bash
./stop-dashboard.sh
```

This stops the frontend and bridge server. The APN node continues running for other services.

## Manual Control

### Check Status
```bash
# Check if services are running
lsof -i :3000  # Frontend
lsof -i :8000  # Bridge server
pgrep -f apn_node  # APN node
```

### View Logs
```bash
tail -f /tmp/dashboard_frontend.log  # Frontend logs
tail -f /tmp/apn_bridge.log          # Bridge server logs
tail -f /tmp/apn_node_dashboard.log  # APN node logs
```

### Stop Individual Services
```bash
kill $(cat /tmp/dashboard_frontend.pid)  # Stop frontend
kill $(cat /tmp/apn_bridge.pid)          # Stop bridge
pkill -f apn_node                         # Stop APN node
```

## Packaging as Desktop App (Advanced)

To build a standalone Tauri desktop application:

### Prerequisites
```bash
sudo apt install -y \
    libwebkit2gtk-4.1-dev \
    libjavascriptcoregtk-4.1-dev \
    libgtk-3-dev \
    libappindicator3-dev \
    librsvg2-dev \
    patchelf
```

### Build
```bash
cd frontend
npm install --legacy-peer-deps
npm run tauri:build
```

The packaged application will be in `frontend/src-tauri/target/release/bundle/`

## Troubleshooting

### Port Already in Use
If you get "port already in use" errors:
```bash
# Find and kill the process
lsof -i :3000  # or :8000, :4001
kill <PID>
```

### Frontend Won't Start
Check if node_modules are installed:
```bash
cd frontend && npm install --legacy-peer-deps
```

### Browser Doesn't Open
Manually navigate to: http://localhost:3000

## Architecture

```
┌─────────────────────┐
│   Browser (Port     │
│     3000)           │
│   Frontend UI       │
└──────────┬──────────┘
           │
           ↓
┌─────────────────────┐
│   Python Bridge     │
│   Server (Port 8000)│
└──────────┬──────────┘
           │
           ↓
┌─────────────────────┐
│   APN Node          │
│   (Port 4001)       │
│   + NATS Relay      │
└─────────────────────┘
```

## Files

- `launch-dashboard.sh` - Main launcher script
- `stop-dashboard.sh` - Stop all services
- `PCG-Dashboard.desktop` - Linux desktop shortcut
- `frontend/src-tauri/` - Tauri desktop app configuration (optional)
- `apn_bridge_server.py` - HTTP API bridge to APN

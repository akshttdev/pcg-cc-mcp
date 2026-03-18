# Dashboard Configuration Fix

## Problem Identified

The dashboard is broken because:

1. ❌ **Nginx is proxying to port 3001** but backend server is running on **port 58297**
2. ❌ Port mismatch causes "Failed to reach the PCG CC server" errors
3. ⚠️ Firefox needs to be replaced with Brave

## Solution

### Step 1: Run Fix Script (Requires Sudo)

```bash
sudo ./fix-dashboard-config.sh
```

This script will:
- ✅ Update nginx config to proxy `/api/` requests to port **58297**
- ✅ Reload nginx
- ✅ Install Brave browser
- ✅ Verify configuration

### Step 2: Verify Services

Check all required services are running:

```bash
# Backend server (port 58297)
lsof -i :58297

# APN Bridge (port 8000)
lsof -i :8000

# APN Node (port 4001)
lsof -i :4001

# Frontend dev server (port 3000)
lsof -i :3000
```

### Step 3: Test Dashboard

```bash
# Test backend API
curl http://localhost:58297/api/health

# Test through nginx proxy
curl http://dashboard.powerclubglobal.com/api/health

# Test projects endpoint
curl http://localhost:58297/api/projects | jq '.data[].name'
```

### Step 4: Open in Brave

```bash
brave-browser http://dashboard.powerclubglobal.com &
```

## Current Status

### Services Running:
- ✅ Backend Server: Port 58297 (PID in /tmp/pcg_backend.pid)
- ✅ APN Bridge: Port 8000
- ✅ APN Node: Port 4001
- ✅ Frontend Dev: Port 3000
- ✅ Nginx: Port 80

### Configuration Issues:
- ❌ Nginx config points to port 3001 (WRONG)
- ✅ Backend is on port 58297 (CORRECT)
- ❌ Mismatch causes API failures

## Manual Fix (If Script Fails)

If you need to fix manually:

### 1. Edit Nginx Config
```bash
sudo nano /etc/nginx/sites-available/pcg-dashboard
```

Find all instances of:
```nginx
proxy_pass http://127.0.0.1:3001;
```

Replace with:
```nginx
proxy_pass http://127.0.0.1:58297;
```

### 2. Test and Reload
```bash
sudo nginx -t
sudo systemctl reload nginx
```

### 3. Install Brave
```bash
sudo curl -fsSLo /usr/share/keyrings/brave-browser-archive-keyring.gpg \
  https://brave-browser-apt-release.s3.brave.com/brave-browser-archive-keyring.gpg

echo "deb [signed-by=/usr/share/keyrings/brave-browser-archive-keyring.gpg] \
  https://brave-browser-apt-release.s3.brave.com/ stable main" | \
  sudo tee /etc/apt/sources.list.d/brave-browser-release.list

sudo apt update
sudo apt install -y brave-browser
```

## After Fix

Your dashboard will be fully operational:
- ✅ Projects load correctly
- ✅ All API endpoints working
- ✅ Brave browser as default
- ✅ All backend services connected

Test the fixed dashboard:
```bash
brave-browser http://dashboard.powerclubglobal.com
```

## Launcher Scripts

The launcher scripts have been updated to start the backend on port 58297:
- `launch-dashboard.sh` - Start all services
- `stop-dashboard.sh` - Stop all services
- `open-dashboard.sh` - Just open the URL

After running the fix script, these launchers will work correctly!

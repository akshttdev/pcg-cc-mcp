# 🔄 Integrated Auto-Sync - Quick Start

The PCG Dashboard now has **built-in automatic syncing** to storage providers!

---

## ✅ What's New

Sovereign storage sync is now **integrated directly into the dashboard backend**. When you start the dashboard, it automatically:

1. Reads configuration from `.env`
2. Starts background auto-sync service
3. Syncs database every 5 minutes (configurable)
4. Logs sync activity to dashboard logs

**No separate services to manage!** Just configure and run.

---

## 🚀 Quick Setup

### Step 1: Configure `.env`

Edit `/home/pythia/pcg-cc-mcp/.env` and update these settings:

```bash
# Sovereign Storage Configuration
SOVEREIGN_STORAGE_ENABLED=true
SOVEREIGN_STORAGE_DEVICE_ID=c5ff23d2-1bdc-4479-b614-dfc103e8aa67  # Your device ID
SOVEREIGN_STORAGE_PROVIDER_ID=d903c0e7-f3a6-4967-80e7-0da9d0fe7632  # Pythia Master
SOVEREIGN_STORAGE_PASSWORD=your_strong_password_here  # ⚠️ CHANGE THIS!
SOVEREIGN_STORAGE_SYNC_INTERVAL=5  # Minutes between syncs
SOVEREIGN_STORAGE_NATS_URL=nats://nonlocal.info:4222
```

**Device IDs:**
- **Sirak Laptop:** `c5ff23d2-1bdc-4479-b614-dfc103e8aa67`
- **Bonomotion Studio:** `ac5eefa8-6ccb-4181-a72d-062be107c338`
- **Pythia Master (Provider):** `d903c0e7-f3a6-4967-80e7-0da9d0fe7632`

### Step 2: Install Dependencies

```bash
pip3 install nats-py cryptography
```

### Step 3: Start Dashboard

```bash
cd /home/pythia/pcg-cc-mcp
./start-all-services.sh
```

**That's it!** Auto-sync starts automatically.

---

## 📊 Verify It's Working

### Check Dashboard Logs

```bash
# Start dashboard and watch logs
./start-all-services.sh

# Look for:
======================================================================
🔄 Starting Sovereign Storage Auto-Sync
======================================================================
Device ID: c5ff23d2-1bdc-4479-b614-dfc103e8aa67
Provider: d903c0e7-f3a6-4967-80e7-0da9d0fe7632
Sync Interval: 5 minutes
Cost: ~2 VIBE/GB/month ($0.02/GB)
======================================================================
✅ Config saved to: /home/user/.config/pcg-dashboard/sovereign-sync.json
✅ Auto-sync service started (PID: 12345)

[SOVEREIGN_SYNC] ✅ Connected to nats://nonlocal.info:4222
[SOVEREIGN_SYNC] 📤 Starting sync to storage provider
[SOVEREIGN_SYNC] ✅ Sync complete!
```

### Check Storage Provider (Pythia)

On Pythia server:

```bash
# View storage stats
/home/pythia/pcg-cc-mcp/manage_storage_provider.sh stats

# Check for your encrypted backup
ls -lh /home/pythia/.sovereign_storage/
# You should see: c5ff23d2-1bdc-4479-b614-dfc103e8aa67.db.encrypted
```

---

## 🎛️ Configuration Options

| Variable | Description | Default |
|----------|-------------|---------|
| `SOVEREIGN_STORAGE_ENABLED` | Enable/disable auto-sync | `false` |
| `SOVEREIGN_STORAGE_DEVICE_ID` | Your device's unique ID | *required* |
| `SOVEREIGN_STORAGE_PROVIDER_ID` | Storage provider ID | Pythia Master |
| `SOVEREIGN_STORAGE_PASSWORD` | Encryption password | *required* |
| `SOVEREIGN_STORAGE_SYNC_INTERVAL` | Minutes between syncs | `5` |
| `SOVEREIGN_STORAGE_NATS_URL` | NATS relay URL | `nats://nonlocal.info:4222` |

**Security Note:** The password is stored in `.env` - make sure to:
- Use a strong, unique password
- Don't commit `.env` to git
- Keep `.env` file permissions secure (`chmod 600 .env`)

---

## 💰 Pricing

**1 VIBE = $0.01 USD**

### Cost (Client)
- **Storage:** 2 VIBE/GB/month ($0.02/GB)
- **Transfer:** 0.5 VIBE/GB ($0.005/GB)
- **Example (5GB):** ~10.5 VIBE/month ($0.105)

### Revenue (Provider)
- Same rates + 1.5x uptime bonus
- **Example (5GB client):** ~16 VIBE/month ($0.16)

---

## 🔍 Troubleshooting

### Auto-Sync Not Starting

**Check logs:**
```bash
tail -f ~/.local/share/duck-kanban/server.log | grep SOVEREIGN
```

**Common issues:**
1. `SOVEREIGN_STORAGE_ENABLED=false` - Set to `true`
2. Missing dependencies - Run `pip3 install nats-py cryptography`
3. Wrong password format - Check for special characters in `.env`
4. Database path wrong - Check `DATABASE_URL` in `.env`

### Sync Failing

**Check NATS connectivity:**
```bash
telnet nonlocal.info 4222
```

**Check provider is running:**
```bash
# On Pythia
/home/pythia/pcg-cc-mcp/manage_storage_provider.sh status
```

**Check dashboard logs:**
```bash
grep "SOVEREIGN_SYNC" ~/.local/share/duck-kanban/server.log
```

### Manual Test

Test sync without starting full dashboard:

```bash
# Create test config
mkdir -p ~/.config/pcg-dashboard
cat > ~/.config/pcg-dashboard/sovereign-sync.json <<EOF
{
  "enabled": true,
  "database_path": "/home/pythia/.local/share/duck-kanban/db.sqlite",
  "device_id": "c5ff23d2-1bdc-4479-b614-dfc103e8aa67",
  "provider_device_id": "d903c0e7-f3a6-4967-80e7-0da9d0fe7632",
  "encryption_password": "your_password_here",
  "sync_interval_minutes": 5,
  "nats_url": "nats://nonlocal.info:4222"
}
EOF

# Run sync service manually
python3 sovereign_storage/auto_sync_service.py
```

---

## 📁 Files Generated

When dashboard starts with auto-sync enabled:

```
~/.config/pcg-dashboard/
└── sovereign-sync.json  # Auto-generated from .env (chmod 600)
```

This config is created automatically - you don't need to create it manually.

---

## 🔄 Migration from Manual Setup

If you previously used the manual `setup-auto-sync.sh` script:

1. **Stop old systemd service:**
   ```bash
   systemctl --user stop pcg-auto-sync
   systemctl --user disable pcg-auto-sync
   ```

2. **Configure `.env`:**
   - Copy settings from `~/.config/pcg-dashboard/sovereign-sync.json`
   - Add to `.env` as shown above

3. **Start dashboard:**
   - Auto-sync now starts automatically!

4. **Clean up (optional):**
   ```bash
   systemctl --user daemon-reload
   rm ~/.config/systemd/user/pcg-auto-sync.service
   ```

---

## ✅ Summary

### Before (Manual)
- ❌ Separate auto-sync service to manage
- ❌ Systemd service to configure
- ❌ Manual startup required
- ❌ Separate logs to check

### After (Integrated)
- ✅ Built into dashboard backend
- ✅ Configure via `.env`
- ✅ Starts automatically with dashboard
- ✅ Logs in dashboard output

**Just edit `.env`, start the dashboard, and you're syncing!** 🚀

---

## 🎉 Next Steps

1. **Pull latest code:**
   ```bash
   cd /home/pythia/pcg-cc-mcp
   git pull
   ```

2. **Configure `.env`:**
   - Set `SOVEREIGN_STORAGE_ENABLED=true`
   - Add your device ID
   - Set encryption password

3. **Start dashboard:**
   ```bash
   ./start-all-services.sh
   ```

4. **Verify sync:**
   - Check dashboard logs for sync messages
   - Verify encrypted backup on Pythia

**You're now automatically syncing to the sovereign storage cloud!** ☁️

---

**Created:** 2026-02-09
**Status:** Production Ready
**Integration:** Built-in to dashboard backend v1.0+

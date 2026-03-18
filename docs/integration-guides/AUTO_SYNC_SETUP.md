# 🔄 Auto-Sync Setup Guide

Automatic database syncing from your local PCG Dashboard to Pythia storage provider.

---

## 🎯 Overview

The auto-sync service runs in the background and automatically syncs your PCG Dashboard database to the storage provider every 5 minutes (configurable). This ensures:

- ✅ Your data is always backed up
- ✅ Data is available when your device is offline
- ✅ No manual intervention needed
- ✅ Encrypted zero-knowledge backups

**Cost:** ~10.5 VIBE/month ($0.105) for 5GB

---

## 🚀 Quick Setup

### Step 1: Run Setup Script

```bash
cd /home/pythia/pcg-cc-mcp
./setup-auto-sync.sh
```

The script will ask you:

1. **Database path** - Default: `~/.local/share/duck-kanban/db.sqlite`
2. **Device ID** - Choose from:
   - Sirak Laptop: `c5ff23d2-1bdc-4479-b614-dfc103e8aa67`
   - Bonomotion Studio: `ac5eefa8-6ccb-4181-a72d-062be107c338`
   - Custom device
3. **Storage Provider** - Default: Pythia Master Node
4. **Encryption password** - Strong password for zero-knowledge encryption
5. **Sync interval** - Default: 5 minutes
6. **Enable now** - Start syncing immediately

### Step 2: Install Systemd Service (Optional)

The script will offer to install a systemd service for auto-start on boot:

```bash
# Enable service
systemctl --user enable pcg-auto-sync

# Start service
systemctl --user start pcg-auto-sync

# Check status
systemctl --user status pcg-auto-sync

# View logs
journalctl --user -u pcg-auto-sync -f
```

---

## 📋 Manual Configuration

If you prefer to configure manually:

### 1. Create Config File

```bash
mkdir -p ~/.config/pcg-dashboard
nano ~/.config/pcg-dashboard/sovereign-sync.json
```

### 2. Add Configuration

```json
{
  "enabled": true,
  "database_path": "~/.local/share/duck-kanban/db.sqlite",
  "device_id": "c5ff23d2-1bdc-4479-b614-dfc103e8aa67",
  "provider_device_id": "d903c0e7-f3a6-4967-80e7-0da9d0fe7632",
  "encryption_password": "your_strong_password_here",
  "sync_interval_minutes": 5,
  "nats_url": "nats://nonlocal.info:4222"
}
```

### 3. Secure the Config

```bash
chmod 600 ~/.config/pcg-dashboard/sovereign-sync.json
```

### 4. Run Auto-Sync

```bash
python3 ~/pcg-cc-mcp/sovereign_storage/auto_sync_service.py
```

---

## 🔧 Usage

### Start Auto-Sync (Manual)

```bash
python3 ~/pcg-cc-mcp/sovereign_storage/auto_sync_service.py
```

### Start Auto-Sync (Systemd)

```bash
systemctl --user start pcg-auto-sync
```

### Check Status

```bash
systemctl --user status pcg-auto-sync
```

### View Logs

```bash
# Real-time logs
journalctl --user -u pcg-auto-sync -f

# Last 50 lines
journalctl --user -u pcg-auto-sync -n 50
```

### Stop Auto-Sync

```bash
systemctl --user stop pcg-auto-sync
```

### Disable Auto-Start

```bash
systemctl --user disable pcg-auto-sync
```

---

## 🎛️ Configuration Options

Edit: `~/.config/pcg-dashboard/sovereign-sync.json`

| Option | Description | Default |
|--------|-------------|---------|
| `enabled` | Enable/disable auto-sync | `false` |
| `database_path` | Path to SQLite database | `~/.local/share/duck-kanban/db.sqlite` |
| `device_id` | Your device ID | (required) |
| `provider_device_id` | Storage provider device ID | `d903c0e7-f3a6-4967-80e7-0da9d0fe7632` |
| `encryption_password` | Zero-knowledge encryption password | (required) |
| `sync_interval_minutes` | Minutes between syncs | `5` |
| `nats_url` | NATS relay server URL | `nats://nonlocal.info:4222` |

**After editing config:**
```bash
systemctl --user restart pcg-auto-sync
```

---

## ✅ Verify It's Working

### 1. Check Service Status

```bash
systemctl --user status pcg-auto-sync
```

Should show: `Active: active (running)`

### 2. Check Logs for Successful Sync

```bash
journalctl --user -u pcg-auto-sync -n 20
```

Look for:
```
✅ Connected to nats://nonlocal.info:4222
📤 Starting sync to storage provider
✅ Sync confirmed by provider
✅ Sync complete!
```

### 3. Verify on Storage Provider (Pythia)

On Pythia server:
```bash
# Check for your encrypted backup
ls -lh /home/pythia/.sovereign_storage/

# View storage stats
/home/pythia/pcg-cc-mcp/manage_storage_provider.sh stats
```

You should see your device's encrypted backup file!

---

## 🔍 Troubleshooting

### Auto-Sync Not Starting

```bash
# Check if enabled in config
cat ~/.config/pcg-dashboard/sovereign-sync.json | grep enabled

# Check systemd status
systemctl --user status pcg-auto-sync

# View detailed logs
journalctl --user -u pcg-auto-sync -n 100
```

### Sync Failing

**Check dependencies:**
```bash
pip3 list | grep nats-py
pip3 list | grep cryptography
```

**Reinstall if needed:**
```bash
pip3 install --upgrade nats-py cryptography
```

### NATS Connection Failed

**Test connectivity:**
```bash
telnet nonlocal.info 4222
```

**Check provider is running:**
```bash
# On Pythia server
/home/pythia/pcg-cc-mcp/manage_storage_provider.sh status
```

### Wrong Password

If you entered the wrong encryption password, you need to:

1. Stop the service
2. Edit config with correct password
3. Restart service

```bash
systemctl --user stop pcg-auto-sync
nano ~/.config/pcg-dashboard/sovereign-sync.json
systemctl --user start pcg-auto-sync
```

---

## 💰 Cost Tracking

### Monitor Your Usage

The storage provider tracks your usage for billing:

**On Pythia:**
```bash
sqlite3 /home/pythia/.local/share/duck-kanban/db.sqlite \
  "SELECT * FROM storage_contracts WHERE client_device_id = 'YOUR_DEVICE_ID'"
```

**Expected cost for 5GB:**
- Storage: 10 VIBE/month ($0.10)
- Transfer: ~0.5 VIBE/month ($0.005)
- **Total: ~10.5 VIBE/month** ($0.105 or $1.26/year)

---

## 🔐 Security

### Encryption

- **Algorithm:** AES-256 (Fernet)
- **Key Derivation:** PBKDF2 with 100,000 iterations
- **Zero-Knowledge:** Storage provider cannot decrypt your data
- **Verification:** SHA-256 checksums ensure data integrity

### Password Security

**Your encryption password:**
- ⚠️ **Never shared** - Only stored locally on your device
- ⚠️ **Cannot be recovered** - If lost, encrypted backups cannot be decrypted
- ⚠️ **Stored in plaintext** in config file - Secure with `chmod 600`

**Best practices:**
- Use a strong, unique password
- Store backup copy in password manager
- Don't share with anyone (not even Pythia admin)

---

## 📊 Performance

### Sync Duration

Depends on database size:
- **1 MB:** ~1-2 seconds
- **5 MB:** ~2-5 seconds
- **10 MB:** ~5-10 seconds

### Network Usage

- **Encryption:** Adds ~5-10% overhead
- **Compression:** Not currently implemented
- **5 MB database:** Uses ~5.5 MB bandwidth per sync

### Resource Usage

- **CPU:** Minimal (encryption only during sync)
- **Memory:** ~30-50 MB
- **Disk:** Only encrypted backup on provider

---

## 🎉 Success!

Once configured, your PCG Dashboard data will be automatically:
- ✅ Backed up every 5 minutes
- ✅ Encrypted with zero-knowledge security
- ✅ Available when your device is offline
- ✅ Synced to Pythia storage provider

**You're now part of the sovereign storage ecosystem!** 🚀

---

**Created:** 2026-02-09
**Status:** Production Ready
**Cost:** ~10.5 VIBE/month ($0.105) for 5GB

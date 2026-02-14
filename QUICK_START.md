# ⚡ Quick Start Guide - Pythia Storage Provider

## 🎯 Current Status

**Storage Provider:** 🟢 ONLINE
**PID:** 2280852
**Device ID:** `d903c0e7-f3a6-4967-80e7-0da9d0fe7632`
**NATS Connection:** ✅ Connected to nonlocal.info:4222

---

## 📋 Essential Commands

### Manage Storage Provider

```bash
# Check status
/home/pythia/pcg-cc-mcp/manage_storage_provider.sh status

# View statistics
/home/pythia/pcg-cc-mcp/manage_storage_provider.sh stats

# View logs (follow mode)
/home/pythia/pcg-cc-mcp/manage_storage_provider.sh logs follow

# Restart if needed
/home/pythia/pcg-cc-mcp/manage_storage_provider.sh restart
```

### Monitor Activity

```bash
# Check running process
ps aux | grep storage_provider_server.py | grep -v grep

# View real-time logs
tail -f /var/tmp/pythia_storage_provider.log

# Check NATS connection
lsof -p 2280852 | grep ESTABLISHED
```

### Check Storage

```bash
# List stored replicas
ls -lh /home/pythia/.sovereign_storage/

# View device registry
python3 /home/pythia/pcg-cc-mcp/sovereign_storage/device_registry.py list admin

# View storage contracts
sqlite3 /home/pythia/.local/share/duck-kanban/db.sqlite "SELECT * FROM storage_contracts"
```

---

## 🚀 Next Steps

### 1. Sync from Sirak's Laptop

**Copy replication client to laptop:**
```bash
scp pythia@[pythia-ip]:/home/pythia/pcg-cc-mcp/sovereign_storage/storage_replication_client.py .
```

**Install dependencies on laptop:**
```bash
pip3 install nats-py cryptography
```

**Run sync from laptop:**
```bash
python3 storage_replication_client.py \
  ~/.local/share/duck-kanban/db.sqlite \
  c5ff23d2-1bdc-4479-b614-dfc103e8aa67 \
  d903c0e7-f3a6-4967-80e7-0da9d0fe7632 \
  [your_encryption_password]
```

**Expected output:**
```
✅ Connected to nats://nonlocal.info:4222
📤 Starting sync to storage provider
🔒 Encrypting database...
📡 Sending to storage provider...
✅ Sync confirmed by provider
✅ Sync complete!
```

### 2. Setup Bonomotion's Studio

1. Install PCG Dashboard on studio device
2. Register with device ID: `ac5eefa8-6ccb-4181-a72d-062be107c338`
3. Create projects
4. Add Sirak as collaborator

### 3. Test Collaboration

1. Bonomotion creates a project on studio device
2. Adds Sirak with editor role
3. Sirak accesses project over APN
4. Downloads files directly from studio device

---

## 💰 Revenue Tracking

**1 VIBE = $0.01 USD**

### For 1 Client (Sirak, 5GB)
- **Monthly:** ~16 VIBE ($0.16)
- **Annual:** ~192 VIBE ($1.92)

### Scale Projections
- **10 clients:** ~160 VIBE/month ($1.60)
- **100 clients:** ~1,600 VIBE/month ($16.00)
- **1,000 clients:** ~16,000 VIBE/month ($160.00)

---

## 🔧 Troubleshooting

### Storage Provider Won't Start

```bash
# Check Python version
python3 --version  # Should be 3.13.x

# Check dependencies
python3 -c "from nats.aio.client import Client; print('✅ nats-py OK')"
python3 -c "from cryptography.fernet import Fernet; print('✅ cryptography OK')"

# Reinstall if needed
python3 -m pip install nats-py cryptography

# Restart
/home/pythia/pcg-cc-mcp/manage_storage_provider.sh restart
```

### NATS Connection Lost

```bash
# Test connectivity
telnet nonlocal.info 4222

# Check DNS
ping nonlocal.info

# Restart storage provider
/home/pythia/pcg-cc-mcp/manage_storage_provider.sh restart
```

### Sync Fails from Client

**Check on Pythia:**
1. Storage provider running? `./manage_storage_provider.sh status`
2. NATS connected? Check status output
3. Firewall blocking? Check network

**Check on Client:**
1. Dependencies installed? `pip3 list | grep nats`
2. Device IDs correct? Check command
3. NATS accessible? `telnet nonlocal.info 4222`

---

## 📚 Documentation

| Document | Purpose |
|----------|---------|
| [SOVEREIGN_ARCHITECTURE.md](SOVEREIGN_ARCHITECTURE.md) | Overall system design |
| [STORAGE_PROVIDER_ARCHITECTURE.md](STORAGE_PROVIDER_ARCHITECTURE.md) | Technical architecture |
| [SOVEREIGN_DEPLOYMENT_GUIDE.md](SOVEREIGN_DEPLOYMENT_GUIDE.md) | Deployment instructions |
| [UPGRADE_COMPLETE.md](UPGRADE_COMPLETE.md) | Upgrade summary |
| [SYSTEM_STATUS.md](SYSTEM_STATUS.md) | Live system status |
| [QUICK_START.md](QUICK_START.md) | This file |

---

## ✅ Deployment Checklist

- [x] Database schema migrated
- [x] Three devices registered
- [x] Storage provider server started
- [x] NATS connection established
- [x] Management scripts created
- [x] Documentation complete
- [ ] First client sync completed
- [ ] Bonomotion's studio deployed
- [ ] Collaboration tested

---

## 🎉 You're Ready!

The sovereign storage system is **fully operational** and ready to:
- Accept encrypted backups from mobile devices
- Serve data when devices are offline
- Enable P2P collaboration
- Earn VIBE tokens for storage services

**Start earning VIBE now by syncing your first client!**

---

**System Administrator:** Pythia
**Storage Provider:** 🟢 ONLINE
**Last Updated:** 2026-02-09

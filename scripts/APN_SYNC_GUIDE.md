# ORCHA APN Multi-Device Sync Guide

**Purpose:** Run ORCHA Dashboard on multiple devices simultaneously with shared data
**Devices:** Space Terminal ↔ Pythia Master Node ↔ Bonomotion Mac Studio
**Protocol:** APN (Alpha Protocol Network)

---

## 🎯 What This Enables

✅ **Dashboard on multiple devices** - Access ORCHA from any machine
✅ **Shared project data** - All devices see the same projects and tasks
✅ **Automatic synchronization** - Changes sync across devices via APN
✅ **Distributed compute** - Tasks can execute on any device
✅ **Offline resilience** - Each device has full data copy

---

## 🏗️ Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                      APN Mesh Network                            │
│                                                                   │
│  ┌──────────────┐          ┌──────────────┐                     │
│  │   Pythia     │◄────────►│Space Terminal│                     │
│  │ Master Node  │          │    Relay     │                     │
│  │  (Primary)   │          │  (Secondary) │                     │
│  └───────┬──────┘          └──────┬───────┘                     │
│          │                         │                             │
│          │    ┌────────────────────┘                             │
│          │    │                                                  │
│          ▼    ▼                                                  │
│  ┌──────────────┐                                               │
│  │  Bonomotion  │                                               │
│  │ Mac Studio   │                                               │
│  │(Master Node) │                                               │
│  └──────────────┘                                               │
│                                                                   │
│  All devices share:                                              │
│  • Database (projects, tasks, users, VIBE balances)             │
│  • Project files (~/topos directory)                            │
│  • Execution history and logs                                   │
└─────────────────────────────────────────────────────────────────┘
```

---

## 📋 Prerequisites

### On ALL Devices:

1. **ORCHA installed:**
   ```bash
   cd ~/pcg-cc-mcp
   git pull origin bonomotion  # or main
   cargo build --release
   ```

2. **Network connectivity:**
   - All devices on same network (LAN/VPN)
   - OR APN mesh network established
   - SSH access between devices (for rsync fallback)

3. **Sufficient storage:**
   - Database: ~5-10MB
   - Projects: ~1-5GB (varies)
   - Total recommended: 10GB+ free space

---

## 🚀 Setup Process

### Step 1: Initial Setup on Each Device

Run on **Space Terminal:**
```bash
cd ~/pcg-cc-mcp
chmod +x scripts/apn_sync_setup.sh
scripts/apn_sync_setup.sh
```

Run on **Pythia:**
```bash
cd ~/pcg-cc-mcp
chmod +x scripts/apn_sync_setup.sh
scripts/apn_sync_setup.sh
```

Run on **Bonomotion:**
```bash
cd ~/pcg-cc-mcp
chmod +x scripts/apn_sync_setup.sh
scripts/apn_sync_setup.sh
```

**What this does:**
- ✅ Registers device in ORCHA network
- ✅ Creates sync configuration
- ✅ Sets up sync daemon
- ✅ Configures auto-start service

---

### Step 2: Initial Data Sync

**Choose Primary Source** (device with most complete data - likely Space Terminal):

**On Space Terminal (source):**
```bash
cd ~/pcg-cc-mcp
chmod +x scripts/apn_manual_sync.sh

# Sync database to Pythia
scripts/apn_manual_sync.sh spaceterminal pythia database

# Sync projects to Pythia
scripts/apn_manual_sync.sh spaceterminal pythia projects

# Sync to Bonomotion
scripts/apn_manual_sync.sh spaceterminal bonomotion-mac-studio database
scripts/apn_manual_sync.sh spaceterminal bonomotion-mac-studio projects
```

**Alternative: Using rsync directly**
```bash
# From Space Terminal to Pythia
rsync -avz --progress ~/pcg-cc-mcp/dev_assets/db.sqlite pythia:~/pcg-cc-mcp/dev_assets/
rsync -avz --progress --exclude='node_modules' --exclude='target' ~/topos/ pythia:~/topos/

# From Space Terminal to Bonomotion
rsync -avz --progress ~/pcg-cc-mcp/dev_assets/db.sqlite bonomotion:~/pcg-cc-mcp/dev_assets/
rsync -avz --progress --exclude='node_modules' --exclude='target' ~/topos/ bonomotion:~/topos/
```

---

### Step 3: Start Sync Daemon

**On ALL devices:**

**Linux (Pythia, Bonomotion if Linux):**
```bash
sudo systemctl start orcha-apn-sync
sudo systemctl status orcha-apn-sync
```

**macOS (Space Terminal, Bonomotion if macOS):**
```bash
launchctl start com.orcha.apn-sync
launchctl list | grep orcha
```

**Manual (if systemd/launchd not available):**
```bash
cd ~/pcg-cc-mcp
nohup scripts/apn_sync_daemon.sh > logs/apn-sync.log 2>&1 &
```

---

### Step 4: Start Dashboard on ALL Devices

**On each device:**
```bash
cd ~/pcg-cc-mcp

# Stop existing server
pkill -f "target/release/server"

# Start server
RUST_LOG=info ./target/release/server &

# Or with custom port if needed
RUST_LOG=info PORT=8080 ./target/release/server &
```

**Access dashboard:**
- Space Terminal: `http://localhost:3000` or `http://spaceterminal:3000`
- Pythia: `http://localhost:3000` or `http://pythia:3000`
- Bonomotion: `http://localhost:3000` or `http://bonomotion:3000`

---

## ✅ Verification

### Check Sync Status

**View sync logs:**
```bash
tail -f ~/pcg-cc-mcp/logs/apn_sync.log
```

**Check device registration:**
```bash
sqlite3 ~/pcg-cc-mcp/dev_assets/db.sqlite "
SELECT id, hostname, device_tier, is_online
FROM devices
WHERE deleted_at IS NULL;
"
```

Expected output (on all devices):
```
pythia|pythia|master_node|1
spaceterminal|spaceterminal|relay|1
bonomotion-mac-studio|bonomotion-mac-studio|master_node|1
```

**Check project counts match:**
```bash
sqlite3 ~/pcg-cc-mcp/dev_assets/db.sqlite "
SELECT COUNT(*) as projects FROM projects WHERE deleted_at IS NULL;
"
```

Should be same number on all devices (e.g., 32).

**Check topos directory:**
```bash
ls -la ~/topos/
```

Should see same project directories on all devices.

---

## 🔄 How Sync Works

### Continuous Sync (Every 5 minutes):

1. **Database Sync:**
   - Master (Pythia) broadcasts database changes
   - Secondaries (Space Terminal, Bonomotion) pull changes
   - Conflict resolution: Master wins

2. **File Sync:**
   - Checksum comparison for each project
   - Newest file wins (by modification time)
   - Bidirectional - any device can update

3. **Conflict Resolution:**
   - Database: Master (Pythia) is source of truth
   - Files: Newest modification wins
   - Manual conflicts logged for review

---

## 🧪 Testing

### Test 1: Create Project on One Device

**On Space Terminal:**
```bash
# Create test project in database
sqlite3 ~/pcg-cc-mcp/dev_assets/db.sqlite "
INSERT INTO projects (id, name, git_repo_path, owner_id)
VALUES (
    randomblob(16),
    'Test Sync Project',
    '/home/spaceterminal/topos/test-sync',
    (SELECT id FROM users WHERE username = 'admin')
);
"
```

**Wait 5 minutes, then check Pythia:**
```bash
sqlite3 ~/pcg-cc-mcp/dev_assets/db.sqlite "
SELECT name FROM projects WHERE name = 'Test Sync Project';
"
```

Should see the project.

### Test 2: Modify File on One Device

**On Pythia:**
```bash
echo "# Test file" > ~/topos/README.md
```

**Wait 5 minutes, check Space Terminal:**
```bash
cat ~/topos/README.md
```

Should see the same content.

### Test 3: Dashboard Access

**Open dashboard on all three devices simultaneously:**
- Sign in as Admin on all three
- Create a task on Space Terminal
- Wait 5 minutes
- Check if task appears on Pythia and Bonomotion

---

## 🛠️ Troubleshooting

### Issue: Sync daemon not running

**Check status:**
```bash
# Linux
sudo systemctl status orcha-apn-sync

# macOS
launchctl list | grep orcha

# Manual
ps aux | grep apn_sync_daemon
```

**Restart:**
```bash
# Linux
sudo systemctl restart orcha-apn-sync

# macOS
launchctl stop com.orcha.apn-sync
launchctl start com.orcha.apn-sync

# Manual
pkill -f apn_sync_daemon
nohup ~/pcg-cc-mcp/scripts/apn_sync_daemon.sh > ~/pcg-cc-mcp/logs/apn-sync.log 2>&1 &
```

### Issue: Devices can't communicate

**Test network connectivity:**
```bash
# Can devices reach each other?
ping pythia
ping spaceterminal
ping bonomotion-mac-studio

# Can SSH connect?
ssh pythia "echo connected"
ssh spaceterminal "echo connected"
```

**Check APN network:**
```bash
# If using APN protocol
apn-ping pythia
apn-status
```

### Issue: Database out of sync

**Force full database sync:**
```bash
# On master (Pythia), export
sqlite3 ~/pcg-cc-mcp/dev_assets/db.sqlite .dump > /tmp/db_master.sql

# Transfer to secondary
scp /tmp/db_master.sql spaceterminal:/tmp/

# On secondary, import
mv ~/pcg-cc-mcp/dev_assets/db.sqlite ~/pcg-cc-mcp/dev_assets/db.sqlite.old
sqlite3 ~/pcg-cc-mcp/dev_assets/db.sqlite < /tmp/db_master.sql
```

### Issue: Project files missing on one device

**Manual rsync:**
```bash
# From Space Terminal (has files) to Pythia (needs files)
rsync -avz --progress \
    --exclude='node_modules' \
    --exclude='target' \
    --exclude='.git/objects' \
    spaceterminal:~/topos/ ~/topos/
```

---

## 📊 Monitoring

### View Sync Activity

**Real-time logs:**
```bash
tail -f ~/pcg-cc-mcp/logs/apn_sync.log
```

**Sync statistics:**
```bash
# Number of sync cycles today
grep "Starting sync cycle" ~/pcg-cc-mcp/logs/apn_sync.log | \
    grep "$(date +%Y-%m-%d)" | wc -l

# Recent sync errors
grep -i "error\|failed" ~/pcg-cc-mcp/logs/apn_sync.log | tail -10

# Last successful sync
grep "Sync cycle complete" ~/pcg-cc-mcp/logs/apn_sync.log | tail -1
```

---

## 🎯 Best Practices

1. **Designate one Master device** (recommend Pythia)
   - Master has priority in conflicts
   - Run most critical services on Master

2. **Regular backups:**
   ```bash
   # Daily backup of database
   cp ~/pcg-cc-mcp/dev_assets/db.sqlite \
      ~/pcg-cc-mcp/dev_assets/db.sqlite.backup.$(date +%Y%m%d)
   ```

3. **Monitor sync health:**
   - Check logs daily
   - Verify device connectivity
   - Test after network changes

4. **Staged rollouts:**
   - Test changes on one device first
   - Wait for sync
   - Verify on other devices before widespread use

5. **Conflict prevention:**
   - Avoid editing same files simultaneously on multiple devices
   - Use git for code projects (automatic conflict resolution)
   - Database changes best made on Master

---

## 🔐 Security Considerations

1. **Encrypted transfer:** APN should use encryption (configure in apn_sync_config.json)
2. **Authentication:** Ensure SSH keys or APN auth tokens configured
3. **Access control:** Dashboard authentication protects data at rest
4. **Network isolation:** Use VPN if devices on different networks

---

## ✅ Success Criteria

**Sync is working when:**
- ✅ All devices show same project count
- ✅ Creating project on one device appears on others within 5 minutes
- ✅ Dashboard accessible on all devices
- ✅ Task execution works on any device
- ✅ VIBE balances consistent across devices
- ✅ Sync logs show regular successful cycles

---

## 📝 Configuration Files

**Sync config:** `~/pcg-cc-mcp/apn_sync_config.json`
**Sync daemon:** `~/pcg-cc-mcp/scripts/apn_sync_daemon.sh`
**Manual sync:** `~/pcg-cc-mcp/scripts/apn_manual_sync.sh`
**Sync logs:** `~/pcg-cc-mcp/logs/apn_sync.log`

---

**Status: READY FOR MULTI-DEVICE DEPLOYMENT** 🚀

Run setup scripts on all devices and enjoy synchronized ORCHA experience!

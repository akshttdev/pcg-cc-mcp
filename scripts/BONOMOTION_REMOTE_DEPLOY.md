# Bonomotion Device Remote Deployment

**Date:** February 16, 2026
**Branch:** bonomotion
**Status:** Ready for Remote Deployment via APN

---

## 🎯 Architecture Correction

**Each user sees ONLY their own projects (no sharing):**
- **Admin**: 32 projects (ORCHA, Powerclub Global, etc.) - PRIVATE
- **Bonomotion**: 1 project (local) - PRIVATE
- **Sirak**: 1 project - PRIVATE

**No cross-tenant visibility unless explicitly shared via project_members table.**

---

## 📦 What's in This Branch

### Code Changes (Already Pushed)
- ✅ `src-tauri/src/topology.rs` - Topology-aware access control
- ✅ `src-tauri/src/remote_execution.rs` - Remote execution service
- ✅ `scripts/fix_database.sh` - Database fix script
- ✅ `scripts/BONOMOTION_REMOTE_DEPLOY.md` - This file

### Database Changes (NOT in Git)
The database file (`dev_assets/db.sqlite`) is in `.gitignore` and must be transferred separately via APN.

---

## 🚀 Deployment Steps on Bonomotion Device

### Step 1: Pull Latest Code

```bash
cd ~/pcg-cc-mcp
git fetch origin
git checkout bonomotion
git pull origin bonomotion
```

### Step 2: Run Database Fix Script

```bash
chmod +x ~/pcg-cc-mcp/scripts/fix_database.sh
~/pcg-cc-mcp/scripts/fix_database.sh
```

This script will:
- ✅ Backup existing database
- ✅ Fix corrupted UUIDs (Sirak, Bonomotion)
- ✅ Create 6 new tables (devices, nodes, project_members, task_executions, vibe_ledger, apn_cloud_capacity)
- ✅ Initialize VIBE balances (Admin: 1000, Bonomotion: 100, Sirak: 50)
- ✅ Register Bonomotion Mac Studio as Master Node
- ✅ **NO PROJECT SHARING** - Each user keeps their own projects private

### Step 3: Rebuild Rust Backend (if needed)

```bash
cd ~/pcg-cc-mcp
cargo build --release
```

### Step 4: Restart ORCHA Server

```bash
pkill -f "target/release/server"
RUST_LOG=info ./target/release/server &
```

---

## ✅ Expected Results After Deployment

### Authentication Test

**Bonomotion:**
- Sign in: `bonomotion@powerclubglobal.com`
- Should see: **0 or 1 projects** (only Bonomotion's local project if it exists)
- Should NOT see: Admin's ORCHA, Powerclub Global, or any other Admin projects

**Sirak:**
- Sign in: `sirak@powerclubglobal.com`
- Should see: **1 project** (Sirak's own project)

**Admin:**
- Sign in: `admin@powerclubglobal.com`
- Should see: **32 projects** (all Admin's projects including ORCHA, Powerclub Global)

### Database State

```bash
sqlite3 ~/pcg-cc-mcp/dev_assets/db.sqlite "
SELECT hex(id), username, vibe_balance FROM users;
"
```

Expected:
```
F8EB8F0268963FDB698AB9DFE836E7BE|admin|1000.0
82EF3C4E943C4678925180629208A183|Bonomotion|100.0
93EE4745203B4FED8F9184315E5C3B3B|Sirak|50.0
```

### Master Node Registration

```bash
sqlite3 ~/pcg-cc-mcp/dev_assets/db.sqlite "
SELECT id, hostname, total_cores, total_ram_gb
FROM devices
WHERE owner_id = X'82EF3C4E943C4678925180629208A183';
"
```

Expected:
```
bonomotion-mac-studio|bonomotion-mac-studio|16|128
```

### Project Ownership (No Sharing)

```bash
sqlite3 ~/pcg-cc-mcp/dev_assets/db.sqlite "
SELECT u.username, COUNT(p.id) as owned_projects
FROM users u
LEFT JOIN projects p ON p.owner_id = u.id AND p.deleted_at IS NULL
GROUP BY u.username;
"
```

Expected:
```
admin|32
Bonomotion|0 or 1 (depends on local project)
Sirak|1
```

---

## 🔧 Troubleshooting

### Issue: "Database is locked"
```bash
pkill -f "target/release/server"
sleep 5
~/pcg-cc-mcp/scripts/fix_database.sh
```

### Issue: Still seeing UUID errors
```bash
sqlite3 ~/pcg-cc-mcp/dev_assets/db.sqlite "
SELECT hex(id), username FROM users;
"
```

UUIDs should be 32 hex chars (16 bytes), NOT long double-encoded strings.

### Issue: Bonomotion sees Admin's projects
This means topology filtering is not working. Check:
1. Are you signed in as the correct user?
2. Did the database fix script run successfully?
3. Are the topology.rs API endpoints being used?

---

## 📋 APN Transfer (Alternative to Fix Script)

If you prefer to transfer the entire fixed database via APN:

### On Pythia (Source):
```bash
# Create database export for APN transfer
cd ~/pcg-cc-mcp/dev_assets
gzip -c db.sqlite > db.sqlite.gz

# Transfer via APN (adjust for your APN protocol)
# Example: apn-send --target bonomotion-mac-studio --file db.sqlite.gz
```

### On Bonomotion (Target):
```bash
# Receive via APN
cd ~/pcg-cc-mcp/dev_assets
cp db.sqlite db.sqlite.backup.$(date +%s)

# Extract received database
gunzip -c db.sqlite.gz > db.sqlite

# Restart server
pkill -f "target/release/server"
RUST_LOG=info ./target/release/server &
```

---

## 🎯 Success Criteria

✅ Bonomotion can sign in without UUID errors
✅ Bonomotion sees ONLY own projects (0 or 1)
✅ Bonomotion does NOT see Admin's ORCHA or Powerclub Global
✅ Sirak sees only 1 project (own)
✅ Admin sees 32 projects (all own)
✅ Bonomotion Mac Studio registered as Master Node
✅ VIBE balance initialized (100.00 for Bonomotion)
✅ Tasks can execute on Bonomotion Mac Studio for FREE (0.00 VIBE)

---

## 📝 Notes on Bonomotion's Local Project

Bonomotion has 1 project that exists only on the local machine. This project:
- May be preserved if the fix script is run (it fixes UUIDs but doesn't delete projects)
- May be lost if the database is replaced entirely
- Can be imported later from the old dashboard if needed

**Decision:** Run fix script (preserves local projects) rather than replacing database entirely.

---

**Status: READY FOR REMOTE DEPLOYMENT via APN** 🚀

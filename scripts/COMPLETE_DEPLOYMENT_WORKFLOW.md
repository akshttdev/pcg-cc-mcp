# Complete Deployment Workflow - Bonomotion Device

**Branch:** bonomotion
**Status:** Ready for Remote Deployment
**Date:** February 16, 2026

---

## 🎯 Overview

This is the complete end-to-end deployment workflow for deploying ORCHA multi-tenant architecture to the Bonomotion Mac Studio device remotely.

**Architecture:**
- ✅ Multi-tenant with strict privacy boundaries
- ✅ Topology-aware access control
- ✅ Remote execution routing (Master Node = FREE)
- ✅ VIBE token economics
- ✅ Each user sees ONLY their own projects

---

## 📋 Pre-Deployment Checklist

**On Pythia (Already Complete):**
- ✅ Code pushed to GitHub (bonomotion branch)
- ✅ 4 commits: 5093b23, 8da25fb, f6f7618, 6c25718
- ✅ Rust modules implemented (topology.rs, remote_execution.rs)
- ✅ Database fix script created
- ✅ Deployment safety scripts created
- ✅ APN transfer package ready (8KB)

**On Bonomotion (To Be Done):**
- ⏳ Pull latest code from bonomotion branch
- ⏳ Run database fix script
- ⏳ Rebuild Rust backend
- ⏳ Restart ORCHA server
- ⏳ Verify deployment

---

## 🚀 DEPLOYMENT WORKFLOW

### Phase 1: Pull Code (2 minutes)

**On Bonomotion device:**

```bash
cd ~/pcg-cc-mcp
git fetch origin
git checkout bonomotion
git pull origin bonomotion
```

**Verify files downloaded:**
```bash
ls -l scripts/
# Should see:
# - fix_database.sh
# - verify_deployment.sh
# - rollback_deployment.sh
# - health_check.sh
# - create_apn_transfer_package.sh
# - BONOMOTION_REMOTE_DEPLOY.md
```

**Verify Rust modules:**
```bash
ls -l src-tauri/src/
# Should see:
# - topology.rs
# - remote_execution.rs
```

---

### Phase 2: Stop Server (1 minute)

```bash
pkill -f "target/release/server"
sleep 2
```

**Verify server stopped:**
```bash
pgrep -f "target/release/server" || echo "Server stopped successfully"
```

---

### Phase 3: Run Database Fix (3 minutes)

```bash
chmod +x ~/pcg-cc-mcp/scripts/fix_database.sh
~/pcg-cc-mcp/scripts/fix_database.sh
```

**What this does:**
- ✅ Creates automatic backup (db.sqlite.backup.TIMESTAMP)
- ✅ Fixes corrupted UUIDs (Sirak, Bonomotion)
- ✅ Creates 6 new tables
- ✅ Initializes VIBE balances
- ✅ Registers Bonomotion Mac Studio as Master Node
- ✅ Preserves Bonomotion's existing local project
- ✅ NO project sharing (privacy enforced)

**Expected output:**
```
=== USERS (Fixed UUIDs) ===
82EF3C4E943C4678925180629208A183|Bonomotion|100.0
93EE4745203B4FED8F9184315E5C3B3B|Sirak|50.0
F8EB8F0268963FDB698AB9DFE836E7BE|admin|1000.0

=== BONOMOTION DEVICE ===
bonomotion-mac-studio|bonomotion-mac-studio|master_node|16|128

=== PROJECT OWNERSHIP (No Sharing) ===
admin|32
Bonomotion|0 or 1
Sirak|1

✅ DATABASE FIX COMPLETE
```

---

### Phase 4: Rebuild Rust Backend (5-10 minutes)

```bash
cd ~/pcg-cc-mcp
cargo build --release
```

**This compiles:**
- ✅ topology.rs (access control)
- ✅ remote_execution.rs (execution routing)
- ✅ All dependencies

**Expected:** Build completes without errors

---

### Phase 5: Restart Server (1 minute)

```bash
cd ~/pcg-cc-mcp
RUST_LOG=info ./target/release/server &
```

**Verify server started:**
```bash
sleep 5
pgrep -f "target/release/server" && echo "✅ Server running"
```

---

### Phase 6: Verify Deployment (2 minutes)

```bash
chmod +x ~/pcg-cc-mcp/scripts/verify_deployment.sh
~/pcg-cc-mcp/scripts/verify_deployment.sh
```

**This runs 7 tests:**
1. ✅ UUID Integrity (32 hex chars)
2. ✅ VIBE Balance Initialization (Bonomotion: 100 VIBE)
3. ✅ Database Schema (6 new tables)
4. ✅ Master Node Registration (bonomotion-mac-studio)
5. ✅ Multi-Tenant Privacy (no cross-tenant sharing)
6. ✅ Project Ownership Distribution
7. ✅ ORCHA Server Status

**Expected output:**
```
Tests Passed: 7
Tests Failed: 0

✅ ALL TESTS PASSED - DEPLOYMENT SUCCESSFUL
```

---

### Phase 7: Manual Testing (5 minutes)

**Open browser and test authentication:**

1. **Sign in as Bonomotion:**
   - Email: `bonomotion@powerclubglobal.com`
   - Password: (existing password)
   - Expected: ✅ Clean sign-in, no UUID errors
   - Expected: ✅ See 0-1 projects (own local project only)
   - Expected: ✅ Do NOT see Admin's ORCHA or Powerclub Global

2. **Sign in as Admin (if accessible):**
   - Email: `admin@powerclubglobal.com`
   - Expected: ✅ See 32 projects
   - Expected: ✅ Do NOT see Bonomotion's project

3. **Sign in as Sirak (if accessible):**
   - Email: `sirak@powerclubglobal.com`
   - Expected: ✅ See 1 project (own)
   - Expected: ✅ Do NOT see Admin's or Bonomotion's projects

---

### Phase 8: Test Remote Execution (5 minutes)

**Create and execute a test task:**

1. Sign in as Bonomotion
2. Navigate to ORCHA dashboard (if have access) or create test task
3. Create new task: "Test Execution - Bonomotion"
4. Execute task

**Expected:**
- ✅ Task routes to Bonomotion Mac Studio (Master Node)
- ✅ Execution cost: **0.00 VIBE** (FREE - own hardware)
- ✅ Task completes successfully
- ✅ VIBE balance unchanged (100.00)

---

## ✅ SUCCESS CRITERIA

**Deployment is successful when ALL criteria are met:**

### Authentication
- ✅ Bonomotion signs in without UUID errors
- ✅ Sirak signs in without UUID errors
- ✅ Admin signs in without UUID errors

### Privacy
- ✅ Bonomotion sees 0-1 projects (own only)
- ✅ Bonomotion does NOT see Admin's ORCHA
- ✅ Bonomotion does NOT see Admin's Powerclub Global
- ✅ Each user's projects are isolated

### Database
- ✅ All UUIDs are 32 hex chars (fixed)
- ✅ 6 new tables exist
- ✅ VIBE balances initialized
- ✅ Bonomotion Mac Studio registered as Master Node

### Execution
- ✅ Tasks execute on Bonomotion Mac Studio
- ✅ Execution cost: 0.00 VIBE (free)
- ✅ Server running and responsive

---

## 🔧 TROUBLESHOOTING

### Issue: Deployment verification fails

**Solution:**
```bash
# Check which tests failed
~/pcg-cc-mcp/scripts/verify_deployment.sh

# If UUID test fails - rerun fix script
~/pcg-cc-mcp/scripts/fix_database.sh

# If server test fails - restart server
pkill -f server
RUST_LOG=info ~/pcg-cc-mcp/target/release/server &
```

### Issue: Server won't start

**Solution:**
```bash
# Check logs
tail -100 ~/pcg-cc-mcp/orcha.log

# Check if port is in use
lsof -i :8080  # or your configured port

# Try rebuilding
cd ~/pcg-cc-mcp
cargo clean
cargo build --release
RUST_LOG=info ./target/release/server &
```

### Issue: Still seeing corrupted UUIDs

**Solution:**
```bash
# Check current UUIDs
sqlite3 ~/pcg-cc-mcp/dev_assets/db.sqlite "
SELECT hex(id), username FROM users;
"

# If still corrupted (>32 chars), check fix script ran
ls -l ~/pcg-cc-mcp/dev_assets/db.sqlite.backup.*

# Rerun fix if needed
~/pcg-cc-mcp/scripts/fix_database.sh
```

### Issue: Bonomotion sees Admin's projects

**Solution:**
```bash
# Check project sharing
sqlite3 ~/pcg-cc-mcp/dev_assets/db.sqlite "
SELECT COUNT(*) FROM project_members
WHERE user_id = X'82EF3C4E943C4678925180629208A183';
"

# Should be 0 - if not, clear incorrect sharing
sqlite3 ~/pcg-cc-mcp/dev_assets/db.sqlite "
DELETE FROM project_members
WHERE user_id = X'82EF3C4E943C4678925180629208A183';
"

# Restart server
pkill -f server
RUST_LOG=info ~/pcg-cc-mcp/target/release/server &
```

---

## 🔄 ROLLBACK PROCEDURE

**If deployment fails or causes issues:**

```bash
# Run rollback script
chmod +x ~/pcg-cc-mcp/scripts/rollback_deployment.sh
~/pcg-cc-mcp/scripts/rollback_deployment.sh

# Follow prompts to confirm rollback
# This will restore database to pre-deployment state
```

**Rollback features:**
- ✅ Restores most recent backup automatically
- ✅ Creates backup of current state before rollback
- ✅ Stops server gracefully
- ✅ Provides git rollback instructions

---

## 📊 MONITORING

**Run health check anytime:**

```bash
~/pcg-cc-mcp/scripts/health_check.sh
```

**This shows:**
- 📁 Database status
- 🖥️ Server status and uptime
- 👥 User status (UUIDs, VIBE balances)
- 🔧 Master Node registration
- 📂 Project ownership distribution
- ⚡ Recent task executions
- 💰 Recent VIBE transactions
- 📊 Database schema completeness

**Run periodically:**
- After deployment
- Before/after major operations
- When investigating issues
- Daily for production monitoring

---

## 📝 POST-DEPLOYMENT NOTES

### Bonomotion's Local Project
- The fix script **preserves** existing projects
- If Bonomotion had a local project, it remains after deployment
- Can be imported from old dashboard later if needed

### Future Project Import
- Bonomotion's old dashboard project can be imported separately
- No urgency - system is functional without it
- Import procedure can be developed as needed

### Project Sharing (If Needed Later)
To share projects between users in the future:
```sql
-- Example: Share ORCHA project from Admin to Bonomotion as Viewer
INSERT INTO project_members (id, project_id, user_id, role, granted_by)
VALUES (
    randomblob(16),
    (SELECT id FROM projects WHERE name = 'ORCHA'),
    X'82EF3C4E943C4678925180629208A183',
    'viewer',
    X'F8EB8F0268963FDB698AB9DFE836E7BE'
);
```

---

## 🎯 TIMELINE

**Total deployment time: ~15-25 minutes**

- Phase 1: Pull Code (2 min)
- Phase 2: Stop Server (1 min)
- Phase 3: Database Fix (3 min)
- Phase 4: Rebuild (5-10 min)
- Phase 5: Restart Server (1 min)
- Phase 6: Verification (2 min)
- Phase 7: Manual Testing (5 min)
- Phase 8: Execution Test (5 min)

---

## ✅ COMPLETION CHECKLIST

**Mark each as complete:**

- [ ] Code pulled from bonomotion branch
- [ ] fix_database.sh executed successfully
- [ ] Rust backend rebuilt
- [ ] Server restarted
- [ ] verify_deployment.sh passed all 7 tests
- [ ] Bonomotion signed in successfully
- [ ] Bonomotion sees only own projects (0-1)
- [ ] Test task executed on Master Node (0.00 VIBE)
- [ ] health_check.sh shows system healthy

**When all checked:**
🎉 **DEPLOYMENT COMPLETE - SYSTEM OPERATIONAL**

---

## 📞 SUPPORT

**Available scripts:**
- `scripts/fix_database.sh` - Apply database fixes
- `scripts/verify_deployment.sh` - Verify deployment success
- `scripts/rollback_deployment.sh` - Rollback if needed
- `scripts/health_check.sh` - System health monitoring
- `scripts/BONOMOTION_REMOTE_DEPLOY.md` - Full deployment guide

**Backup locations:**
- Automatic backups: `~/pcg-cc-mcp/dev_assets/db.sqlite.backup.*`
- Latest backup used for rollback

---

**Status: READY FOR DEPLOYMENT** 🚀

Follow phases 1-8 above for complete end-to-end deployment.

# APN Peer Deduplication & Identity Persistence Patch

**Date:** 2026-02-13
**Version:** Dashboard 1.0 + APN Core Rust 1.0
**Status:** ✅ Deployed

---

## Problem

Devices were creating **duplicate peer entries** in the APN network due to:

1. **No identity persistence** - Rust APN nodes regenerated new identities on each restart
2. **Database pollution** - Same device appearing multiple times with different node IDs
3. **Lost VIBE rewards** - Rewards spread across multiple wallets for same device

### Example (RTX 3070 Laptop)

| Node ID | Last Heartbeat | Status |
|---------|----------------|--------|
| apn_b2664a6a | 2026-02-08 18:45 | Active ✅ |
| apn_6cc33305 | 2026-02-08 17:26 | Duplicate ❌ |
| apn_83e0fa39 | 2026-02-07 03:02 | Duplicate ❌ |

**Same device, 3 different identities!**

---

## Solution

### Part 1: Dashboard Automatic Cleanup

**Files Modified:**
- `crates/db/src/models/peer_node.rs` - Added deduplication methods
- `crates/server/src/main.rs` - Added background cleanup task

**New Features:**

#### 1. Hardware Fingerprint Deduplication
```rust
pub async fn cleanup_duplicates(pool: &SqlitePool) -> Result<i64, sqlx::Error>
```
- Identifies duplicates by matching: CPU cores + RAM + GPU model
- Marks older entries as inactive, keeps most recent
- Runs automatically every 60 seconds

#### 2. Stale Peer Cleanup
```rust
pub async fn mark_stale_inactive(pool: &SqlitePool) -> Result<i64, sqlx::Error>
```
- Marks peers inactive if no heartbeat in 5+ minutes
- Prevents zombie nodes from cluttering the network
- Runs automatically every 60 seconds

#### 3. Background Service
```rust
// In main.rs - Spawns cleanup task on server startup
tokio::spawn(async move {
    let mut interval = tokio::time::interval(Duration::from_secs(60));
    loop {
        interval.tick().await;
        PeerNode::cleanup_duplicates(&pool).await;
        PeerNode::mark_stale_inactive(&pool).await;
    }
});
```

---

### Part 2: Rust APN Identity Persistence

**Files Created:**
- `crates/alpha-protocol-core/src/identity_storage.rs` - **NEW** - Identity file persistence

**Files Modified:**
- `crates/alpha-protocol-core/src/lib.rs` - Export identity_storage module
- `crates/alpha-protocol-core/src/bin/apn_node.rs` - Use persistent identities

**New Behavior:**

#### Automatic Identity Persistence
```rust
pub fn load_or_create_identity(import_mnemonic: Option<&str>) -> Result<NodeIdentity>
```

**On First Run:**
```
⚠️  Generated NEW identity: apn_814d37f4
⚠️  Wallet address: 0x814d37f4ab0c7ef2...
⚠️  This identity will be saved and reused on restart
✓ Saved identity to /home/user/.apn/node_identity.json
```

**On Subsequent Runs:**
```
Loading existing identity from /home/user/.apn/node_identity.json
✓ Loaded existing node identity: apn_814d37f4
✓ Wallet address: 0x814d37f4ab0c7ef2...
✓ Identity file: /home/user/.apn/node_identity.json
```

#### Robust Error Handling
- **File missing** → Generate new (expected behavior)
- **File corrupted** → FAIL LOUDLY with helpful error (prevents wallet loss)
- **Permission denied** → FAIL LOUDLY
- **Invalid mnemonic** → FAIL LOUDLY

#### Security Features
- Identity directory: `~/.apn/` with permissions `0o700` (owner only)
- Identity file: `node_identity.json` with permissions `0o600` (owner read/write only)
- Automatic backup: `.json.backup` created before modifications
- Verification after save: Ensures file is readable and correct

#### File Format
```json
{
  "node_id": "apn_814d37f4",
  "wallet_address": "0x814d37f4ab0c7ef2d9c7aa5f3de9fc5c0f79b3b576b77f84edc9e773eed71c92",
  "mnemonic": "slush index float shaft flight citizen swear chunk correct veteran eyebrow blind",
  "public_key": "a1b2c3d4..."
}
```

---

## Deployment

### Prerequisites
- Rust toolchain
- SQLite database
- NATS relay at `nats://nonlocal.info:4222`

### Build
```bash
cd /home/pythia/pcg-cc-mcp
cargo build --release --bin server --bin apn_node
```

### Services
```bash
# Dashboard with cleanup
sudo systemctl restart pcg-unified

# APN node with persistence
sudo systemctl restart apn-master-node
```

---

## Migration

### For Existing Deployments

**Dashboard (Server):**
- ✅ Automatic - cleanup runs on server startup
- ✅ Backwards compatible - existing data unaffected
- ✅ No downtime required

**APN Nodes:**
- ✅ Automatic identity saving on first run with new binary
- ✅ Existing nodes will save their current identity
- ✅ Future restarts will reuse saved identity

### For Devices with Duplicates

**Option 1: Let cleanup handle it**
- Dashboard will automatically mark old duplicates as inactive
- Latest identity becomes the canonical one

**Option 2: Manual consolidation**
```bash
# Find all identities for a device
ls -la ~/.apn/

# Choose the one with most VIBE rewards
# Copy it to standard location
cp ~/.apn/node_identity_backup_20260208.json ~/.apn/node_identity.json

# Restart node
sudo systemctl restart apn-master-node
```

---

## Testing

### Test 1: Identity Persistence
```bash
# Start node
./target/release/apn_node

# Note the node_id from output
# Kill and restart
pkill apn_node
./target/release/apn_node

# Verify SAME node_id (not new one)
# Check identity file exists
cat ~/.apn/node_identity.json | jq .node_id
```

### Test 2: Duplicate Cleanup
```bash
# Check for duplicates
sqlite3 dev_assets/db.sqlite "
  SELECT node_id, cpu_cores, ram_mb, gpu_model, is_active
  FROM peer_nodes
  ORDER BY last_heartbeat_at DESC;
"

# Wait 60 seconds for cleanup to run
sleep 60

# Verify duplicates marked inactive
# (Same hardware specs, only most recent is active)
```

### Test 3: Stale Peer Cleanup
```bash
# Kill a peer node
pkill apn_node

# Wait 5 minutes
sleep 300

# Check it's marked inactive
sqlite3 dev_assets/db.sqlite "
  SELECT node_id, is_active, last_heartbeat_at
  FROM peer_nodes
  WHERE node_id = 'apn_xxx';
"
```

---

## Related Work

### Python APN Core
The Python implementation at `/home/pythia/apn-core` already has wallet persistence:
- **Commit:** `71dec77` - "Fix critical wallet persistence issue"
- **Documentation:** `WALLET-PERSISTENCE-FIX.md`
- **Recovery tool:** `recover_wallet.py`

This Rust implementation brings the same protections to the Rust-based dashboard.

---

## Impact

**Before Patch:**
- ❌ 5 active peers (2 were duplicates)
- ❌ New wallet on every restart
- ❌ VIBE rewards spread across multiple wallets
- ❌ Manual cleanup required

**After Patch:**
- ✅ 3 active peers (deduplicated)
- ✅ Persistent identities across restarts
- ✅ Single wallet per device
- ✅ Automatic cleanup every 60 seconds

---

## Best Practices

### For Users

1. **Backup your identity after first run:**
   ```bash
   cp ~/.apn/node_identity.json ~/apn_identity_backup_$(date +%Y%m%d).json
   ```

2. **Check identity before starting:**
   ```bash
   cat ~/.apn/node_identity.json | jq '.node_id, .wallet_address'
   ```

3. **Monitor your node_id:**
   - Should stay constant across restarts
   - If it changes, identity was regenerated (check logs)

### For Developers

1. **Never silently create new identity** - Fail loudly instead
2. **Always backup before modifications** - Can restore if update fails
3. **Always verify after save** - Ensure file is readable and correct
4. **Log file paths clearly** - Help users find/fix issues
5. **Use appropriate permissions** - Protect sensitive key material

---

## Files Changed

### New Files
- `crates/alpha-protocol-core/src/identity_storage.rs` - Identity persistence module
- `APN-DEDUPLICATION-PATCH.md` - This documentation

### Modified Files
- `crates/db/src/models/peer_node.rs` - Added cleanup methods
- `crates/server/src/main.rs` - Added background cleanup task
- `crates/alpha-protocol-core/src/lib.rs` - Export identity_storage
- `crates/alpha-protocol-core/src/bin/apn_node.rs` - Use persistent identities

---

**Status:** ✅ Tested and Deployed
**Authors:** Claude Sonnet 4.5, KingBodhi
**License:** MIT

Co-Authored-By: Claude Sonnet 4.5 <noreply@anthropic.com>

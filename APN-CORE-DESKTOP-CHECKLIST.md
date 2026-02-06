# APN Core Desktop App - Configuration Checklist

## 📋 Quick Verification Checklist

Use this checklist to ensure the desktop app on the other device is properly configured to connect and earn rewards.

---

## ✅ Step 1: Update Code (CRITICAL)

On the device running the desktop app:

```bash
cd pcg-cc-mcp
git checkout new
git pull origin new
```

**Why:** The `new` branch has all the reward tracker updates. Old code won't earn rewards.

---

## ✅ Step 2: Rebuild App

```bash
cd apn-app
npm install
npm run tauri build
```

**Or for dev mode:**
```bash
npm run tauri dev
```

---

## ✅ Step 3: Verify Network Configuration

The app should auto-configure these settings (no manual changes needed):

- ✅ **NATS Relay:** `nats://nonlocal.info:4222`
- ✅ **Port:** 4001 (or auto-assigned)
- ✅ **Capabilities:** compute, relay, storage
- ✅ **Heartbeat Interval:** 30 seconds

**Location in code:** `/home/pythia/pcg-cc-mcp/apn-app/src-tauri/src/main.rs`

```rust
let mut builder = AlphaNodeBuilder::new()
    .with_port(port)
    .with_relay(DEFAULT_NATS_RELAY)  // nats://nonlocal.info:4222
    .with_capabilities(vec![
        "compute".to_string(),
        "relay".to_string(),
        "storage".to_string(),
    ]);
```

---

## ✅ Step 4: Start the Node

1. Open the desktop app
2. Click "Start Node"
3. Wait for connection confirmation

**Expected behavior:**
- App shows "Node Running" ✅
- App shows "Relay Connected" ✅
- App shows "Peers: 1+" (at least Pythia master)

---

## ✅ Step 5: Verify Connection (from Pythia master)

Run these commands on Pythia (192.168.1.77) to verify the desktop app is connected:

### Check Database

```bash
sqlite3 ~/pcg-cc-mcp/dev_assets/db.sqlite "SELECT node_id, wallet_address, cpu_cores, ram_mb, gpu_available, updated_at FROM peer_nodes WHERE is_active = 1;"
```

**Expected:** Should show 2+ nodes (Pythia master + desktop app)

### Check Heartbeat Logs

```bash
grep "📨 Message from apn.heartbeat" /tmp/apn_node.log | tail -20
```

**Expected:** Should see heartbeats from both `apn_814d37f4` (Pythia) and another node_id (desktop app)

### Check Reward Tracker

```bash
tail -50 /tmp/apn_reward_tracker.log | grep "💰 Peer"
```

**Expected:** Should show reward calculations for multiple peers

---

## ✅ Step 6: Check Rewards

After 1-2 minutes of running, check if the desktop app is earning:

### Get Wallet Address

From the desktop app UI, copy the wallet address shown after starting the node.

Or check logs on the device running the app:
```bash
# If you saved the mnemonic/address when starting
```

### Check Balance (from any device)

```bash
curl http://192.168.1.77:58297/api/peers/YOUR_WALLET_ADDRESS/balance | jq
```

**Expected output:**
```json
{
  "wallet_address": "0x...",
  "balance_vibe": "1.234",
  "pending_distribution_vibe": "0.567",
  "total_earned_vibe": "1.801"
}
```

### Check Network Stats

```bash
curl http://192.168.1.77:58297/api/network/stats | jq
```

**Expected:** Should show `active_peers: 2+`

---

## 🐛 Troubleshooting

### Issue: Desktop app not showing in network

**Possible causes:**
1. ❌ App is running old code (not from `new` branch)
2. ❌ NATS relay can't be reached
3. ❌ App is running but node not started (user didn't click "Start Node")
4. ❌ Firewall blocking outbound connection to port 4222

**Solutions:**

```bash
# 1. Verify git branch
cd pcg-cc-mcp
git branch  # Should show "* new"

# 2. Test NATS connectivity
telnet nonlocal.info 4222  # Should connect

# 3. Check if app is running
ps aux | grep -i apn

# 4. Check firewall (if needed)
# Allow outbound to port 4222
```

### Issue: No rewards accumulating

**Possible causes:**
1. ❌ Node not sending heartbeats
2. ❌ Database not registering peer
3. ❌ Reward tracker not running on Pythia

**Solutions:**

```bash
# On Pythia master, verify reward services are running
ps aux | grep apn_reward

# Should see:
# - apn_reward_tracker
# - apn_reward_distributor

# Check reward tracker logs
tail -50 /tmp/apn_reward_tracker.log
```

### Issue: App shows "Relay Disconnected"

**Cause:** Can't reach NATS relay

**Solution:**
```bash
# Test connectivity
ping nonlocal.info
telnet nonlocal.info 4222

# If fails, check:
# - Internet connection
# - DNS resolution
# - Firewall settings
```

---

## 📊 Expected Earnings

Based on hardware specs:

### Example: MacBook Pro (M1, 8 cores, 16GB)
- Base: 0.1 VIBE per heartbeat
- No multipliers (no dedicated GPU, <16 cores, <32GB RAM)
- **Earnings:** ~0.2 VIBE/min = 12 VIBE/hour = 288 VIBE/day

### Example: Gaming PC (Ryzen 9, 16 cores, 32GB, RTX 3080)
- Base: 0.1 VIBE
- GPU multiplier: 2x
- **Earnings:** 0.2 VIBE per heartbeat = 24 VIBE/hour = 576 VIBE/day

### Example: Workstation (24 cores, 64GB, RTX 3090)
- Base: 0.1 VIBE
- GPU: 2x
- High CPU (>16 cores): 1.5x
- High RAM (>32GB): 1.3x
- Combined: 2.0 × 1.5 × 1.3 = 3.9x
- **Earnings:** 0.39 VIBE per heartbeat = 47 VIBE/hour = 1,128 VIBE/day

---

## 📞 Quick Support

**Can't connect?**
→ Check git branch: `git branch` (must be `new`)
→ Rebuild: `cd apn-app && npm run tauri build`
→ Test NATS: `telnet nonlocal.info 4222`

**Not earning rewards?**
→ Verify on Pythia: Check peer_nodes table
→ Check heartbeats: `grep apn.heartbeat /tmp/apn_node.log`
→ Verify reward tracker: `ps aux | grep reward_tracker`

**Need wallet address?**
→ Shown in desktop app after clicking "Start Node"
→ Or check app logs on the device

---

## ✅ Success Indicators

Your desktop app is properly configured when:

1. ✅ App shows "Node Running"
2. ✅ App shows "Relay Connected"
3. ✅ App shows "Peers: 1+"
4. ✅ Node appears in Pythia's peer_nodes database
5. ✅ Heartbeats visible in /tmp/apn_node.log on Pythia
6. ✅ Balance increases when checked via API
7. ✅ Rewards show in /tmp/apn_reward_tracker.log on Pythia

---

**Last Updated:** 2026-02-06
**Network Status:** ACTIVE ✅
**Rewards:** ENABLED 💰

# APN Core Desktop App - Rewards Setup Guide

## ✅ Current Network Status

**Master Node (Pythia):**
- Online at 192.168.1.77
- NATS Relay: nats://nonlocal.info:4222
- Reward System: ACTIVE ✅
- Distribution: Every 5 minutes

## 🚀 Quick Setup

### 1. Pull Latest Code (IMPORTANT!)

The desktop app MUST be running the latest code from the `new` branch to earn rewards:

```bash
cd pcg-cc-mcp
git checkout new
git pull origin new
```

### 2. Build Desktop App

```bash
cd apn-app
npm install
npm run tauri build
```

### 3. Run the App

**Development Mode:**
```bash
npm run tauri dev
```

**Production (after build):**
- Run the built binary from `src-tauri/target/release/`

### 4. Start the Node

Once the app opens:
1. Click "Start Node" button
2. The app will automatically:
   - Connect to NATS relay at `nats://nonlocal.info:4222`
   - Discover Pythia master node
   - Begin sending heartbeats every 30 seconds
   - Start earning VIBE rewards

## 💰 Reward System

### How It Works

The desktop app earns rewards by:
- Sending heartbeats every 30 seconds to prove it's online
- Rewards are calculated based on your hardware:
  - **Base:** 0.1 VIBE per heartbeat
  - **GPU Multiplier:** 2x (if GPU detected)
  - **High CPU Multiplier:** 1.5x (if >16 cores)
  - **High RAM Multiplier:** 1.3x (if >32GB RAM)

### Expected Earnings

**Example configurations:**
- **Basic laptop (8 cores, 16GB, no GPU):**
  - 0.1 VIBE per heartbeat
  - ~12 VIBE/hour
  - ~288 VIBE/day

- **Gaming PC (16 cores, 32GB, RTX GPU):**
  - 0.2 VIBE × 2x (GPU) = 0.4 VIBE per heartbeat
  - ~48 VIBE/hour
  - ~1,152 VIBE/day

- **Workstation (24+ cores, 64GB, GPU):**
  - 0.1 × 2.0 (GPU) × 1.5 (CPU) × 1.3 (RAM) = 0.39 VIBE per heartbeat
  - ~47 VIBE/hour
  - ~1,128 VIBE/day

### Check Your Rewards

After the node starts, note your wallet address from the app. Then check your balance:

```bash
# Replace YOUR_WALLET with your actual address
curl http://192.168.1.77:58297/api/peers/YOUR_WALLET/balance | jq
```

**View reward history:**
```bash
curl "http://192.168.1.77:58297/api/peers/YOUR_WALLET/rewards?limit=10" | jq
```

**Network stats:**
```bash
curl http://192.168.1.77:58297/api/network/stats | jq
```

## 🔧 Configuration

### Automatic Settings (No Changes Needed)

The app is pre-configured with:
- **NATS Relay:** `nats://nonlocal.info:4222` ✅
- **Port:** 4001 (default, will auto-increment if busy)
- **Capabilities:** compute, relay, storage ✅
- **Heartbeat Interval:** 30 seconds ✅

### Network Discovery

The app automatically discovers the network through NATS relay:
1. Connects to NATS relay
2. Subscribes to `apn.discovery` and `apn.heartbeat` topics
3. Announces itself to the network
4. Receives peer information from Pythia master

**No manual bootstrap address needed!** The NATS relay handles peer discovery.

## 📊 Verify Connection

### 1. Check App Status

In the desktop app, you should see:
- ✅ Node Running
- ✅ Relay Connected
- ✅ Peers: 1+ (at least Pythia master)

### 2. Check Database (from Pythia master)

```bash
# Run this on Pythia master (192.168.1.77)
sqlite3 ~/pcg-cc-mcp/dev_assets/db.sqlite "SELECT node_id, wallet_address, cpu_cores, ram_mb, gpu_available FROM peer_nodes WHERE is_active = 1;"
```

Your node should appear in the list.

### 3. Check Logs (from Pythia master)

```bash
# Run this on Pythia master
grep "📨 Message from apn.heartbeat" /tmp/apn_node.log | tail -10
```

You should see heartbeat messages from your node_id.

## 🐛 Troubleshooting

### App Won't Connect

**Check NATS relay connectivity:**
```bash
telnet nonlocal.info 4222
# Should connect successfully
```

**Check firewall:**
- Ensure outbound connections to port 4222 are allowed
- The app uses NATS relay, so no inbound ports need to be open

### No Rewards Appearing

**Possible causes:**
1. **Old code:** Make sure you pulled the latest `new` branch
2. **Node not sending heartbeats:** Check app logs for errors
3. **Database not registering:** Check if your node appears in peer_nodes table

**Solution:**
```bash
cd pcg-cc-mcp
git checkout new
git pull origin new
cd apn-app
npm run tauri build
```

Then restart the app.

### Can't See Network Peers

**Check in app:**
- The app reads peer info from `/tmp/apn_node.log` on the local machine
- If running on a different device, it won't see other peers in the GUI
- This is normal - your node still earns rewards!

**To verify you're connected:**
- Check your VIBE balance via API (see "Check Your Rewards" above)
- Ask network admin to verify your node in the database

## 🎯 What's Next

Once connected:
1. **Let it run:** The longer you're online, the more you earn
2. **Check rewards:** Use the API to monitor your VIBE balance
3. **Auto-distribution:** Rewards are batched and sent to your wallet every 5 minutes
4. **Stay updated:** Pull latest code periodically for updates

## 📞 Support

**Connection issues?**
- Verify NATS relay: `telnet nonlocal.info 4222`
- Check git branch: `git branch` (should show `* new`)
- Rebuild app: `npm run tauri build`

**Reward issues?**
- Verify heartbeats: Check with network admin
- Check database: Ask admin to verify your node_id
- Update code: `git pull origin new && npm run tauri build`

---

**Version:** Enhanced with VIBE Rewards
**Last Updated:** 2026-02-06
**Status:** Production Ready ✅
**Rewards:** ACTIVE - Start earning now! 💰

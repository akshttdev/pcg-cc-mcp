# Mac Studio - VIBE Rewards Setup Guide

## ✅ System Status

**Master Node (Pythia):**
- ✅ Running and sending heartbeats
- ✅ Reward tracker active (processing every 60s)
- ✅ Reward distributor active (distributing every 5min)
- ✅ Rewards wallet funded: 110M VIBE + 2.1 APT
- ✅ Currently earning: 0.3 VIBE per heartbeat (36 VIBE/hour)

**GitHub:**
- ✅ All code pushed to `new` branch
- ✅ Latest heartbeat fixes included
- ✅ Reward system fully operational
- ✅ Documentation updated

---

## 🚀 Mac Studio Quick Start

### 1. Pull Latest Code

```bash
cd ~/pcg-cc-mcp
git checkout new
git pull origin new
```

### 2. Build APN Node

```bash
cargo build --release --bin apn_node
```

### 3. Start Earning VIBE

```bash
./start-peer-node.sh
```

That's it! The Mac Studio will:
- Connect to NATS relay at `nats://nonlocal.info:4222`
- Send heartbeats every 30 seconds
- Earn **~0.13-0.2 VIBE per heartbeat** (20 cores + 64GB RAM)
- Receive distributions to wallet every 5 minutes

---

## 💰 Expected Earnings (Mac Studio)

**Hardware:**
- 20 CPU cores → 1.5x multiplier (>16 cores)
- 64GB RAM → 1.3x multiplier (>32GB)
- Combined: 1.95x total multiplier

**Rewards:**
- Base: 0.1 VIBE per heartbeat
- Multiplied: ~0.195 VIBE per heartbeat
- Per minute: 0.39 VIBE
- **Per hour: 23.4 VIBE**
- **Per day: 562 VIBE (~$5.62 at $0.01/VIBE)**

---

## 📊 Monitor Your Earnings

### Get Your Wallet Address

```bash
# From logs after node starts
grep "Address:" /tmp/apn_peer.log
```

### Check Balance

```bash
# Replace YOUR_WALLET with actual address from above
curl http://192.168.1.77:58297/api/peers/YOUR_WALLET/balance | jq
```

### View Network Stats

```bash
curl http://192.168.1.77:58297/api/network/stats | jq
```

---

**Ready to earn VIBE!** 🏴💰

See APN-README.md for complete documentation.
Last Updated: 2026-02-06

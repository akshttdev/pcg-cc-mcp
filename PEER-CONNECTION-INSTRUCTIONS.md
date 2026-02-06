# 🚨 IMPORTANT: Peer Device Connection Instructions

## Current Network Status

**Master Node (Pythia)**: 🟢 **ONLINE AND READY**
- Node ID: `apn_814d37f4`
- Peer ID: `12D3KooWGaopz8uKs5ikXxD4yy5wQDn5yue2Q38T81pMtLbxMVvt`
- Location: `192.168.1.77:4001`
- Features: ✅ Resource reporting, ✅ Heartbeat (30s), ✅ NATS relay
- Status: **Waiting for peer connections**

**Expected Peers**: 3 devices (currently 0 connected)

---

## 🔥 Action Required: Connect Your Peer Devices

The master node was **rebuilt with a new identity**. All peer devices need to reconnect using the **NEW bootstrap address**.

### Option 1: Automated Script (Recommended) ⚡

On each peer device, run:

```bash
curl -sSL https://raw.githubusercontent.com/KingBodhi/pcg-cc-mcp/new/connect-to-pythia.sh | bash
```

Or if you already have the repo:

```bash
cd pcg-cc-mcp
git checkout new
git pull origin new
./connect-to-pythia.sh
```

The script will:
- ✅ Verify master node connectivity
- ✅ Update to latest code
- ✅ Stop old nodes
- ✅ Build the binary
- ✅ Start your node with correct settings
- ✅ Verify connection

### Option 2: Manual Connection 🔧

If you prefer manual control:

```bash
# 1. Update repo
cd pcg-cc-mcp
git checkout new
git pull origin new

# 2. Stop old node
pkill -f apn_node

# 3. Build
cargo build --release --bin apn_node

# 4. Start with NEW bootstrap
./target/release/apn_node \
  --port 4002 \
  --relay nats://nonlocal.info:4222 \
  --bootstrap /ip4/192.168.1.77/tcp/4001/p2p/12D3KooWGaopz8uKs5ikXxD4yy5wQDn5yue2Q38T81pMtLbxMVvt \
  --heartbeat-interval 30 \
  > /tmp/apn_peer.log 2>&1 &

# 5. Monitor
tail -f /tmp/apn_peer.log
```

---

## ✅ Verify Your Connection

After starting your node, check the logs for these messages:

```
🟢 Node started: 12D3Koo...
🌐 Relay connected
📊 Collected resources: CPU=X cores, RAM=XMB, Storage=XGB
🔗 Peer connected
```

**On the master node**, you should see:

```
📨 Peer announcement: apn_xxxxx
💓 Heartbeat from peer: apn_xxxxx
```

---

## 🎯 What Changed?

**Old Bootstrap** (no longer valid):
```
/ip4/192.168.1.77/tcp/4001/p2p/12D3KooWPoBpKG7vzM2ufLt...
```

**NEW Bootstrap** (use this):
```
/ip4/192.168.1.77/tcp/4001/p2p/12D3KooWGaopz8uKs5ikXxD4yy5wQDn5yue2Q38T81pMtLbxMVvt
```

The master node identity changed, so all peers must reconnect with the new address.

---

## 🔍 Troubleshooting

### "Cannot reach master node"
```bash
ping 192.168.1.77
```
- If fails: Check network connectivity or VPN
- If succeeds: Check firewall allows port 4001

### "Build failed"
```bash
rustup update
cargo clean
cargo build --release --bin apn_node
```

### "Node keeps dying"
```bash
# Check logs for errors
tail -100 /tmp/apn_peer.log

# Common issues:
# - Port 4002 already in use: Change --port to 4003, 4004, etc.
# - Missing dependencies: cargo clean && cargo build
```

### "Connected but no peers visible"
- Wait 60 seconds for peer discovery
- Check NATS relay: Should see "Relay connected" in logs
- Verify bootstrap address is correct

---

## 📊 Check Network Status

On any connected node, run:

```bash
./check-network-capacity.sh
```

This will show all connected peers and their resources.

---

## 🆘 Quick Reference

| File | Purpose |
|------|---------|
| `connect-to-pythia.sh` | Automated connection script |
| `CONNECT-NOW.txt` | Quick copy-paste instructions |
| `APN-QUICKSTART.md` | 3-step guide |
| `DEPLOYMENT-GUIDE.md` | Full documentation |
| `check-network-capacity.sh` | Network monitoring |

---

## 📞 Current Master Node Recovery Phrase

**Save this for Pythia master node recovery:**

```
slush index float shaft flight citizen swear chunk correct veteran eyebrow blind
```

(This is only shown in master node logs: `/tmp/apn_node.log`)

---

## ⏰ Timeline

- **Master rebuilt**: 2026-02-04 14:47 UTC
- **New identity**: apn_814d37f4
- **Current status**: Master healthy, waiting for 3 peer connections
- **Action needed**: Peer devices must reconnect with new bootstrap

---

## 🎯 Expected Result

Once all 3 peers connect, you should see:

**Master Node** (`/tmp/apn_node.log`):
```
📨 Peer announcement: apn_xxxxx1
📨 Peer announcement: apn_xxxxx2
📨 Peer announcement: apn_xxxxx3
💓 Heartbeat from peer: apn_xxxxx1
💓 Heartbeat from peer: apn_xxxxx2
💓 Heartbeat from peer: apn_xxxxx3
```

**Network Capacity** (`./check-network-capacity.sh`):
```
Total Nodes: 4 (1 master + 3 peers)
Total Capacity:
  CPU: XXX cores
  RAM: XXX GB
  Storage: XXX GB
  GPUs: X devices
```

---

**Questions?** Check the full docs or review logs on master node.

**Ready to connect?** Pick Option 1 or Option 2 above and run on each peer device!

---

*Last Updated: 2026-02-04 | Master: apn_814d37f4 | Status: 🟢 READY*

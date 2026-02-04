# Alpha Protocol Network - Quick Start Guide

## 🚀 Connect Your Device in 3 Steps

### Step 1: Clone and Setup Repository

```bash
# Clone the repository
git clone https://github.com/KingBodhi/pcg-cc-mcp.git
cd pcg-cc-mcp

# Switch to the enhanced branch
git checkout new
git pull origin new
```

### Step 2: Run the Setup Script

```bash
# Make the script executable
chmod +x setup-peer-node.sh

# Run the automated setup
./setup-peer-node.sh
```

The script will prompt you for:
1. **Bootstrap address** - Copy from below
2. **Device name** - Choose a friendly name (e.g., "OKB-Terminal")

### Step 3: Use the Bootstrap Address

When prompted, paste this exact address:

```
/ip4/192.168.1.77/tcp/4001/p2p/12D3KooWGaopz8uKs5ikXxD4yy5wQDn5yue2Q38T81pMtLbxMVvt
```

**Note:** If connecting from outside the local network, use the public IP instead of `192.168.1.77`

---

## ✅ That's It!

Your device will:
- ✅ Build the APN node binary
- ✅ Connect to Pythia Master Node
- ✅ Report system resources (CPU, RAM, GPU, Storage)
- ✅ Send heartbeat status every 30 seconds
- ✅ Join the distributed compute network

---

## 📊 Verify Connection

After setup completes, check if your node is connected:

```bash
# View live logs
tail -f /tmp/apn_peer.log

# Look for these messages:
# 🟢 Node started
# 🌐 Relay connected
# 🔗 Peer connected
```

---

## 🔧 Manual Connection (Advanced)

If you prefer manual setup:

```bash
# Build the binary
cargo build --release --bin apn_node

# Start your node
./target/release/apn_node \
  --port 4002 \
  --relay nats://nonlocal.info:4222 \
  --bootstrap /ip4/192.168.1.77/tcp/4001/p2p/12D3KooWGaopz8uKs5ikXxD4yy5wQDn5yue2Q38T81pMtLbxMVvt
  --heartbeat-interval 30
```

---

## 🆘 Troubleshooting

### "Bootstrap address required"
- Make sure you copied the full bootstrap address including `/ip4/...`

### "Connection refused"
- Check if master node is running: `ping 192.168.1.77`
- Verify firewall allows port 4001

### "Build failed"
- Update Rust: `rustup update`
- Clean and rebuild: `cargo clean && cargo build --release --bin apn_node`

### "No GPU detected"
- This is normal if your device doesn't have a dedicated GPU
- The node will still work and report other resources

---

## 📖 More Information

- **Full Deployment Guide**: See [DEPLOYMENT-GUIDE.md](DEPLOYMENT-GUIDE.md)
- **Master Node Info**: See [PYTHIA-MASTER-INFO-ENHANCED.txt](PYTHIA-MASTER-INFO-ENHANCED.txt)
- **Network Capacity**: Run `./check-network-capacity.sh` on any node

---

## 🎯 What Your Node Shares

Your device will automatically share with the network:
- **CPU**: Number of cores and availability
- **RAM**: Total and available memory
- **Storage**: Available disk space
- **GPU**: Detection and model (if present)
- **Status**: Heartbeat every 30 seconds

All resource sharing is **read-only** - your actual files and data remain private.

---

## 🔒 Security

- Each node has a unique Ed25519 keypair
- All communication is encrypted (libp2p Noise protocol)
- Your mnemonic phrase is saved in the logs - keep it safe for node recovery
- No passwords or sensitive data are transmitted

---

**Need Help?** Check the full documentation or ask in the project channel.

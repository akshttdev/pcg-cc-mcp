# Alpha Protocol Network (APN) - Enhanced Edition

## 🌐 Distributed Mesh Network with Resource Reporting

The Alpha Protocol Network enables secure peer-to-peer communication and distributed computing across devices. This enhanced version includes automatic resource reporting and continuous status updates.

---

## 📋 Table of Contents

- [Quick Start](#-quick-start)
- [Features](#-features)
- [Architecture](#-architecture)
- [Network Information](#-network-information)
- [Setup Instructions](#-setup-instructions)
- [Monitoring](#-monitoring)
- [Documentation](#-documentation)

---

## 🚀 Quick Start

### For New Peer Nodes

```bash
git clone https://github.com/KingBodhi/pcg-cc-mcp.git
cd pcg-cc-mcp
git checkout new
chmod +x setup-peer-node.sh
./setup-peer-node.sh
```

When prompted, use this bootstrap address:
```
/ip4/192.168.1.77/tcp/4001/p2p/12D3KooWPoBpKG7vzM2ufLtHDVqenRKUvUu8DizsADhqqW7Z9du5
```

**That's it!** Your device will connect to the network automatically.

---

## ✨ Features

### Resource Reporting
- **Automatic Detection**: CPU cores, RAM, storage, GPU
- **Real-time Updates**: Fresh measurements every heartbeat
- **Network Visibility**: See capacity across all nodes

### Heartbeat Mechanism
- **Status Broadcasts**: Every 30 seconds (configurable)
- **Health Monitoring**: Detect offline/stale nodes
- **Load Distribution**: Task scheduling based on available resources

### Security
- **Encrypted Communication**: libp2p with Noise protocol
- **Identity Management**: Ed25519 keypairs with BIP39 mnemonics
- **Private by Default**: Only system metrics shared, not data

### Network Features
- **NAT Traversal**: NATS relay for nodes behind firewalls
- **Peer Discovery**: Automatic via relay and mDNS
- **Mesh Topology**: Direct P2P connections when possible
- **Gossipsub**: Efficient message broadcasting

### 💰 VIBE Rewards System
- **Automatic Earnings**: Get paid in VIBE tokens for being online
- **Heartbeat Rewards**: 0.1 VIBE base per heartbeat (every 30s)
- **Resource Multipliers**:
  - GPU: 2x multiplier
  - High CPU (>16 cores): 1.5x multiplier
  - High RAM (>32GB): 1.3x multiplier
- **Automatic Distribution**: Rewards batched and sent to your wallet every 5 minutes
- **On-Chain**: All VIBE tokens distributed on Aptos blockchain (Testnet)

**Example Earnings:**
- Basic node (no GPU): ~0.12 VIBE/min = 7.2 VIBE/hour = 173 VIBE/day
- GPU node (24 cores + RTX 3080 Ti): ~0.6 VIBE/min = 36 VIBE/hour = 864 VIBE/day
- Mac Studio (20 cores, 64GB RAM): ~0.2 VIBE/min = 12 VIBE/hour = 288 VIBE/day

**Track Your Earnings:**
```bash
# Check your balance
curl http://192.168.1.77:58297/api/peers/YOUR_WALLET_ADDRESS/balance | jq

# View network stats
curl http://192.168.1.77:58297/api/network/stats | jq
```

---

## 🏗 Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                    PYTHIA MASTER NODE                        │
│                                                               │
│  Resources: 24 cores, 32GB RAM, 3.1TB storage, RTX 3080 Ti │
│  Role: Network coordinator and bootstrap node                │
│  Status: Broadcasting resources every 30s                    │
└───────────────────────────┬─────────────────────────────────┘
                            │
            ┌───────────────┼───────────────┐
            │               │               │
       ┌────▼────┐     ┌────▼────┐    ┌────▼────┐
       │ PEER 1  │     │ PEER 2  │    │ PEER 3  │
       │         │     │         │    │         │
       │ Auto    │     │ Auto    │    │ Auto    │
       │ Reports │     │ Reports │    │ Reports │
       └─────────┘     └─────────┘    └─────────┘
```

**Communication Channels:**
- **NATS Relay**: `nats://nonlocal.info:4222` (NAT traversal)
- **libp2p**: Direct P2P connections (when not firewalled)
- **Topics**: `apn.discovery`, `apn.heartbeat`, `apn.tasks`, `apn.peers`

---

## 🌐 Network Information

### Current Master Node

**Identity:**
- Node ID: `apn_7bb626c3`
- Wallet: `0x7bb626c38d1d4668f6459c2134bc8b44f3db7e5b35d7c6be443b8e9973561d42`
- LibP2P: `12D3KooWPoBpKG7vzM2ufLtHDVqenRKUvUu8DizsADhqqW7Z9du5`

**Bootstrap Addresses:**

**Local Network:**
```
/ip4/192.168.1.77/tcp/4001/p2p/12D3KooWPoBpKG7vzM2ufLtHDVqenRKUvUu8DizsADhqqW7Z9du5
```

**External Network** (use public IP if connecting from outside):
```
/ip4/YOUR_PUBLIC_IP/tcp/4001/p2p/12D3KooWPoBpKG7vzM2ufLtHDVqenRKUvUu8DizsADhqqW7Z9du5
```

### Connection Details

- **NATS Relay**: `nats://nonlocal.info:4222`
- **Master P2P Port**: `4001`
- **Peer P2P Port**: `4002+` (use different ports per device)
- **Heartbeat Interval**: `30 seconds` (default)

---

## 📥 Setup Instructions

### Prerequisites

- **Rust**: Install from https://rustup.rs/
- **Git**: For cloning the repository
- **Linux/macOS**: Tested on Ubuntu, Pop!_OS, macOS
- **Internet**: For building dependencies and NATS relay

### Option 1: Automated Setup (Recommended)

**For Peer Nodes:**
```bash
cd pcg-cc-mcp
git checkout new
git pull origin new
./setup-peer-node.sh
```

The script handles everything:
- Pulls latest code
- Builds the binary
- Prompts for bootstrap address
- Starts the node with proper configuration
- Displays connection status

### Option 2: Manual Setup

**Build:**
```bash
cargo build --release --bin apn_node
```

**Run:**
```bash
./target/release/apn_node \
  --port 4002 \
  --relay nats://nonlocal.info:4222 \
  --bootstrap /ip4/192.168.1.77/tcp/4001/p2p/12D3KooWPoBpKG7vzM2ufLtHDVqenRKUvUu8DizsADhqqW7Z9du5 \
  --heartbeat-interval 30
```

### Configuration Options

```bash
# Custom heartbeat interval (seconds)
--heartbeat-interval 60

# Disable heartbeat (not recommended)
--no-heartbeat

# Custom port
--port 4003

# Multiple bootstrap peers
--bootstrap /ip4/IP1/tcp/4001/p2p/PEER1 \
--bootstrap /ip4/IP2/tcp/4001/p2p/PEER2

# Import existing identity
--import "your twelve word mnemonic phrase here"
```

---

## 📊 Monitoring

### Check Network Capacity

```bash
./check-network-capacity.sh
```

**Output includes:**
- Local node resources and utilization
- All discovered peers and their capabilities
- Network health summary
- Capacity ratings

### View Live Logs

```bash
# Master node
tail -f /tmp/apn_node.log

# Peer node
tail -f /tmp/apn_peer.log
```

### Monitor Heartbeats

```bash
# Watch resource collections
grep "Collected resources" /tmp/apn_peer.log

# Watch peer discoveries
grep "📨" /tmp/apn_peer.log
```

### Check Node Status

```bash
# Check if node is running
ps aux | grep apn_node

# Check uptime
ps -p $(cat /tmp/apn_peer.pid) -o etime=
```

---

## 📖 Documentation

### Quick References
- **[APN-QUICKSTART.md](APN-QUICKSTART.md)** - 3-step connection guide
- **[PYTHIA-MASTER-INFO-ENHANCED.txt](PYTHIA-MASTER-INFO-ENHANCED.txt)** - Master node details
- **[DEPLOYMENT-GUIDE.md](DEPLOYMENT-GUIDE.md)** - Complete deployment documentation

### Setup Scripts
- **[setup-peer-node.sh](setup-peer-node.sh)** - Automated peer setup
- **[setup-pythia-master-enhanced.sh](setup-pythia-master-enhanced.sh)** - Master node setup
- **[check-network-capacity.sh](check-network-capacity.sh)** - Capacity monitoring

### Technical Documentation
- **Architecture**: See `docs/APN_INTEGRATION_ARCHITECTURE.md`
- **Wire Protocol**: See `crates/alpha-protocol-core/src/wire.rs`
- **Resources Module**: See `crates/alpha-protocol-core/src/resources.rs`

---

## 🛠 Troubleshooting

### Node Won't Connect

**Check NATS relay:**
```bash
grep "Relay connected" /tmp/apn_peer.log
```

**Verify bootstrap address:**
```bash
# Test connectivity to master
ping 192.168.1.77
telnet 192.168.1.77 4001
```

**Check firewall:**
```bash
# If needed, open P2P port
sudo ufw allow 4002/tcp
```

### Build Errors

**Update dependencies:**
```bash
rustup update
cargo clean
cargo build --release --bin apn_node
```

### Resource Collection Fails

**Verify system access:**
```bash
# Run resource detection test
cargo test --release --package alpha-protocol-core test_system_info
```

**Check GPU detection:**
```bash
nvidia-smi  # NVIDIA GPUs
rocm-smi    # AMD GPUs
```

### No Peers Discovered

**Possible causes:**
1. Master node is offline
2. Bootstrap address is incorrect
3. Firewall blocking NATS relay
4. Network connectivity issues

**Solutions:**
1. Verify master is running: Check with network admin
2. Copy bootstrap address from `PYTHIA-MASTER-INFO-ENHANCED.txt`
3. Test NATS connectivity: `telnet nonlocal.info 4222`
4. Check internet connection

---

## 🔐 Security Notes

### Identity Management
- Each node generates a unique Ed25519 keypair
- Recovery phrase (12 words) saved in logs
- Keep recovery phrase secure for node restoration

### Network Security
- All P2P communication encrypted with Noise protocol
- NATS relay messages use TLS
- Only metadata shared (resources, capabilities)
- No file access or data transmission without explicit tasks

### Resource Sharing
Resources reported are **read-only system metrics**:
- CPU core count (not usage patterns)
- Total RAM (not process details)
- Available storage (not file contents)
- GPU model (not usage history)

**Your files and data remain private and secure.**

---

## 📞 Support

### Getting Help
- **Documentation**: Check `DEPLOYMENT-GUIDE.md` for detailed instructions
- **Logs**: Review `/tmp/apn_peer.log` for error messages
- **Status**: Run `./check-network-capacity.sh` for network overview

### Common Issues
- Build fails → Update Rust and clean rebuild
- Can't connect → Verify bootstrap address and network
- No resources → Normal if GPU not present, other metrics will report

---

## 🎯 What's Next

After connecting:
1. **Monitor Your Node**: Use `./check-network-capacity.sh`
2. **View Network**: See all connected peers and their resources
3. **Start Earning VIBE**: Your node automatically earns rewards for being online
4. **Check Your Balance**: Use the API to see your pending rewards
5. **Receive Distributions**: Rewards automatically sent to your wallet every 5 minutes
6. **Ready for Tasks**: Your node can receive distributed compute tasks

**Check Your Earnings:**
```bash
# Get your wallet address from logs
grep "Address:" /tmp/apn_peer.log

# Check your VIBE balance
curl http://192.168.1.77:58297/api/peers/YOUR_WALLET_ADDRESS/balance | jq

# View detailed rewards history
curl "http://192.168.1.77:58297/api/peers/YOUR_WALLET_ADDRESS/rewards?limit=10" | jq
```

---

## 📄 License

MIT License - See LICENSE file for details

---

## 🤝 Contributing

This is an active development project. For issues or improvements:
1. Check existing documentation
2. Review logs for error messages
3. Contact project maintainers

---

**Version**: Enhanced with Resource Reporting + VIBE Rewards
**Last Updated**: 2026-02-06
**Status**: Production Ready ✅
**Rewards**: ACTIVE - Earn VIBE for being online! 💰

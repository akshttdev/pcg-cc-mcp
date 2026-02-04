# Alpha Protocol Network (APN) - Connect Your Device

> **Quick Connect:** See [APN-QUICKSTART.md](APN-QUICKSTART.md) for 3-step connection guide

## 🌐 What is APN?

Alpha Protocol Network is a distributed mesh network that enables:
- Secure peer-to-peer communication
- Distributed task execution
- Resource sharing across devices
- Automatic capacity reporting

## 🚀 Quick Start for New Devices

```bash
# 1. Clone and setup
git clone https://github.com/KingBodhi/pcg-cc-mcp.git
cd pcg-cc-mcp
git checkout new

# 2. Run automated setup
chmod +x setup-peer-node.sh
./setup-peer-node.sh
```

**Bootstrap address when prompted:**
```
/ip4/192.168.1.77/tcp/4001/p2p/12D3KooWPoBpKG7vzM2ufLtHDVqenRKUvUu8DizsADhqqW7Z9du5
```

## 📚 Documentation

| Document | Description |
|----------|-------------|
| [APN-QUICKSTART.md](APN-QUICKSTART.md) | 3-step connection guide |
| [APN-README.md](APN-README.md) | Complete APN documentation |
| [DEPLOYMENT-GUIDE.md](DEPLOYMENT-GUIDE.md) | Full deployment instructions |
| [BOOTSTRAP-INFO.txt](BOOTSTRAP-INFO.txt) | Copy-paste connection info |
| [PYTHIA-MASTER-INFO-ENHANCED.txt](PYTHIA-MASTER-INFO-ENHANCED.txt) | Master node details |

## ✨ Features

- **Automatic Resource Detection**: CPU, RAM, GPU, Storage
- **Heartbeat Status**: Real-time updates every 30s
- **Secure Communication**: End-to-end encrypted with libp2p
- **NAT Traversal**: Works behind firewalls via NATS relay
- **Zero Configuration**: Setup script handles everything

## 🔧 Manual Connection

If you prefer manual control:

```bash
# Build
cargo build --release --bin apn_node

# Run
./target/release/apn_node \
  --port 4002 \
  --relay nats://nonlocal.info:4222 \
  --bootstrap /ip4/192.168.1.77/tcp/4001/p2p/12D3KooWPoBpKG7vzM2ufLtHDVqenRKUvUu8DizsADhqqW7Z9du5 \
  --heartbeat-interval 30
```

## 📊 Check Network Status

```bash
# View network capacity
./check-network-capacity.sh

# Monitor your node
tail -f /tmp/apn_peer.log
```

## 🆘 Need Help?

- **Connection issues?** See [DEPLOYMENT-GUIDE.md](DEPLOYMENT-GUIDE.md#troubleshooting)
- **Build errors?** Update Rust: `rustup update`
- **Questions?** Check the full documentation in [APN-README.md](APN-README.md)

---

**Status:** ✅ Production Ready | **Last Updated:** 2026-02-04

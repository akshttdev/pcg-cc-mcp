# 🏴 APN Peer Node Setup - Sovereign Stack Edition

## Quick Start for Mac Studio

```bash
# 1. Clone repository
git clone -b new https://github.com/KingBodhi/pcg-cc-mcp.git
cd pcg-cc-mcp

# 2. Build APN node
cargo build --release -p alpha-protocol-core

# 3. Start as peer
./start-peer-node.sh
```

That's it! You're now part of the sovereign mesh! 🏴

## What This Does

- Connects to NATS relay (nats://nonlocal.info:4222)
- Joins LibP2P mesh network
- Registers with master node (apn_814d37f4)
- Ready to contribute compute and earn VIBE

## Architecture

```
Master Node (192.168.1.77) ←→ NATS Relay ←→ Your Peer Node
```

All communication:
- ✅ Peer-to-peer
- ✅ Encrypted
- ✅ Sovereign
- ✅ No third parties

## Verify Connection

```bash
# Check your peer logs
tail -f /tmp/apn_peer.log

# Should see:
# ✅ Connected to NATS relay
# ✅ Published to apn.discovery
# ✅ Mesh peer ID: [your_id]
```

## Running Projects

Projects connect to local backend, which uses the mesh:

```bash
# Projects → Local Backend → APN Mesh → Master Node Agents
```

Fully decentralized, fully sovereign!

For detailed setup, see full documentation in the file.

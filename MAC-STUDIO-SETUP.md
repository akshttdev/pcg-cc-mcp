# 🏴 Mac Studio - Join the Sovereign APN Mesh

## Quick Start (3 Commands)

```bash
# 1. Clone the repository
git clone -b new https://github.com/KingBodhi/pcg-cc-mcp.git
cd pcg-cc-mcp

# 2. Build the APN node
cargo build --release -p alpha-protocol-core

# 3. Join the mesh!
./start-peer-node.sh
```

**That's it!** You're now part of the sovereign mesh! 🏴

## What Just Happened

✅ Your Mac is now an APN peer node
✅ Connected to NATS relay (nats://nonlocal.info:4222)
✅ Joined LibP2P mesh network
✅ Can communicate with master node (apn_814d37f4)
✅ Ready to run projects that use distributed compute

## Verify Connection

```bash
# Check your peer logs
tail -f /tmp/apn_peer.log

# You should see:
✅ Connected to NATS relay at nats://nonlocal.info:4222
✅ Mesh peer ID: apn_[your unique ID]
✅ Publishing heartbeat every 30 seconds
```

## Running Your Projects

Your projects now work locally AND leverage the mesh:

```bash
# Start local backend (optional)
BACKEND_PORT=58297 ./target/release/server &

# Your projects connect to localhost
# But compute is distributed through APN mesh!
```

**Projects → Local Backend → APN Mesh → Master Node Agents**

## Architecture

```
┌──────────────────────────────────────────┐
│         SOVEREIGN APN MESH               │
├──────────────────────────────────────────┤
│                                          │
│  Master Node (192.168.1.77)             │
│  ├─ 7 Agents (Nora, Maci, etc.)        │
│  ├─ All AI services                     │
│  └─ Network coordination                │
│                                          │
│           ↕️  NATS Relay  ↕️            │
│     (nats://nonlocal.info:4222)         │
│                                          │
│  Your Mac Studio (Peer Node)            │
│  ├─ APN Node (LibP2P)                   │
│  ├─ Local Projects                      │
│  └─ Distributed Compute                 │
│                                          │
└──────────────────────────────────────────┘
```

## What Makes This Sovereign

✅ **No Third Parties**
- No Cloudflare Tunnel
- No Tailscale VPN
- No centralized coordinators

✅ **Peer-to-Peer**
- Direct LibP2P connections
- NATS relay YOU control
- Fully decentralized

✅ **Bitcoin Incentivized**
- Contribute compute → Earn VIBE
- Sovereign economic layer

## Troubleshooting

### Can't Build?
```bash
# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Restart terminal, try again
cargo build --release -p alpha-protocol-core
```

### Peer Not Connecting?
```bash
# Check NATS connectivity
nc -zv nonlocal.info 4222

# Should say: "succeeded!"
```

### Want to See Master Node?
```bash
# Master node dashboard
open http://dashboard.powerclubglobal.com

# Master node API
curl http://192.168.1.77:8081/api/status
```

## Next Steps

1. ✅ Peer node running
2. ✅ Connected to mesh
3. ▶️  Run your projects locally
4. ▶️  Leverage distributed agents
5. ▶️  Earn VIBE for contributions

## Support

Questions? The master node is always here:
- Dashboard: http://dashboard.powerclubglobal.com
- API: http://192.168.1.77:8081/api/status
- NATS: nats://nonlocal.info:4222

**Welcome to the sovereign stack!** 🏴

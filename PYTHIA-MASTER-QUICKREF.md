# Pythia Master Node - Quick Reference

## Current Configuration

**Machine**: 192.168.1.77 (Pythia Master Node)
**Node ID**: apn_814d37f4
**Resources**: 24 cores, 32GB RAM, RTX 3080 Ti
**Role**: Network Orchestrator & Coordinator

---

## Starting the Master Node

### Quick Start (Recommended)
```bash
cd /home/pythia/pcg-cc-mcp
./start-pythia-master.sh
```

### Manual Start
```bash
cd /home/pythia/pcg-cc-mcp

# Start Master Node
nohup ./target/release/apn_node \
  --port 4001 \
  --relay nats://nonlocal.info:4222 \
  --heartbeat-interval 30 \
  --import "slush index float shaft flight citizen swear chunk correct veteran eyebrow blind" \
  > /tmp/apn_node.log 2>&1 &

# Start API Server
./target/release/apn_api_server > /tmp/apn_api.log 2>&1 &
```

---

## Monitoring

### Check Status
```bash
# Process status
ps aux | grep apn_node
ps aux | grep apn_api_server

# Network status (API)
curl http://192.168.1.77:8081/api/status | jq

# View logs
tail -f /tmp/apn_node.log
tail -f /tmp/apn_api.log
```

### Key Endpoints
- **API Status**: http://192.168.1.77:8081/api/status
- **Web Dashboard**: http://192.168.1.77:8081/
- **LibP2P Port**: 4001
- **NATS Relay**: nats://nonlocal.info:4222

---

## Stopping

### Stop All Services
```bash
pkill apn_node
pkill apn_api_server
```

### Stop Individual Services
```bash
# Stop master node
kill $(cat /tmp/apn_master.pid)

# Stop API server
kill $(cat /tmp/apn_api.pid)
```

---

## Rebuilding

### After Code Changes
```bash
cd /home/pythia/pcg-cc-mcp
cargo build --release --bin apn_node
cargo build --release --bin apn_api_server
```

### Quick Restart After Build
```bash
pkill apn_node; pkill apn_api_server
sleep 2
./start-pythia-master.sh
```

---

## PyQt6 Dashboard

### Running the Dashboard
```bash
cd /home/pythia/apn-core
python3 main.py
```

### Dashboard Features
- **Home**: Overview and system stats
- **Node Config**: APN node settings
- **Peer Nodes**: Network-wide node visibility (port 8081 API)
- **Devices**: Meshtastic device management
- **Profile**: Identity and wallet info

### Dashboard Dependencies
```bash
cd /home/pythia/apn-core
pip install -r requirements.txt
```

Key dependencies: PyQt6, httpx, psutil

---

## Troubleshooting

### Master Node Won't Start
```bash
# Check if port is in use
netstat -tlnp | grep 4001

# Check NATS relay connectivity
telnet nonlocal.info 4222

# View recent errors
tail -50 /tmp/apn_node.log | grep ERROR
```

### API Server Issues
```bash
# Check if port 8081 is available
netstat -tlnp | grep 8081

# Verify API server is running
curl http://localhost:8081/api/status

# Check logs
tail -50 /tmp/apn_api.log
```

### No Peers Showing Up
```bash
# Verify relay connection
grep "Relay connected" /tmp/apn_node.log

# Check for peer announcements
grep "PeerAnnouncement" /tmp/apn_node.log

# Verify heartbeat messages
grep "Heartbeat" /tmp/apn_node.log
```

---

## Network Topology

```
Pythia Master (192.168.1.77)
├─ LibP2P: /ip4/0.0.0.0/tcp/4001
├─ NATS: nats://nonlocal.info:4222
├─ API: http://192.168.1.77:8081
│
├─ Peer Node 1 (APN Core Worker)
├─ Peer Node 2 (APN Core Worker)
└─ Peer Node N (APN Core Worker)
```

All peers connect to:
- **NATS Relay**: nats://nonlocal.info:4222
- **LibP2P Mesh**: Automatic discovery via mDNS and DHT
- **Heartbeat**: 30-second intervals

---

## Important Files

**Binaries:**
- `/home/pythia/pcg-cc-mcp/target/release/apn_node`
- `/home/pythia/pcg-cc-mcp/target/release/apn_api_server`

**Logs:**
- `/tmp/apn_node.log` - Master node logs
- `/tmp/apn_api.log` - API server logs

**PIDs:**
- `/tmp/apn_master.pid` - Master node process ID
- `/tmp/apn_api.pid` - API server process ID

**Scripts:**
- `/home/pythia/pcg-cc-mcp/start-pythia-master.sh` - Start all services

**Documentation:**
- `/home/pythia/pcg-cc-mcp/APN-PEER-SETUP.md` - Peer setup guide

---

## Vibe Check Command

```bash
echo "=== PYTHIA MASTER STATUS ===" && \
echo "" && \
echo "Processes:" && \
ps aux | grep -E "(apn_node|apn_api)" | grep -v grep && \
echo "" && \
echo "Network Status:" && \
curl -s http://192.168.1.77:8081/api/status | jq && \
echo "" && \
echo "Recent Logs:" && \
tail -10 /tmp/apn_node.log
```

---

## Master Node Identity

**Mnemonic**: `slush index float shaft flight citizen swear chunk correct veteran eyebrow blind`
**Node ID**: `apn_814d37f4`
**Wallet Address**: Derived from mnemonic (Ed25519)

⚠️ **NEVER SHARE THE MNEMONIC** - This is the master key for Pythia.

---

## Next Steps for Network Expansion

1. Share `APN-PEER-SETUP.md` with new peer operators
2. Monitor `/tmp/apn_node.log` for peer connections
3. Watch API at http://192.168.1.77:8081/api/status for new nodes
4. Use PyQt6 dashboard to visualize network growth

---

**Pythia Master Node is the heart of the Alpha Protocol Network.** ❤️

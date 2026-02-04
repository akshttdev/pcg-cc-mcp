# Alpha Protocol Network - Enhanced Deployment Guide

## 🚀 Quick Start

### For Pythia Master Node

```bash
cd /path/to/pcg-cc-mcp
git checkout new
git pull origin new
chmod +x setup-pythia-master-enhanced.sh
./setup-pythia-master-enhanced.sh
```

This will:
- Build the enhanced apn_node binary
- Start Pythia Master with resource reporting enabled
- Create `PYTHIA-MASTER-INFO-ENHANCED.txt` with connection details

### For Peer Nodes

```bash
cd /path/to/pcg-cc-mcp
git checkout new
git pull origin new
chmod +x setup-peer-node.sh
./setup-peer-node.sh
```

The script will prompt for:
- Bootstrap address (get from PYTHIA-MASTER-INFO-ENHANCED.txt)
- Device name (e.g., "OKB-Terminal", "Device-3")

---

## 🎯 New Features

### Resource Reporting
Every node now reports:
- **CPU**: Core count and architecture
- **RAM**: Total and available memory
- **Storage**: Available disk space
- **GPU**: Detection and model information

### Heartbeat Mechanism
- Broadcasts status every 30 seconds (configurable)
- Includes fresh resource measurements
- Helps detect offline/stale nodes

### Network Visibility
Use the capacity check script to see all nodes:

```bash
./check-network-capacity.sh
```

---

## 📋 Manual Deployment

If you prefer manual control:

### 1. Build the Binary

```bash
cargo build --release --bin apn_node
```

### 2. Start Pythia Master

```bash
./target/release/apn_node \
  --port 4001 \
  --relay nats://nonlocal.info:4222 \
  --heartbeat-interval 30 \
  > /tmp/apn_master.log 2>&1 &
```

### 3. Start Peer Node

```bash
./target/release/apn_node \
  --port 4002 \
  --relay nats://nonlocal.info:4222 \
  --bootstrap /ip4/MASTER_IP/tcp/4001/p2p/MASTER_PEER_ID \
  --heartbeat-interval 30 \
  > /tmp/apn_peer.log 2>&1 &
```

---

## 🔧 Configuration Options

### CLI Flags

- `--port <PORT>` - P2P port (default: 4001)
- `--relay <URL>` - NATS relay URL (default: nats://nonlocal.info:4222)
- `--bootstrap <MULTIADDR>` - Bootstrap peer address (can use multiple times)
- `--heartbeat-interval <SECS>` - Heartbeat interval in seconds (default: 30)
- `--no-heartbeat` - Disable heartbeat broadcasts
- `--import <PHRASE>` - Import existing identity from mnemonic
- `--new` - Generate new identity (default)

### Examples

Start with custom heartbeat interval:
```bash
./target/release/apn_node --heartbeat-interval 60
```

Start without heartbeat (not recommended):
```bash
./target/release/apn_node --no-heartbeat
```

Start on custom port:
```bash
./target/release/apn_node --port 4003
```

---

## 🔍 Monitoring

### View Live Logs

Master node:
```bash
tail -f /tmp/apn_master.log
```

Peer node:
```bash
tail -f /tmp/apn_peer.log
```

### Check Network Capacity

```bash
./check-network-capacity.sh
```

Shows:
- Local node resources and utilization
- All discovered peer nodes
- Network health summary
- Capacity ratings

### Check Heartbeats

```bash
grep "Heartbeat" /tmp/apn_master.log
```

You should see heartbeat messages every 30 seconds.

---

## 🛠 Troubleshooting

### Node Not Discovering Peers

**Symptom**: No peer announcements in logs

**Solutions**:
1. Check NATS relay connection:
   ```bash
   grep "Relay connected" /tmp/apn_*.log
   ```

2. Verify bootstrap address is correct:
   ```bash
   cat PYTHIA-MASTER-INFO-ENHANCED.txt
   ```

3. Check firewall settings:
   ```bash
   sudo ufw status
   sudo ufw allow 4001/tcp  # If needed
   ```

### Resource Collection Fails

**Symptom**: Resources show as `None` or errors in logs

**Solutions**:
1. Check if sysinfo can access system info:
   ```bash
   cargo test --release --package alpha-protocol-core test_system_info
   ```

2. Verify GPU detection:
   ```bash
   nvidia-smi  # For NVIDIA GPUs
   rocm-smi    # For AMD GPUs
   ```

### Heartbeat Not Working

**Symptom**: No heartbeat messages after startup

**Solutions**:
1. Check if heartbeat is enabled:
   ```bash
   grep "Heartbeat enabled" /tmp/apn_*.log
   ```

2. Verify no errors in resource collection:
   ```bash
   grep "ERROR.*resource" /tmp/apn_*.log
   ```

### Build Errors

If build fails with missing dependencies:

```bash
# Update Rust
rustup update

# Clean and rebuild
cargo clean
cargo build --release --bin apn_node
```

---

## 📊 Network Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                    PYTHIA MASTER NODE                        │
│  ┌────────────────────────────────────────────────────────┐ │
│  │ Resources: 24 cores, 32GB RAM, 3.1TB, RTX 3080 Ti      │ │
│  │ Heartbeat: Every 30s                                   │ │
│  │ NATS Relay: nats://nonlocal.info:4222                 │ │
│  │ P2P Port: 4001                                         │ │
│  └────────────────────────────────────────────────────────┘ │
│                                                               │
│  Announces resources via:                                    │
│  - NATS relay (apn.discovery, apn.heartbeat)                │
│  - libp2p gossipsub (apn.peers, apn.heartbeat)              │
└───────────────────────────┬─────────────────────────────────┘
                            │
            ┌───────────────┼───────────────┐
            │               │               │
       ┌────▼────┐     ┌────▼────┐    ┌────▼────┐
       │ PEER 1  │     │ PEER 2  │    │ PEER 3  │
       │         │     │         │    │         │
       │ Port:   │     │ Port:   │    │ Port:   │
       │ 4002    │     │ 4003    │    │ 4004    │
       └─────────┘     └─────────┘    └─────────┘
```

---

## 🎯 Next Steps for Peer Nodes

Once the enhanced master node is running:

1. **Get bootstrap address** from `PYTHIA-MASTER-INFO-ENHANCED.txt`

2. **On each peer device**:
   ```bash
   cd /path/to/pcg-cc-mcp
   git checkout new
   git pull origin new
   ./setup-peer-node.sh
   ```

3. **Verify connection**:
   - Check peer logs for "Relay connected"
   - Check master logs for peer announcements
   - Run `./check-network-capacity.sh` on master

4. **Monitor resources**:
   - All nodes will report resources every 30s
   - Use capacity check script to see network health
   - Resources include CPU, RAM, GPU, storage

---

## 📝 Notes

- **Backward Compatibility**: Old nodes (without resource reporting) can still connect, but will show `resources: None`
- **Network Overhead**: ~200 bytes per heartbeat per node. With 10 nodes @ 30s interval = ~4KB/minute (negligible)
- **Resource Collection**: Takes 100-500ms, runs in background thread to avoid blocking
- **GPU Detection**: Automatic for NVIDIA (nvidia-smi), AMD (rocm-smi), and macOS (system_profiler)

---

## 🔐 Security

- Each node has unique Ed25519 keypair
- Mnemonic phrase for recovery (12 words)
- All communication encrypted via libp2p Noise protocol
- Resources are non-sensitive system information

**Important**: Keep your mnemonic phrase safe! It's needed to recover node identity.

---

## 📚 Additional Resources

- Main README: `/home/pythia/pcg-cc-mcp/README.md`
- Architecture docs: `/home/pythia/pcg-cc-mcp/docs/APN_INTEGRATION_ARCHITECTURE.md`
- Master node info: `/home/pythia/pcg-cc-mcp/PYTHIA-MASTER-INFO-ENHANCED.txt`

---

**Status**: ✅ Enhanced system deployed and tested
**Master Node**: Running with full resource reporting
**Ready For**: Peer node connections with same capabilities

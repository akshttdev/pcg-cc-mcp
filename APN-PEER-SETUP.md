# APN Core Peer Setup Guide

## Overview
This guide helps you set up an **APN Core Peer Node** to contribute computational resources to the Alpha Protocol Network and earn VIBE tokens.

---

## Prerequisites

1. **Hardware Requirements:**
   - CPU: 2+ cores (more is better)
   - RAM: 4GB minimum (8GB+ recommended)
   - Storage: 50GB+ available
   - GPU: Optional (NVIDIA GPUs earn more VIBE)

2. **Network Requirements:**
   - Internet connection
   - Ability to reach NATS relay: nats://nonlocal.info:4222
   - Open port 4001 for LibP2P (or use different port)

3. **Software Requirements:**
   - Linux (Ubuntu 20.04+, PopOS, Debian)
   - Rust toolchain installed
   - Git

---

## Installation Steps

### 1. Clone the Repository

```bash
cd ~
git clone https://github.com/KingBodhi/pcg-cc-mcp.git
cd pcg-cc-mcp
```

### 2. Build APN Core

```bash
cargo build --release --bin apn_node
```

This will take 5-10 minutes depending on your hardware.

### 3. Generate Your Node Identity

**IMPORTANT:** Save your mnemonic phrase! This is your node's identity and wallet.

```bash
# Generate a new identity
./target/release/apn_node --generate-mnemonic

# Output will show your 12-word recovery phrase:
# slush index float shaft flight citizen swear chunk correct veteran eyebrow blind
```

**SAVE THIS PHRASE SECURELY!** You'll need it to recover your node and VIBE earnings.

### 4. Start Your Peer Node

```bash
./target/release/apn_node \
  --port 4001 \
  --relay nats://nonlocal.info:4222 \
  --heartbeat-interval 30 \
  --import "YOUR_12_WORD_MNEMONIC_HERE"
```

Replace `YOUR_12_WORD_MNEMONIC_HERE` with your actual mnemonic phrase.

### 5. Verify Connection

Your node should show:
```
✅ Node started!
🌐 Relay connected
💓 Heartbeat enabled (interval: 30s)
```

---

## Running as a Background Service

### Option 1: Using nohup (Simple)

```bash
cd ~/pcg-cc-mcp
nohup ./target/release/apn_node \
  --port 4001 \
  --relay nats://nonlocal.info:4222 \
  --heartbeat-interval 30 \
  --import "YOUR_MNEMONIC" \
  > /tmp/apn_peer.log 2>&1 &

echo $! > /tmp/apn_peer.pid
```

Check logs: `tail -f /tmp/apn_peer.log`
Stop node: `kill $(cat /tmp/apn_peer.pid)`

### Option 2: Systemd Service (Recommended for 24/7 operation)

Create service file:
```bash
sudo nano /etc/systemd/system/apn-peer.service
```

Add this content (update paths and mnemonic):
```ini
[Unit]
Description=APN Core Peer Node
After=network.target

[Service]
Type=simple
User=YOUR_USERNAME
WorkingDirectory=/home/YOUR_USERNAME/pcg-cc-mcp
ExecStart=/home/YOUR_USERNAME/pcg-cc-mcp/target/release/apn_node \
  --port 4001 \
  --relay nats://nonlocal.info:4222 \
  --heartbeat-interval 30 \
  --import "YOUR_MNEMONIC"
Restart=always
RestartSec=10
StandardOutput=append:/var/log/apn-peer.log
StandardError=append:/var/log/apn-peer.log

[Install]
WantedBy=multi-user.target
```

Enable and start:
```bash
sudo systemctl daemon-reload
sudo systemctl enable apn-peer
sudo systemctl start apn-peer
sudo systemctl status apn-peer
```

View logs: `sudo journalctl -u apn-peer -f`

---

## Port Configuration

If port 4001 is already in use, choose a different port:

```bash
./target/release/apn_node \
  --port 4002 \
  --relay nats://nonlocal.info:4222 \
  --heartbeat-interval 30 \
  --import "YOUR_MNEMONIC"
```

Each node on the same machine needs a unique port.

---

## Resource Detection

Your node automatically detects and reports:
- **CPU cores**: All available cores
- **RAM**: Total and available memory
- **Storage**: Available disk space
- **GPU**: NVIDIA GPUs detected via nvidia-smi

To maximize VIBE earnings:
- Keep your node online 24/7
- Ensure GPU drivers are installed (for NVIDIA GPUs)
- Allocate sufficient storage for tasks

---

## Monitoring Your Node

### Check if running:
```bash
ps aux | grep apn_node
```

### View logs:
```bash
tail -f /tmp/apn_peer.log
# or for systemd:
sudo journalctl -u apn-peer -f
```

### Check network status:
```bash
curl http://192.168.1.77:8081/api/status | jq
```

This shows all nodes in the network (if you're on the same network as Pythia).

---

## Troubleshooting

### Node won't start
- Check if port 4001 is available: `netstat -tlnp | grep 4001`
- Verify NATS relay is reachable: `telnet nonlocal.info 4222`
- Check Rust is installed: `rustc --version`

### Can't connect to NATS relay
- Verify internet connection
- Check firewall settings
- Try telnet: `telnet nonlocal.info 4222`

### GPU not detected
- Install NVIDIA drivers: `sudo ubuntu-drivers autoinstall`
- Install nvidia-smi: `sudo apt install nvidia-utils-525`
- Verify: `nvidia-smi`

### Not earning VIBE
- Ensure node is online 24/7
- Check logs for errors
- Verify heartbeat messages are being sent
- Contact support with your node ID

---

## Security Notes

1. **Never share your mnemonic phrase** - This is your private key
2. **Backup your mnemonic** - Store it securely offline
3. **Keep your system updated**: `sudo apt update && sudo apt upgrade`
4. **Monitor your node** - Set up alerts for downtime

---

## Earning VIBE Tokens

Your node earns VIBE tokens by:
1. **Contributing compute resources** - Processing workloads for orchestrators
2. **Staying online** - Uptime rewards
3. **Providing GPU** - GPU nodes earn more
4. **Network efficiency** - Well-connected nodes earn bonuses

VIBE tokens will be distributed to your wallet address (derived from your mnemonic).

---

## Getting Help

- **Logs**: Always check logs first
- **Network status**: http://192.168.1.77:8081/api/status
- **Support**: Contact the APN team with your node ID

---

## Quick Start Script

Create `start-peer.sh`:
```bash
#!/bin/bash
cd ~/pcg-cc-mcp
./target/release/apn_node \
  --port 4001 \
  --relay nats://nonlocal.info:4222 \
  --heartbeat-interval 30 \
  --import "YOUR_MNEMONIC" \
  2>&1 | tee /tmp/apn_peer.log
```

Make executable: `chmod +x start-peer.sh`
Run: `./start-peer.sh`

---

**Welcome to the Alpha Protocol Network!** 🚀

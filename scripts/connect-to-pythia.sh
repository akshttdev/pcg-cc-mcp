#!/bin/bash

# Quick Connect Script for APN Peer Nodes
# Run this on any device to connect to the Pythia master node

set -e

echo "╔═══════════════════════════════════════════════════════════════╗"
echo "║     Connect to Pythia - Alpha Protocol Network              ║"
echo "╚═══════════════════════════════════════════════════════════════╝"
echo ""

# Bootstrap information
BOOTSTRAP="/ip4/192.168.1.77/tcp/4001/p2p/12D3KooWGaopz8uKs5ikXxD4yy5wQDn5yue2Q38T81pMtLbxMVvt"
RELAY="nats://nonlocal.info:4222"
MASTER_IP="192.168.1.77"
MASTER_PORT="4001"

echo "📋 Connection Details:"
echo "   Master Node: Pythia (apn_814d37f4)"
echo "   Bootstrap:   $BOOTSTRAP"
echo "   NATS Relay:  $RELAY"
echo ""

# Step 1: Verify master is reachable
echo "🔍 Step 1: Checking if master node is reachable..."
if ping -c 2 -W 2 $MASTER_IP >/dev/null 2>&1; then
    echo "   ✅ Master node is reachable at $MASTER_IP"
else
    echo "   ❌ Cannot reach master node at $MASTER_IP"
    echo "   💡 Check your network connection or firewall settings"
    exit 1
fi

# Step 2: Check if port is accessible
echo ""
echo "🔍 Step 2: Checking if port $MASTER_PORT is accessible..."
if timeout 2 bash -c "cat < /dev/null > /dev/tcp/$MASTER_IP/$MASTER_PORT" 2>/dev/null; then
    echo "   ✅ Port $MASTER_PORT is accessible"
else
    echo "   ⚠️  Cannot connect to port $MASTER_PORT"
    echo "   💡 Master may be behind a firewall, but relay should still work"
fi

# Step 3: Check if repo exists
echo ""
echo "🔍 Step 3: Checking repository..."
if [ ! -d "pcg-cc-mcp" ]; then
    echo "   📥 Cloning repository..."
    git clone https://github.com/KingBodhi/pcg-cc-mcp.git
    cd pcg-cc-mcp
else
    cd pcg-cc-mcp
    echo "   ✅ Repository exists"
fi

# Step 4: Update to latest
echo ""
echo "🔍 Step 4: Updating to latest code..."
git fetch origin
git checkout new
git pull origin new
echo "   ✅ Updated to latest"

# Step 5: Stop any old nodes
echo ""
echo "🔍 Step 5: Cleaning up old node processes..."
if pkill -f apn_node 2>/dev/null; then
    echo "   ✅ Stopped old node process"
    sleep 2
else
    echo "   ℹ️  No old nodes running"
fi

# Step 6: Check if binary exists or needs building
echo ""
echo "🔍 Step 6: Checking binary..."
if [ ! -f "target/release/apn_node" ]; then
    echo "   🔨 Building node binary (this may take a few minutes)..."
    cargo build --release --bin apn_node
    echo "   ✅ Build complete"
else
    echo "   ✅ Binary exists"
    echo "   💡 Rebuilding to ensure latest code..."
    cargo build --release --bin apn_node
fi

# Step 7: Get device name
echo ""
echo "🔍 Step 7: Device identification..."
read -p "   Enter a name for this device (e.g., 'OKB-Terminal'): " DEVICE_NAME
DEVICE_NAME=${DEVICE_NAME:-"Peer-$(hostname)"}
echo "   ✅ Device name: $DEVICE_NAME"

# Step 8: Start the node
echo ""
echo "🚀 Step 8: Starting node..."
LOG_FILE="/tmp/apn_peer.log"
PID_FILE="/tmp/apn_peer.pid"

nohup ./target/release/apn_node \
    --port 4002 \
    --relay $RELAY \
    --bootstrap "$BOOTSTRAP" \
    --heartbeat-interval 30 \
    > $LOG_FILE 2>&1 &

NODE_PID=$!
echo $NODE_PID > $PID_FILE

echo "   ✅ Node started with PID: $NODE_PID"
echo ""

# Step 9: Wait and verify
echo "🔍 Step 9: Verifying connection (waiting 10 seconds)..."
sleep 10

if ps -p $NODE_PID > /dev/null; then
    echo "   ✅ Node process is running"

    # Check logs for success indicators
    if grep -q "Node started" $LOG_FILE; then
        echo "   ✅ Node started successfully"
    fi

    if grep -q "Relay connected" $LOG_FILE; then
        echo "   ✅ Connected to NATS relay"
    fi

    if grep -q "Collected resources" $LOG_FILE; then
        echo "   ✅ Resources detected and reporting"
    fi

    if grep -q "Peer connected" $LOG_FILE 2>/dev/null; then
        echo "   ✅ Connected to master node"
    else
        echo "   ⏳ Waiting for peer connection (may take 30-60 seconds)..."
    fi
else
    echo "   ❌ Node process died - check logs: $LOG_FILE"
    exit 1
fi

echo ""
echo "╔═══════════════════════════════════════════════════════════════╗"
echo "║                   CONNECTION SUCCESSFUL!                      ║"
echo "╚═══════════════════════════════════════════════════════════════╝"
echo ""
echo "📊 Monitor your node:"
echo "   tail -f $LOG_FILE"
echo ""
echo "🛑 Stop your node:"
echo "   kill \$(cat $PID_FILE)"
echo ""
echo "📈 Check network capacity:"
echo "   ./check-network-capacity.sh"
echo ""
echo "🔍 View your node info:"
echo "   grep 'Node ID\\|Peer ID\\|Mnemonic' $LOG_FILE"
echo ""
echo "✅ Your device ($DEVICE_NAME) is now part of the Alpha Protocol Network!"
echo ""

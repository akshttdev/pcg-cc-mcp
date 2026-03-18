#!/bin/bash
set -e

echo "╔══════════════════════════════════════════════════════════════════╗"
echo "║                                                                  ║"
echo "║          🌟 PYTHIA MASTER NODE SETUP                             ║"
echo "║                                                                  ║"
echo "╚══════════════════════════════════════════════════════════════════╝"
echo ""

cd /home/pythia/pcg-cc-mcp

# Stop existing node if running
if [ -f /tmp/apn_node.pid ]; then
    echo "🛑 Stopping existing node..."
    ./stop-apn.sh
    sleep 2
fi

# Pull latest code
echo "📥 Pulling latest code from 'new' branch..."
git pull origin new

# Build the binary
echo "🔨 Building apn_node binary (this may take a few minutes)..."
cargo build --release --bin apn_node

# Check if binary exists
if [ ! -f ./target/release/apn_node ]; then
    echo "❌ Build failed - binary not found"
    exit 1
fi

echo "✅ Binary built successfully!"
echo ""

# Start Pythia Master Node
echo "🚀 Starting Pythia Master Node..."
echo ""

./target/release/apn_node \
  --port 4001 \
  --relay nats://nonlocal.info:4222 \
  > /tmp/apn_node.log 2>&1 &

echo $! > /tmp/apn_node.pid

sleep 3

echo "✅ Pythia Master Node started!"
echo ""

# Show node info
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
grep -A 5 "Node ID:" /tmp/apn_node.log || tail -20 /tmp/apn_node.log
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""

# Get public IP
PUBLIC_IP=$(curl -s https://api.ipify.org 2>/dev/null || echo "UNKNOWN")

# Extract peer ID from logs
PEER_ID=$(grep "Local peer ID:" /tmp/apn_node.log | awk '{print $NF}')

echo "📝 SAVE THESE CONNECTION DETAILS:"
echo ""
echo "  Bootstrap Multiaddr:"
echo "  /ip4/$PUBLIC_IP/tcp/4001/p2p/$PEER_ID"
echo ""
echo "  For Device 3 to connect:"
echo "  ./target/release/apn_node --bootstrap /ip4/$PUBLIC_IP/tcp/4001/p2p/$PEER_ID"
echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""
echo "🎉 Pythia Master Node is now seeding the network!"
echo ""
echo "Monitor: ./monitor-apn.sh"
echo "Logs:    tail -f /tmp/apn_node.log"


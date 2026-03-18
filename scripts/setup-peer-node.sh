#!/bin/bash
set -e

echo "╔══════════════════════════════════════════════════════════════════╗"
echo "║                                                                  ║"
echo "║          🔌 PEER NODE SETUP - Enhanced Features                 ║"
echo "║                                                                  ║"
echo "╚══════════════════════════════════════════════════════════════════╝"
echo ""

# Check if we're in the right directory
if [ ! -f "Cargo.toml" ]; then
    echo "❌ Error: Not in pcg-cc-mcp directory"
    echo "   Run: cd /path/to/pcg-cc-mcp"
    exit 1
fi

# Pull latest code
echo "📥 Pulling latest code from 'new' branch..."
git fetch origin
git checkout new
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

# Get master node multiaddr from user
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "Enter Pythia Master Node bootstrap address:"
echo "(Example: /ip4/153.66.61.104/tcp/4001/p2p/12D3KooW...)"
echo ""
read -p "Bootstrap address: " BOOTSTRAP_ADDR

if [ -z "$BOOTSTRAP_ADDR" ]; then
    echo "❌ No bootstrap address provided"
    exit 1
fi

# Get device name
echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
read -p "Enter device name (e.g., OKB-Terminal, Device-3): " DEVICE_NAME

if [ -z "$DEVICE_NAME" ]; then
    DEVICE_NAME="peer-$(hostname)"
fi

# Use different port to avoid conflicts
PORT=4002

# Stop any existing node
echo ""
echo "🛑 Stopping any existing APN nodes..."
pkill -f "apn_node" || true
sleep 2

echo ""
echo "🚀 Starting $DEVICE_NAME with resource reporting..."
echo ""

./target/release/apn_node \
  --port $PORT \
  --relay nats://nonlocal.info:4222 \
  --bootstrap "$BOOTSTRAP_ADDR" \
  --heartbeat-interval 30 \
  > /tmp/apn_peer.log 2>&1 &

echo $! > /tmp/apn_peer.pid

sleep 4

echo "✅ $DEVICE_NAME started!"
echo ""

# Show node info
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
tail -30 /tmp/apn_peer.log
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""

echo "🎉 $DEVICE_NAME is now connecting to Pythia Master Node!"
echo ""
echo "📊 Features enabled:"
echo "  ✅ Resource reporting (CPU, RAM, GPU, Storage)"
echo "  ✅ Heartbeat broadcasts (30s interval)"
echo "  ✅ NATS relay for NAT traversal"
echo "  ✅ libp2p mesh networking"
echo ""
echo "Monitor: tail -f /tmp/apn_peer.log"
echo "Stop:    kill \$(cat /tmp/apn_peer.pid)"
echo ""

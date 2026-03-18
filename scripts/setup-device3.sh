#!/bin/bash
set -e

echo "╔══════════════════════════════════════════════════════════════════╗"
echo "║                                                                  ║"
echo "║          🔌 DEVICE 3 - PEER NODE SETUP                           ║"
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

echo ""
echo "🚀 Starting Device 3 peer node..."
echo ""

./target/release/apn_node \
  --port 4002 \
  --relay nats://nonlocal.info:4222 \
  --bootstrap "$BOOTSTRAP_ADDR" \
  > /tmp/apn_device3.log 2>&1 &

echo $! > /tmp/apn_device3.pid

sleep 3

echo "✅ Device 3 peer node started!"
echo ""

# Show node info
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
grep -A 5 "Node ID:" /tmp/apn_device3.log || tail -20 /tmp/apn_device3.log
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""

echo "🎉 Device 3 is now connecting to Pythia Master Node!"
echo ""
echo "Monitor: tail -f /tmp/apn_device3.log"
echo "Stop:    kill $(cat /tmp/apn_device3.pid)"


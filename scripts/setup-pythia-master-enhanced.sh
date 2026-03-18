#!/bin/bash
set -e

echo "╔══════════════════════════════════════════════════════════════════╗"
echo "║                                                                  ║"
echo "║  🚀 PYTHIA MASTER NODE - Enhanced with Resource Reporting       ║"
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
echo "🔨 Building apn_node with resource reporting (this may take a few minutes)..."
cargo build --release --bin apn_node

# Check if binary exists
if [ ! -f ./target/release/apn_node ]; then
    echo "❌ Build failed - binary not found"
    exit 1
fi

echo "✅ Binary built successfully with resource reporting!"
echo ""

# Stop any existing node
echo "🛑 Stopping any existing APN nodes..."
pkill -f "apn_node" || true
sleep 2

# Start the enhanced node
echo "🚀 Starting Pythia Master Node with resource reporting..."
echo ""

./target/release/apn_node \
  --port 4001 \
  --relay nats://nonlocal.info:4222 \
  --heartbeat-interval 30 \
  > /tmp/apn_node.log 2>&1 &

echo $! > /tmp/apn_node.pid

sleep 4

# Show node info
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
tail -30 /tmp/apn_node.log
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""

# Save master node info
NODE_ID=$(grep "Node ID:" /tmp/apn_node.log | awk '{print $4}')
WALLET=$(grep "Address:" /tmp/apn_node.log | awk '{print $4}')
LIBP2P=$(grep "Node started:" /tmp/apn_node.log | awk '{print $4}')
MNEMONIC=$(grep -A 1 "Mnemonic (save this for recovery):" /tmp/apn_node.log | tail -1 | xargs)

# Get server IP
SERVER_IP=$(hostname -I | awk '{print $1}')

cat > PYTHIA-MASTER-INFO.txt << EOF
═══════════════════════════════════════════════════════════════════
              🌟 PYTHIA MASTER NODE CONNECTION INFO
═══════════════════════════════════════════════════════════════════

MASTER NODE IDENTITY:
  Name:       Pythia Master Node
  Node ID:    $NODE_ID
  Wallet:     $WALLET
  LibP2P ID:  $LIBP2P

RECOVERY PHRASE:
  $MNEMONIC

BOOTSTRAP MULTIADDR:
  /ip4/$SERVER_IP/tcp/4001/p2p/$LIBP2P

───────────────────────────────────────────────────────────────────

FOR ALL DEVICES TO CONNECT:

./target/release/apn_node \\
  --port 4002 \\
  --relay nats://nonlocal.info:4222 \\
  --bootstrap /ip4/$SERVER_IP/tcp/4001/p2p/$LIBP2P \\
  --heartbeat-interval 30

═══════════════════════════════════════════════════════════════════

FEATURES ENABLED:
  ✅ Resource reporting (CPU, RAM, GPU, Storage)
  ✅ Heartbeat broadcasts (30s interval)
  ✅ NATS relay for NAT traversal
  ✅ libp2p mesh networking

EOF

echo "✅ Pythia Master Node started with enhanced features!"
echo ""
echo "📄 Connection info saved to: PYTHIA-MASTER-INFO.txt"
echo ""
echo "Monitor: tail -f /tmp/apn_node.log"
echo "Stop:    kill \$(cat /tmp/apn_node.pid)"
echo ""

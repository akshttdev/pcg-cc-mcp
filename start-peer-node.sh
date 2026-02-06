#!/bin/bash
# Start APN Peer Node - Join the Sovereign Mesh

echo "🏴 Starting APN Peer Node - Sovereign Stack"
echo ""

cd "$(dirname "$0")"

# Build if needed
if [ ! -f "./target/release/apn_node" ]; then
    echo "Building APN node..."
    cargo build --release -p alpha-protocol-core
fi

# Check if running
if lsof -i :4001 >/dev/null 2>&1; then
    echo "⚠️  Port 4001 already in use"
    echo "Stop existing node: kill $(cat /tmp/apn_peer.pid 2>/dev/null)"
    exit 1
fi

echo "Starting peer node..."
echo "  NATS: nats://nonlocal.info:4222"
echo "  Port: 4001"
echo ""

# Start node (generates new peer ID automatically)
nohup ./target/release/apn_node \
    --port 4001 \
    --relay nats://nonlocal.info:4222 \
    --heartbeat-interval 30 \
    > /tmp/apn_peer.log 2>&1 &

echo $! > /tmp/apn_peer.pid
sleep 3

if ps -p $(cat /tmp/apn_peer.pid) > /dev/null; then
    echo "✅ Peer node started!"
    echo ""
    echo "PID:  $(cat /tmp/apn_peer.pid)"
    echo "Logs: tail -f /tmp/apn_peer.log"
    echo ""
    tail -10 /tmp/apn_peer.log
else
    echo "❌ Failed to start"
    cat /tmp/apn_peer.log
    exit 1
fi

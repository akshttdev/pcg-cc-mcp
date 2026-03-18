#!/bin/bash
# Pythia Master Node Startup Script
# This is the stable orchestrator for the Alpha Protocol Network

set -e

cd /home/pythia/pcg-cc-mcp

echo "╔══════════════════════════════════════════════════════════╗"
echo "║   Starting Pythia Master Node                            ║"
echo "╚══════════════════════════════════════════════════════════╝"

# Kill any existing instances
echo "→ Cleaning up old processes..."
pkill -9 apn_node 2>/dev/null || true
pkill -9 apn_api_server 2>/dev/null || true
sleep 2

# Start Master Node
echo "→ Starting APN Master Node..."
nohup ./target/release/apn_node \
  --port 4001 \
  --relay nats://nonlocal.info:4222 \
  --heartbeat-interval 30 \
  --import "slush index float shaft flight citizen swear chunk correct veteran eyebrow blind" \
  > /tmp/apn_node.log 2>&1 &

echo $! > /tmp/apn_master.pid
sleep 3

# Start API Server
echo "→ Starting API Server..."
./target/release/apn_api_server > /tmp/apn_api.log 2>&1 &
echo $! > /tmp/apn_api.pid
sleep 2

echo ""
echo "✅ Pythia Master Node Started!"
echo ""
echo "📊 Master Node:"
echo "   Process ID: $(cat /tmp/apn_master.pid)"
echo "   LibP2P Port: 4001"
echo "   NATS Relay: nats://nonlocal.info:4222"
echo "   Logs: tail -f /tmp/apn_node.log"
echo ""
echo "📡 API Server:"
echo "   Process ID: $(cat /tmp/apn_api.pid)"
echo "   API Endpoint: http://192.168.1.77:8081/api/status"
echo "   Web Dashboard: http://192.168.1.77:8081/"
echo "   Logs: tail -f /tmp/apn_api.log"
echo ""
echo "🛑 To stop: pkill apn_node; pkill apn_api_server"
echo ""

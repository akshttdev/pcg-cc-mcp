#!/bin/bash
# Monitor APN Master Node

echo "🔍 APN Master Node Monitor"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""

# Check if running
if [ -f /tmp/apn_node.pid ]; then
    PID=$(cat /tmp/apn_node.pid)
    if ps -p $PID > /dev/null 2>&1; then
        echo "✅ Node Status: RUNNING (PID: $PID)"
    else
        echo "❌ Node Status: NOT RUNNING (stale PID file)"
        rm /tmp/apn_node.pid
        exit 1
    fi
else
    echo "❌ Node Status: NOT RUNNING"
    exit 1
fi

echo ""
echo "📊 Node Info:"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
grep -E "Node ID:|Address:|Peer ID:" /tmp/apn_node.log | head -3
echo ""

echo "🌐 Network Status:"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
if grep -q "NATS relay" /tmp/apn_node.log; then
    echo "✅ NATS Relay: Connected"
else
    echo "❌ NATS Relay: Not connected"
fi

if grep -q "Listening on port 4001" /tmp/apn_node.log; then
    echo "✅ P2P Port: 4001 (Listening)"
else
    echo "❌ P2P Port: Not listening"
fi

echo ""
echo "👥 Recent Activity (last 10 lines):"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
tail -10 /tmp/apn_node.log | grep -v "^$"
echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""
echo "Commands:"
echo "  Watch logs:  tail -f /tmp/apn_node.log"
echo "  Stop node:   ./stop-apn.sh"
echo "  Restart:     ./stop-apn.sh && ./start-apn-node.sh"

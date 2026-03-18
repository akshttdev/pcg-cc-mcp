#!/bin/bash

# Start APN API Server for remote network monitoring
# Allows any device on the network to view APN nodes

cd /home/pythia/pcg-cc-mcp

./target/release/apn_api_server > /tmp/apn_api.log 2>&1 &

echo $! > /tmp/apn_api.pid

echo "✅ APN API Server started"
echo ""
echo "📊 Access the dashboard:"
echo "   Local:  http://localhost:8080"
echo "   Network: http://192.168.1.77:8080"
echo ""
echo "📡 API endpoint:"
echo "   http://192.168.1.77:8080/api/status"
echo ""
echo "📝 Logs: tail -f /tmp/apn_api.log"
echo "🛑 Stop:  kill \$(cat /tmp/apn_api.pid)"

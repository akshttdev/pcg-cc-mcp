#!/bin/bash

# Start Pythia Master Node for Alpha Protocol Network
# This script starts the master node with the existing identity

cd /home/pythia/pcg-cc-mcp

./target/release/apn_node \
  --port 4001 \
  --relay nats://nonlocal.info:4222 \
  --heartbeat-interval 30 \
  --import "slush index float shaft flight citizen swear chunk correct veteran eyebrow blind" \
  > /tmp/apn_node.log 2>&1 &

echo "Master node started with PID: $!"
echo "View logs: tail -f /tmp/apn_node.log"

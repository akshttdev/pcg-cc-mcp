#!/bin/bash
# Start the real APN node as a master node

cd /home/pythia/pcg-cc-mcp

echo "🚀 Starting Real APN Master Node"
echo ""

# Run the APN node
./target/release/apn_node \
  --port 4001 \
  --relay nats://nonlocal.info:4222

# This will:
# - Listen on port 4001 for peer connections
# - Connect to NATS relay for peer discovery
# - Generate/use a node identity
# - Become discoverable by other peers

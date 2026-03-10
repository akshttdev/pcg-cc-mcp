#!/bin/bash
set -a
source /home/pythia/pcg-cc-mcp/.env
set +a

# Kill any existing APN node instances to prevent duplicates
pkill -f "apn_node --port" 2>/dev/null
sleep 1

exec /home/pythia/pcg-cc-mcp/target/release/apn_node \
    --port "${APN_PORT:-4001}" \
    --relay "${NATS_RELAY:-nats://nonlocal.info:4222}" \
    --heartbeat-interval "${APN_HEARTBEAT_INTERVAL:-30}" \
    --import $MASTER_WALLET_SEED

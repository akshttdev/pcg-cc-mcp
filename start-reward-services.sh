#!/bin/bash
# Start Reward Services (Tracker + Distributor)
# These services should only run on the master node

set -e

cd /home/pythia/pcg-cc-mcp

echo "╔══════════════════════════════════════════════════════════╗"
echo "║   Starting Peer Reward Services                          ║"
echo "╚══════════════════════════════════════════════════════════╝"

# Check if .env exists
if [ ! -f .env ]; then
    echo "❌ .env file not found!"
    echo "Please create .env with:"
    echo "  REWARDS_WALLET_SEED=\"your 12-word mnemonic\""
    echo "  REWARDS_WALLET_ADDRESS=\"0x...\""
    exit 1
fi

# Source .env
set -a
source .env
set +a

# Verify rewards wallet is configured
if [ -z "$REWARDS_WALLET_SEED" ]; then
    echo "❌ REWARDS_WALLET_SEED not set in .env"
    exit 1
fi

# Kill any existing instances
echo "→ Cleaning up old processes..."
pkill -9 apn_reward_tracker 2>/dev/null || true
pkill -9 apn_reward_distributor 2>/dev/null || true
sleep 2

# Build if binaries don't exist
if [ ! -f "./target/release/apn_reward_tracker" ] || [ ! -f "./target/release/apn_reward_distributor" ]; then
    echo "→ Building reward service binaries..."
    cargo build --release --bin apn_reward_tracker --bin apn_reward_distributor
fi

# Start Reward Tracker
echo "→ Starting Reward Tracker..."
nohup ./target/release/apn_reward_tracker \
  > /tmp/apn_reward_tracker.log 2>&1 &
echo $! > /tmp/apn_reward_tracker.pid
sleep 3

# Check if tracker started successfully
if ps -p $(cat /tmp/apn_reward_tracker.pid) > /dev/null; then
    echo "✅ Reward Tracker started (PID: $(cat /tmp/apn_reward_tracker.pid))"
else
    echo "❌ Reward Tracker failed to start"
    cat /tmp/apn_reward_tracker.log
    exit 1
fi

# Start Reward Distributor
echo "→ Starting Reward Distributor..."
nohup ./target/release/apn_reward_distributor \
  > /tmp/apn_reward_distributor.log 2>&1 &
echo $! > /tmp/apn_reward_distributor.pid
sleep 3

# Check if distributor started successfully
if ps -p $(cat /tmp/apn_reward_distributor.pid) > /dev/null; then
    echo "✅ Reward Distributor started (PID: $(cat /tmp/apn_reward_distributor.pid))"
else
    echo "❌ Reward Distributor failed to start"
    cat /tmp/apn_reward_distributor.log
    exit 1
fi

echo ""
echo "✅ Reward Services Started!"
echo ""
echo "📊 Reward Tracker:"
echo "   Process ID: $(cat /tmp/apn_reward_tracker.pid)"
echo "   Logs: tail -f /tmp/apn_reward_tracker.log"
echo ""
echo "💸 Reward Distributor:"
echo "   Process ID: $(cat /tmp/apn_reward_distributor.pid)"
echo "   Logs: tail -f /tmp/apn_reward_distributor.log"
echo ""
echo "🛑 To stop: pkill apn_reward_tracker; pkill apn_reward_distributor"
echo ""

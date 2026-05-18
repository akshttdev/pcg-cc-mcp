#!/bin/bash
cd /home/pythia/pcg-cc-mcp
export PATH="/home/pythia/.nvm/versions/node/v22.20.0/bin:/home/pythia/.local/bin:/home/pythia/miniconda3/bin:/usr/local/bin:/usr/bin:/bin"
set -a
source /home/pythia/pcg-cc-mcp/.env
set +a

# Kill any stale server process on our port before starting
PORT="${FRONTEND_PORT:-3000}"
STALE_PID=$(fuser "${PORT}/tcp" 2>/dev/null | tr -s ' ' '\n' | grep -v '^$' | head -1)
if [ -n "$STALE_PID" ]; then
  echo "Killing stale server process $STALE_PID on port $PORT"
  kill "$STALE_PID" 2>/dev/null
  sleep 1
fi

exec ./target/debug/server

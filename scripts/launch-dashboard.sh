#!/bin/bash
# PCG Dashboard Launcher - Starts services and opens dashboard in browser

echo "╔══════════════════════════════════════════════════════════╗"
echo "║  PCG Dashboard Launcher                                 ║"
echo "╚══════════════════════════════════════════════════════════╝"
echo ""

# Get script directory
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# Ensure all services use the project-local database
export PCG_ASSET_DIR="$SCRIPT_DIR/dev_assets"

# Start all agent services first
echo "→ Initializing all dashboard services..."
"$SCRIPT_DIR/start-all-services.sh"
echo ""

# Check if PCG backend server is running
if lsof -i :58297 >/dev/null 2>&1; then
    echo "✓ PCG backend server already running on port 58297"
else
    echo "→ Starting PCG backend server on port 58297..."
    cd "$SCRIPT_DIR"
    # Set BACKEND_PORT to 58297 to match nginx configuration
    # Disable auto-start of ComfyUI to prevent server from hanging
    # Configure Topsi to use Ollama for LLM (qwen2.5:7b supports function calling)
    AUTO_START_COMFYUI=false BACKEND_PORT=58297 \
    PCG_ASSET_DIR="$SCRIPT_DIR/dev_assets" \
    TOPSI_LLM_PROVIDER=ollama TOPSI_LLM_MODEL=qwen2.5:7b \
    OLLAMA_BASE_URL=http://localhost:11434 \
    nohup ./target/release/server > /tmp/pcg_backend.log 2>&1 &
    SERVER_PID=$!
    echo $SERVER_PID > /tmp/pcg_backend.pid
    sleep 4
    # Verify it started successfully
    if lsof -i :58297 >/dev/null 2>&1; then
        echo "✓ PCG backend server started (PID: $SERVER_PID)"
    else
        echo "❌ Failed to start backend server. Check /tmp/pcg_backend.log"
    fi
fi

# Check if Python bridge server is running
if lsof -i :8000 >/dev/null 2>&1; then
    echo "✓ APN bridge server already running on port 8000"
else
    echo "→ Starting APN bridge server..."
    cd "$SCRIPT_DIR"
    python3 apn_bridge_server.py > /tmp/apn_bridge.log 2>&1 &
    echo $! > /tmp/apn_bridge.pid
    sleep 2
    echo "✓ APN bridge server started (PID: $(cat /tmp/apn_bridge.pid))"
fi

# Check if APN node is running
if pgrep -f "apn_node" >/dev/null 2>&1; then
    echo "✓ APN node already running"
else
    echo "→ Starting APN node..."
    cd "$SCRIPT_DIR"
    ./target/release/apn_node \
        --port 4001 \
        --relay nats://nonlocal.info:4222 \
        --heartbeat-interval 30 \
        --import "slush index float shaft flight citizen swear chunk correct veteran eyebrow blind" \
        > /tmp/apn_node_dashboard.log 2>&1 &
    echo $! > /tmp/apn_node_dashboard.pid
    sleep 3
    echo "✓ APN node started (PID: $(cat /tmp/apn_node_dashboard.pid))"
fi

# Check if frontend dev server is running
if lsof -i :3000 >/dev/null 2>&1; then
    echo "✓ Frontend server already running on port 3000"
else
    echo "→ Starting frontend dev server..."
    cd "$SCRIPT_DIR/frontend"
    VITE_OPEN=false npm run dev > /tmp/dashboard_frontend.log 2>&1 &
    echo $! > /tmp/dashboard_frontend.pid
    sleep 5
    echo "✓ Frontend server started (PID: $(cat /tmp/dashboard_frontend.pid))"
fi

echo ""
echo "╔══════════════════════════════════════════════════════════╗"
echo "║  ✅ PCG Dashboard is ready!                             ║"
echo "╠══════════════════════════════════════════════════════════╣"
echo "║  Opening dashboard in your browser...                   ║"
echo "║                                                          ║"
echo "║  URL: http://dashboard.powerclubglobal.com              ║"
echo "║  Local: http://localhost:3000                           ║"
echo "║                                                          ║"
echo "║  To stop services, use:                                 ║"
echo "║  ./stop-dashboard.sh                                    ║"
echo "╚══════════════════════════════════════════════════════════╝"
echo ""

# Open dashboard in default browser
sleep 2

DASHBOARD_URL="http://dashboard.powerclubglobal.com"

if command -v xdg-open >/dev/null 2>&1; then
    xdg-open "$DASHBOARD_URL" 2>/dev/null
elif command -v gnome-open >/dev/null 2>&1; then
    gnome-open "$DASHBOARD_URL" 2>/dev/null
elif command -v open >/dev/null 2>&1; then
    open "$DASHBOARD_URL" 2>/dev/null
else
    echo "→ Please open $DASHBOARD_URL in your browser"
fi

echo "→ Dashboard launched! Window will stay open..."
echo "→ Press Ctrl+C to view logs or close this window anytime"
echo ""

# Keep the script running to show it's active
if [ -f /tmp/dashboard_frontend.log ]; then
    tail -f /tmp/dashboard_frontend.log
else
    echo "✓ Dashboard is running in the background"
    echo "  To view logs: tail -f /tmp/nextjs-dev.log"
    echo ""
    echo "Press Enter to close this window..."
    read
fi

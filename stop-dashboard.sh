#!/bin/bash
# Stop all PCG Dashboard services

echo "╔══════════════════════════════════════════════════════════╗"
echo "║  Stopping PCG Dashboard Services                        ║"
echo "╚══════════════════════════════════════════════════════════╝"
echo ""

# Stop frontend dev server
if [ -f /tmp/dashboard_frontend.pid ]; then
    PID=$(cat /tmp/dashboard_frontend.pid)
    if ps -p $PID > /dev/null 2>&1; then
        echo "→ Stopping frontend server (PID: $PID)..."
        kill $PID 2>/dev/null
        rm /tmp/dashboard_frontend.pid
        echo "✓ Frontend server stopped"
    fi
fi

# Stop PCG backend server
if [ -f /tmp/pcg_backend.pid ]; then
    PID=$(cat /tmp/pcg_backend.pid)
    if ps -p $PID > /dev/null 2>&1; then
        echo "→ Stopping PCG backend server (PID: $PID)..."
        kill $PID 2>/dev/null
        rm /tmp/pcg_backend.pid
        echo "✓ PCG backend server stopped"
    fi
fi

# Stop APN bridge server
if [ -f /tmp/apn_bridge.pid ]; then
    PID=$(cat /tmp/apn_bridge.pid)
    if ps -p $PID > /dev/null 2>&1; then
        echo "→ Stopping APN bridge server (PID: $PID)..."
        kill $PID 2>/dev/null
        rm /tmp/apn_bridge.pid
        echo "✓ APN bridge server stopped"
    fi
fi

# Note: We don't automatically stop APN node as it may be used by other services
echo ""
echo "✅ Dashboard services stopped"
echo ""
echo "Note: APN node was left running for other services."
echo "To stop it manually, use: pkill -f apn_node"

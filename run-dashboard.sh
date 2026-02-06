#!/bin/bash
# Launch PCG Dashboard Desktop Application

echo "╔══════════════════════════════════════════════════════════╗"
echo "║  Starting PCG Dashboard                                 ║"
echo "╚══════════════════════════════════════════════════════════╝"
echo ""

# Get the script directory
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# Check if binary exists
BINARY="$SCRIPT_DIR/target/release/pcg-dashboard"

if [ ! -f "$BINARY" ]; then
    echo "⚠️  Dashboard not built yet. Building now..."
    "$SCRIPT_DIR/build-dashboard.sh"
fi

# Launch the dashboard
echo "→ Launching PCG Dashboard..."
"$BINARY"

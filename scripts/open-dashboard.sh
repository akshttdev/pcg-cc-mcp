#!/bin/bash
# Quick launcher to open PCG Dashboard in browser
# For users who just want to access the dashboard without managing services

echo "╔══════════════════════════════════════════════════════════╗"
echo "║  Opening PCG Dashboard                                  ║"
echo "╚══════════════════════════════════════════════════════════╝"
echo ""

DASHBOARD_URL="http://dashboard.powerclubglobal.com"

echo "→ Opening: $DASHBOARD_URL"
echo ""

if command -v xdg-open >/dev/null 2>&1; then
    xdg-open "$DASHBOARD_URL" 2>/dev/null
    echo "✓ Dashboard opened in your browser"
elif command -v gnome-open >/dev/null 2>&1; then
    gnome-open "$DASHBOARD_URL" 2>/dev/null
    echo "✓ Dashboard opened in your browser"
elif command -v open >/dev/null 2>&1; then
    open "$DASHBOARD_URL" 2>/dev/null
    echo "✓ Dashboard opened in your browser"
else
    echo "→ Please manually open: $DASHBOARD_URL"
fi

echo ""
echo "Alternative access:"
echo "  • http://dashboard.powerclubglobal.com"
echo "  • http://192.168.1.77:3000 (local network)"
echo "  • http://localhost:3000 (if on master node)"

#!/bin/bash

# Simple health check script for the deployed application
# Usage: ./health-check.sh [URL]

URL="${1:-http://localhost:3001}"

echo "🏥 Health Check for PCG-CC-MCP"
echo "================================"
echo "Target: $URL"
echo ""

# Check if curl is available
if ! command -v curl &> /dev/null; then
    echo "❌ curl is not installed"
    exit 1
fi

# Check main API endpoint
echo -n "Checking API endpoint... "
HTTP_CODE=$(curl -s -o /dev/null -w "%{http_code}" "$URL/api/auth/me")
if [ "$HTTP_CODE" -eq 200 ] || [ "$HTTP_CODE" -eq 401 ]; then
    echo "✅ OK (HTTP $HTTP_CODE)"
else
    echo "❌ Failed (HTTP $HTTP_CODE)"
    exit 1
fi

# Check if Docker is being used
if command -v docker &> /dev/null; then
    echo ""
    echo "Docker Status:"
    echo "--------------"
    
    # Check if containers are running
    if docker-compose ps | grep -q "pcg-cc-mcp"; then
        echo "✅ Application container is running"
    else
        echo "⚠️  Application container not found"
    fi
    
    if docker-compose ps | grep -q "cloudflared"; then
        echo "✅ Cloudflared tunnel is running"
    else
        echo "⚠️  Cloudflared not running (local deployment?)"
    fi
    
    echo ""
    echo "Container Details:"
    docker-compose ps
fi

echo ""
echo "✅ Health check complete!"

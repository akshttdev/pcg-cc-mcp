#!/bin/bash
# manual-update.sh - Manually update to a specific version or latest release
set -e

VERSION="${1:-latest}"
GITHUB_REPO="${GITHUB_REPO:-powerclubglobal/pcg-cc-mcp}"

echo "=========================================="
echo "PCG-CC-MCP Manual Update"
echo "=========================================="

# Resolve "latest" to actual version number
if [ "$VERSION" = "latest" ]; then
    echo "Fetching latest release version..."
    VERSION=$(curl -sS \
        ${GITHUB_TOKEN:+-H "Authorization: token ${GITHUB_TOKEN}"} \
        -H "Accept: application/vnd.github+json" \
        "https://api.github.com/repos/${GITHUB_REPO}/releases/latest" | \
        grep -o '"tag_name": *"[^"]*"' | head -1 | cut -d'"' -f4 | tr -d 'v')

    if [ -z "$VERSION" ]; then
        echo "ERROR: Could not fetch latest version"
        exit 1
    fi
fi

echo "Target version: v${VERSION}"
echo "Repository: ${GITHUB_REPO}"
echo "=========================================="

# Export for docker compose build args
export RELEASE_VERSION="$VERSION"
export GITHUB_REPO="$GITHUB_REPO"

# Rebuild with new version
echo ""
echo "Building with version v${VERSION}..."
docker compose build --no-cache app

echo ""
echo "Restarting containers..."
docker compose up -d app

echo ""
echo "=========================================="
echo "Update complete!"
echo "Version: v${VERSION}"
echo "=========================================="

# Show version from running container
echo ""
echo "Verifying deployment..."
sleep 5
docker exec pcg-cc-mcp cat /app/VERSION 2>/dev/null || echo "(Container not yet ready)"

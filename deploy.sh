#!/bin/bash
set -e

echo "🚀 PCG-CC-MCP Docker Deployment Script"
echo "========================================"
echo ""

# Check if Docker is installed
if ! command -v docker &> /dev/null; then
    echo "❌ Docker is not installed. Please install Docker first."
    exit 1
fi

# Check if Docker Compose is installed
if ! command -v docker-compose &> /dev/null; then
    echo "❌ Docker Compose is not installed. Please install Docker Compose first."
    exit 1
fi

# Check if .env file exists
if [ ! -f .env ]; then
    echo "⚠️  No .env file found. Creating from .env.example..."
    cp .env.example .env
    echo ""
    echo "📝 Please edit .env and add your CLOUDFLARE_TUNNEL_TOKEN"
    echo "   Get your token from: https://one.dash.cloudflare.com/"
    echo ""
    echo "Press Enter when you've added your token to .env..."
    read
fi

# Source the .env file to check for token
source .env

if [ -z "$CLOUDFLARE_TUNNEL_TOKEN" ] || [ "$CLOUDFLARE_TUNNEL_TOKEN" = "your_cloudflare_tunnel_token_here" ]; then
    echo "❌ CLOUDFLARE_TUNNEL_TOKEN is not set in .env file"
    echo "   Please get your token from: https://one.dash.cloudflare.com/"
    exit 1
fi

echo "✅ Configuration verified"
echo ""

# Ask what to do
echo "What would you like to do?"
echo "1) Build and start (fresh deployment)"
echo "2) Start existing containers"
echo "3) Stop containers"
echo "4) View logs"
echo "5) Rebuild (clean build)"
echo ""
read -p "Enter choice [1-5]: " choice

case $choice in
    1)
        echo ""
        echo "🔨 Building and starting containers..."
        docker-compose build
        docker-compose up -d
        echo ""
        echo "✅ Deployment complete!"
        echo ""
        echo "📊 Check status: docker-compose ps"
        echo "📝 View logs: docker-compose logs -f"
        echo "🌐 Access your app via the Cloudflare Tunnel URL you configured"
        ;;
    2)
        echo ""
        echo "▶️  Starting containers..."
        docker-compose up -d
        echo "✅ Containers started!"
        ;;
    3)
        echo ""
        echo "⏹️  Stopping containers..."
        docker-compose down
        echo "✅ Containers stopped!"
        ;;
    4)
        echo ""
        echo "📝 Showing logs (Ctrl+C to exit)..."
        docker-compose logs -f
        ;;
    5)
        echo ""
        echo "🧹 Cleaning and rebuilding..."
        docker-compose down
        docker-compose build --no-cache
        docker-compose up -d
        echo "✅ Clean rebuild complete!"
        ;;
    *)
        echo "❌ Invalid choice"
        exit 1
        ;;
esac

echo ""
echo "📊 Container status:"
docker-compose ps

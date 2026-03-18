#!/bin/bash
set -e

# Colors
GREEN='\033[0;32m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m'

clear

echo -e "${CYAN}"
cat << "EOF"
╔═══════════════════════════════════════════════════════════╗
║                                                           ║
║     ██████╗  ██████╗ ██████╗        ██████╗ ██████╗      ║
║     ██╔══██╗██╔════╝██╔════╝       ██╔════╝██╔════╝      ║
║     ██████╔╝██║     ██║  ███╗█████╗██║     ██║           ║
║     ██╔═══╝ ██║     ██║   ██║╚════╝██║     ██║           ║
║     ██║     ╚██████╗╚██████╔╝      ╚██████╗╚██████╗      ║
║     ╚═╝      ╚═════╝ ╚═════╝        ╚═════╝ ╚═════╝      ║
║                                                           ║
║            PCG-CC-MCP Dashboard Quick Deploy             ║
║                                                           ║
╚═══════════════════════════════════════════════════════════╝
EOF
echo -e "${NC}"

echo ""
echo -e "${BLUE}🚀 Welcome to PCG-CC-MCP Quick Deploy${NC}"
echo -e "${BLUE}This script will help you deploy the dashboard in minutes${NC}"
echo ""

# Function to print step
print_step() {
    echo ""
    echo -e "${CYAN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
    echo -e "${CYAN}$1${NC}"
    echo -e "${CYAN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
    echo ""
}

# Function to print success
print_success() {
    echo -e "${GREEN}✅ $1${NC}"
}

# Function to print error
print_error() {
    echo -e "${RED}❌ $1${NC}"
}

# Function to print warning
print_warning() {
    echo -e "${YELLOW}⚠️  $1${NC}"
}

# Function to print info
print_info() {
    echo -e "${BLUE}ℹ️  $1${NC}"
}

# Check prerequisites
print_step "STEP 1: Checking Prerequisites"

PREREQ_OK=true

echo -n "Checking Docker... "
if command -v docker &> /dev/null; then
    print_success "Installed ($(docker --version | cut -d' ' -f3 | cut -d',' -f1))"
else
    print_error "Not installed"
    PREREQ_OK=false
fi

echo -n "Checking Docker Compose... "
if command -v docker-compose &> /dev/null || docker compose version &> /dev/null; then
    print_success "Installed"
else
    print_error "Not installed"
    PREREQ_OK=false
fi

echo -n "Checking Docker daemon... "
if docker ps &> /dev/null; then
    print_success "Running"
else
    print_error "Not running"
    PREREQ_OK=false
fi

echo -n "Checking for GPU... "
if command -v nvidia-smi &> /dev/null; then
    GPU_NAME=$(nvidia-smi --query-gpu=name --format=csv,noheader | head -1)
    print_success "Found ($GPU_NAME)"
else
    print_info "No GPU (will use CPU)"
fi

if [ "$PREREQ_OK" = false ]; then
    echo ""
    print_error "Please install missing prerequisites"
    print_info "See DEPLOYMENT.md for installation instructions"
    exit 1
fi

echo ""
print_success "All prerequisites satisfied!"

# Environment setup
print_step "STEP 2: Environment Configuration"

if [ -f .env ]; then
    print_info "Found existing .env file"
    echo ""
    read -p "Do you want to reconfigure? [y/N]: " reconfigure
    if [[ ! $reconfigure =~ ^[Yy]$ ]]; then
        print_info "Using existing configuration"
    else
        rm .env
    fi
fi

if [ ! -f .env ]; then
    echo ""
    print_info "Creating environment configuration..."
    cp .env.example .env

    echo ""
    echo -e "${YELLOW}══════════════════════════════════════════════════════${NC}"
    echo -e "${YELLOW}CLOUDFLARE TUNNEL SETUP (Optional but Recommended)${NC}"
    echo -e "${YELLOW}══════════════════════════════════════════════════════${NC}"
    echo ""
    echo "Cloudflare Tunnel provides:"
    echo "  • Secure external access without port forwarding"
    echo "  • Automatic SSL/TLS encryption"
    echo "  • DDoS protection"
    echo "  • Free for personal use"
    echo ""
    echo "To get your token:"
    echo "  1. Visit: https://one.dash.cloudflare.com/"
    echo "  2. Go to: Zero Trust > Networks > Tunnels"
    echo "  3. Click 'Create a tunnel'"
    echo "  4. Name it: pcg-cc-mcp"
    echo "  5. Copy the token"
    echo "  6. Configure public hostname:"
    echo "     - Service: http://nginx:80"
    echo ""

    read -p "Enter Cloudflare Tunnel token (or press Enter to skip): " CF_TOKEN
    if [ -n "$CF_TOKEN" ]; then
        sed -i "s|CLOUDFLARE_TUNNEL_TOKEN=.*|CLOUDFLARE_TUNNEL_TOKEN=$CF_TOKEN|" .env
        print_success "Cloudflare Tunnel configured!"
    else
        print_warning "Skipped - app will only be accessible locally"
    fi

    echo ""
    echo -e "${YELLOW}══════════════════════════════════════════════════════${NC}"
    echo -e "${YELLOW}OPENAI API KEY (Required for NORA AI)${NC}"
    echo -e "${YELLOW}══════════════════════════════════════════════════════${NC}"
    echo ""
    echo "NORA is your AI Assistant that requires OpenAI API access."
    echo ""
    echo "To get your API key:"
    echo "  1. Visit: https://platform.openai.com/api-keys"
    echo "  2. Click 'Create new secret key'"
    echo "  3. Copy the key (starts with sk-)"
    echo ""

    read -p "Enter OpenAI API key: " OPENAI_KEY
    if [ -n "$OPENAI_KEY" ]; then
        sed -i "s|OPENAI_API_KEY=.*|OPENAI_API_KEY=$OPENAI_KEY|" .env
        print_success "OpenAI API key configured!"
    else
        print_warning "Skipped - NORA AI will not work without this"
    fi

    echo ""
    print_success "Environment configuration complete!"
fi

# Deployment
print_step "STEP 3: Building and Deploying"

echo ""
print_info "This will take 10-15 minutes on first run..."
print_info "Docker will download images and build the application"
echo ""

read -p "Ready to deploy? [Y/n]: " DEPLOY_CONFIRM
if [[ $DEPLOY_CONFIRM =~ ^[Nn]$ ]]; then
    print_info "Deployment cancelled"
    echo ""
    print_info "To deploy later, run: ./deploy.sh deploy"
    exit 0
fi

echo ""
print_info "Building Docker images..."
if docker-compose build; then
    print_success "Build complete!"
else
    print_error "Build failed!"
    print_info "Check logs above for details"
    exit 1
fi

echo ""
print_info "Starting services..."
if docker-compose up -d; then
    print_success "Services started!"
else
    print_error "Failed to start services!"
    exit 1
fi

# Wait for health checks
print_step "STEP 4: Waiting for Services"

echo ""
print_info "Waiting for services to become healthy..."
echo ""

sleep 5

MAX_WAIT=120
WAITED=0
HEALTHY=false

while [ $WAITED -lt $MAX_WAIT ]; do
    if docker-compose ps | grep -q "unhealthy"; then
        echo -ne "\r⏳ Services starting... ${WAITED}/${MAX_WAIT}s"
        sleep 5
        WAITED=$((WAITED + 5))
    else
        HEALTHY=true
        break
    fi
done

echo ""

if [ "$HEALTHY" = true ]; then
    print_success "All services are healthy!"
else
    print_warning "Services may still be starting..."
    print_info "Check status with: ./deploy.sh status"
fi

# Final status
print_step "🎉 DEPLOYMENT COMPLETE!"

echo ""
echo -e "${GREEN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo -e "${GREEN}Your PCG-CC-MCP Dashboard is now running!${NC}"
echo -e "${GREEN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo ""

# Load env to check tunnel config
source .env 2>/dev/null || true

echo -e "${CYAN}📍 Access Points:${NC}"
echo ""
echo "  🌐 Web Interface:"
echo "     http://localhost:8080"
echo ""
echo "  🔌 API Endpoints:"
echo "     • Main API:      http://localhost:3001"
echo "     • Ollama LLM:    http://localhost:11434"
echo "     • Chatterbox TTS: http://localhost:8100"
echo ""

if [ -n "$CLOUDFLARE_TUNNEL_TOKEN" ] && [ "$CLOUDFLARE_TUNNEL_TOKEN" != "your_cloudflare_tunnel_token_here" ]; then
    echo "  ✨ External Access:"
    echo "     Your app is accessible via Cloudflare Tunnel!"
    echo "     Check your dashboard: https://one.dash.cloudflare.com/"
    echo ""
fi

echo -e "${CYAN}🛠️  Management Commands:${NC}"
echo ""
echo "  ./deploy.sh status   - View service status"
echo "  ./deploy.sh logs     - View logs"
echo "  ./deploy.sh stop     - Stop services"
echo "  ./deploy.sh restart  - Restart services"
echo "  make help            - Show all commands"
echo ""

echo -e "${CYAN}📊 Current Status:${NC}"
echo ""
docker-compose ps

echo ""
echo -e "${GREEN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo -e "${GREEN}Enjoy your dashboard! 🚀${NC}"
echo -e "${GREEN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo ""

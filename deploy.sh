#!/bin/bash
set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color

# Helper functions
print_header() {
    echo ""
    echo -e "${CYAN}========================================${NC}"
    echo -e "${CYAN}$1${NC}"
    echo -e "${CYAN}========================================${NC}"
    echo ""
}

print_success() {
    echo -e "${GREEN}✅ $1${NC}"
}

print_error() {
    echo -e "${RED}❌ $1${NC}"
}

print_warning() {
    echo -e "${YELLOW}⚠️  $1${NC}"
}

print_info() {
    echo -e "${BLUE}ℹ️  $1${NC}"
}

# Check prerequisites
check_prerequisites() {
    print_header "Checking Prerequisites"

    local missing=0

    if ! command -v docker &> /dev/null; then
        print_error "Docker is not installed"
        print_info "Install from: https://docs.docker.com/get-docker/"
        missing=1
    else
        print_success "Docker is installed ($(docker --version))"
    fi

    if ! command -v docker-compose &> /dev/null && ! docker compose version &> /dev/null; then
        print_error "Docker Compose is not installed"
        print_info "Install from: https://docs.docker.com/compose/install/"
        missing=1
    else
        print_success "Docker Compose is installed"
    fi

    # Check if Docker daemon is running
    if ! docker ps &> /dev/null; then
        print_error "Docker daemon is not running"
        print_info "Start Docker and try again"
        missing=1
    else
        print_success "Docker daemon is running"
    fi

    # Check for NVIDIA GPU support (optional)
    if command -v nvidia-smi &> /dev/null; then
        print_success "NVIDIA GPU detected"
        if ! docker run --rm --gpus all nvidia/cuda:12.1.0-base-ubuntu22.04 nvidia-smi &> /dev/null; then
            print_warning "NVIDIA Docker runtime not properly configured"
            print_info "GPU acceleration will not be available"
            print_info "Install nvidia-container-toolkit: https://docs.nvidia.com/datacenter/cloud-native/container-toolkit/install-guide.html"
        else
            print_success "NVIDIA Docker runtime is configured"
        fi
    else
        print_info "No NVIDIA GPU detected (optional - Ollama will run in CPU mode)"
    fi

    if [ $missing -eq 1 ]; then
        print_error "Please install missing prerequisites and try again"
        exit 1
    fi

    echo ""
}

# Setup environment file
setup_env() {
    if [ -f .env ]; then
        print_info "Found existing .env file"
        read -p "Do you want to reconfigure? [y/N]: " reconfigure
        if [[ ! $reconfigure =~ ^[Yy]$ ]]; then
            return
        fi
    fi

    print_header "Environment Configuration"

    # Create .env from example if it doesn't exist
    if [ ! -f .env ]; then
        cp .env.example .env
        print_success "Created .env from .env.example"
    fi

    print_info "Configure your Cloudflare Tunnel..."
    echo ""
    echo "To get your Cloudflare Tunnel token:"
    echo "1. Go to: https://one.dash.cloudflare.com/"
    echo "2. Navigate to: Zero Trust > Networks > Tunnels"
    echo "3. Click 'Create a tunnel'"
    echo "4. Choose 'Cloudflared' connector"
    echo "5. Name your tunnel (e.g., 'pcg-cc-mcp')"
    echo "6. Copy the token"
    echo "7. Configure a public hostname:"
    echo "   - Subdomain: your-app"
    echo "   - Domain: your-domain.com"
    echo "   - Service: http://nginx:80"
    echo ""

    read -p "Enter your Cloudflare Tunnel token (or press Enter to skip): " cf_token
    if [ -n "$cf_token" ]; then
        # Update .env file
        if grep -q "^CLOUDFLARE_TUNNEL_TOKEN=" .env; then
            sed -i "s|^CLOUDFLARE_TUNNEL_TOKEN=.*|CLOUDFLARE_TUNNEL_TOKEN=$cf_token|" .env
        else
            echo "CLOUDFLARE_TUNNEL_TOKEN=$cf_token" >> .env
        fi
        print_success "Cloudflare Tunnel token configured"
    else
        print_warning "Cloudflare Tunnel not configured - application will only be accessible locally"
    fi

    echo ""
    read -p "Enter your OpenAI API key (required for NORA AI): " openai_key
    if [ -n "$openai_key" ]; then
        if grep -q "^OPENAI_API_KEY=" .env; then
            sed -i "s|^OPENAI_API_KEY=.*|OPENAI_API_KEY=$openai_key|" .env
        else
            echo "OPENAI_API_KEY=$openai_key" >> .env
        fi
        print_success "OpenAI API key configured"
    else
        print_warning "OpenAI API key not set - NORA AI Assistant will not work"
    fi

    echo ""
    print_success "Environment configuration complete"
}

# Validate environment
validate_env() {
    print_header "Validating Configuration"

    if [ ! -f .env ]; then
        print_error "No .env file found"
        print_info "Run setup first or create .env from .env.example"
        exit 1
    fi

    source .env

    local warnings=0

    # Check Cloudflare token
    if [ -z "$CLOUDFLARE_TUNNEL_TOKEN" ] || [ "$CLOUDFLARE_TUNNEL_TOKEN" = "your_cloudflare_tunnel_token_here" ]; then
        print_warning "Cloudflare Tunnel token not configured"
        print_info "App will only be accessible locally on port 8080"
        warnings=$((warnings + 1))
    else
        print_success "Cloudflare Tunnel configured"
    fi

    # Check OpenAI key
    if [ -z "$OPENAI_API_KEY" ] || [ "$OPENAI_API_KEY" = "sk-your-openai-api-key-here" ]; then
        print_warning "OpenAI API key not configured"
        print_info "NORA AI Assistant will not work"
        warnings=$((warnings + 1))
    else
        print_success "OpenAI API key configured"
    fi

    if [ $warnings -gt 0 ]; then
        echo ""
        print_warning "Configuration has $warnings warning(s)"
        read -p "Continue anyway? [y/N]: " continue_anyway
        if [[ ! $continue_anyway =~ ^[Yy]$ ]]; then
            print_info "Run './deploy.sh setup' to configure"
            exit 1
        fi
    fi

    echo ""
}

# Build and start
build_and_start() {
    print_header "Building and Starting Services"

    validate_env

    print_info "Building Docker images (this may take 10-15 minutes on first run)..."
    docker-compose build

    print_success "Build complete"
    echo ""

    print_info "Starting services..."
    docker-compose up -d

    print_success "Services started"
    echo ""

    print_info "Waiting for services to be healthy..."
    sleep 5

    # Wait for health checks
    local max_wait=120
    local waited=0
    while [ $waited -lt $max_wait ]; do
        if docker-compose ps | grep -q "unhealthy"; then
            print_info "Services still starting... ($waited/$max_wait seconds)"
            sleep 5
            waited=$((waited + 5))
        else
            break
        fi
    done

    if [ $waited -ge $max_wait ]; then
        print_warning "Services may not be fully healthy yet"
        print_info "Check status with: docker-compose ps"
        print_info "Check logs with: docker-compose logs -f"
    else
        print_success "All services are healthy!"
    fi

    show_access_info
}

# Start existing containers
start_services() {
    print_header "Starting Services"

    print_info "Starting containers..."
    docker-compose up -d

    print_success "Services started"
    show_access_info
}

# Stop services
stop_services() {
    print_header "Stopping Services"

    print_info "Stopping containers..."
    docker-compose down

    print_success "Services stopped"
}

# View logs
view_logs() {
    print_header "Viewing Logs"

    echo "Available services:"
    echo "  1) app        - Main application"
    echo "  2) nginx      - Reverse proxy"
    echo "  3) apn-bridge - Alpha Protocol Network bridge"
    echo "  4) cloudflared - Cloudflare Tunnel"
    echo "  5) db-backup  - Database backup service"
    echo "  6) all        - All services"
    echo ""

    read -p "Select service [1-6]: " service_choice

    case $service_choice in
        1) docker-compose logs -f app ;;
        2) docker-compose logs -f nginx ;;
        3) docker-compose logs -f apn-bridge ;;
        4) docker-compose logs -f cloudflared ;;
        5) docker-compose logs -f db-backup ;;
        6) docker-compose logs -f ;;
        *) print_error "Invalid choice"; exit 1 ;;
    esac
}

# Show status
show_status() {
    print_header "Service Status"

    docker-compose ps

    echo ""
    print_header "Resource Usage"
    docker stats --no-stream --format "table {{.Container}}\t{{.CPUPerc}}\t{{.MemUsage}}\t{{.NetIO}}" \
        $(docker-compose ps -q 2>/dev/null) 2>/dev/null || print_info "No running containers"

    echo ""
    show_access_info
}

# Show access information
show_access_info() {
    print_header "Access Information"

    source .env 2>/dev/null || true

    print_info "Local Access:"
    echo "  🌐 Web Interface: http://localhost:8080"
    echo "  🤖 Ollama API: http://localhost:11434"
    echo "  🎤 Chatterbox TTS: http://localhost:8100"
    echo "  🔌 Main API: http://localhost:3001"

    if [ -n "$CLOUDFLARE_TUNNEL_TOKEN" ] && [ "$CLOUDFLARE_TUNNEL_TOKEN" != "your_cloudflare_tunnel_token_here" ]; then
        echo ""
        print_info "External Access:"
        echo "  ✨ Your app is accessible via your Cloudflare Tunnel URL"
        echo "  📋 Check your Cloudflare dashboard for the public URL"
        echo "  🔗 https://one.dash.cloudflare.com/ > Zero Trust > Networks > Tunnels"
    fi

    echo ""
}

# Clean rebuild
clean_rebuild() {
    print_header "Clean Rebuild"

    print_warning "This will stop all containers and rebuild from scratch"
    read -p "Continue? [y/N]: " confirm
    if [[ ! $confirm =~ ^[Yy]$ ]]; then
        print_info "Cancelled"
        exit 0
    fi

    print_info "Stopping containers..."
    docker-compose down

    print_info "Removing images..."
    docker-compose rm -f

    print_info "Rebuilding..."
    docker-compose build --no-cache

    print_info "Starting services..."
    docker-compose up -d

    print_success "Clean rebuild complete!"
    show_access_info
}

# Backup database
backup_db() {
    print_header "Database Backup"

    TIMESTAMP=$(date +%Y%m%d_%H%M%S)
    BACKUP_FILE="./backups/manual_backup_${TIMESTAMP}.sqlite"

    print_info "Creating manual backup..."

    if [ ! -d "./backups" ]; then
        mkdir -p ./backups
    fi

    if [ -f "./dev_assets/db.sqlite" ]; then
        cp ./dev_assets/db.sqlite "$BACKUP_FILE"
        print_success "Backup created: $BACKUP_FILE"

        DB_SIZE=$(du -h "$BACKUP_FILE" | cut -f1)
        echo "  📊 Size: $DB_SIZE"
    else
        print_error "Database file not found at ./dev_assets/db.sqlite"
        exit 1
    fi
}

# Update application
update_app() {
    print_header "Update Application"

    print_info "Pulling latest code..."
    if [ -d .git ]; then
        git pull
        print_success "Code updated"
    else
        print_warning "Not a git repository - skipping code update"
    fi

    echo ""
    print_info "Rebuilding and restarting services..."
    docker-compose down
    docker-compose build
    docker-compose up -d

    print_success "Update complete!"
    show_access_info
}

# Main menu
show_menu() {
    print_header "🚀 PCG-CC-MCP Docker Deployment"

    echo "Choose an option:"
    echo ""
    echo "  Setup & Deployment:"
    echo "    1)  Setup - Configure environment"
    echo "    2)  Deploy - Build and start (fresh deployment)"
    echo "    3)  Start - Start existing containers"
    echo "    4)  Stop - Stop all containers"
    echo "    5)  Restart - Restart all containers"
    echo ""
    echo "  Maintenance:"
    echo "    6)  Status - Show service status"
    echo "    7)  Logs - View service logs"
    echo "    8)  Backup - Create manual database backup"
    echo "    9)  Update - Update and rebuild"
    echo "    10) Clean Rebuild - Complete rebuild from scratch"
    echo ""
    echo "  Utilities:"
    echo "    11) Shell - Open shell in app container"
    echo "    12) Check - Run system checks"
    echo "    0)  Exit"
    echo ""
    read -p "Enter choice [0-12]: " choice
    echo ""

    case $choice in
        1) setup_env ;;
        2) build_and_start ;;
        3) start_services ;;
        4) stop_services ;;
        5) docker-compose restart; print_success "Services restarted"; show_access_info ;;
        6) show_status ;;
        7) view_logs ;;
        8) backup_db ;;
        9) update_app ;;
        10) clean_rebuild ;;
        11) docker-compose exec app /bin/bash ;;
        12) check_prerequisites; validate_env ;;
        0) print_info "Goodbye!"; exit 0 ;;
        *) print_error "Invalid choice"; exit 1 ;;
    esac
}

# Main script
main() {
    # Check if running in script directory
    if [ ! -f "docker-compose.yml" ]; then
        print_error "Please run this script from the pcg-cc-mcp directory"
        print_info "cd to the directory containing docker-compose.yml"
        exit 1
    fi

    # If no arguments, show menu
    if [ $# -eq 0 ]; then
        show_menu
        exit 0
    fi

    # Handle command line arguments
    case "$1" in
        setup) setup_env ;;
        deploy) check_prerequisites; build_and_start ;;
        start) start_services ;;
        stop) stop_services ;;
        restart) docker-compose restart; print_success "Services restarted" ;;
        status) show_status ;;
        logs) shift; docker-compose logs -f "$@" ;;
        backup) backup_db ;;
        update) update_app ;;
        clean) clean_rebuild ;;
        shell) docker-compose exec app /bin/bash ;;
        check) check_prerequisites; validate_env ;;
        *)
            echo "Usage: $0 {setup|deploy|start|stop|restart|status|logs|backup|update|clean|shell|check}"
            echo ""
            echo "Or run without arguments for interactive menu"
            exit 1
            ;;
    esac
}

main "$@"

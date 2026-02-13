#!/bin/bash
# Sovereign Stack Startup Script
# Starts all auxiliary services in the correct order before the main dashboard

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"

cd "$PROJECT_DIR"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color

log_info() { echo -e "${CYAN}[INFO]${NC} $1"; }
log_success() { echo -e "${GREEN}[OK]${NC} $1"; }
log_warn() { echo -e "${YELLOW}[WARN]${NC} $1"; }
log_error() { echo -e "${RED}[ERROR]${NC} $1"; }

# Cleanup function
cleanup() {
    log_info "Shutting down Sovereign Stack services..."

    # Kill background processes
    [ -n "$WHISPER_PID" ] && kill $WHISPER_PID 2>/dev/null && log_info "Stopped Whisper STT"
    [ -n "$CHATTERBOX_PID" ] && kill $CHATTERBOX_PID 2>/dev/null && log_info "Stopped Chatterbox TTS"

    # Kill any remaining python voice servers
    pkill -f "whisper_server.py" 2>/dev/null || true
    pkill -f "chatterbox_server.py" 2>/dev/null || true

    log_info "Cleanup complete"
    exit 0
}

trap cleanup SIGINT SIGTERM

# Check for required tools
check_dependencies() {
    log_info "Checking dependencies..."

    command -v python3 >/dev/null 2>&1 || { log_error "python3 is required"; exit 1; }
    command -v pnpm >/dev/null 2>&1 || { log_error "pnpm is required"; exit 1; }
    command -v cargo >/dev/null 2>&1 || { log_error "cargo is required"; exit 1; }

    log_success "All dependencies found"
}

# Wait for a service to be ready
wait_for_service() {
    local url=$1
    local name=$2
    local max_attempts=${3:-30}
    local attempt=0

    log_info "Waiting for $name to be ready..."

    while [ $attempt -lt $max_attempts ]; do
        if curl -s "$url" >/dev/null 2>&1; then
            log_success "$name is ready"
            return 0
        fi
        attempt=$((attempt + 1))
        sleep 1
    done

    log_error "$name failed to start after ${max_attempts}s"
    return 1
}

# Start Ollama if not running
start_ollama() {
    if curl -s http://localhost:11434/api/tags >/dev/null 2>&1; then
        log_success "Ollama already running"
    else
        log_info "Starting Ollama..."
        ollama serve &
        wait_for_service "http://localhost:11434/api/tags" "Ollama" 30
    fi

    # Ensure required model is available
    if ! ollama list | grep -q "llama3.2"; then
        log_info "Pulling llama3.2 model..."
        ollama pull llama3.2:latest
    fi
}

# Start Whisper STT server
start_whisper() {
    if curl -s http://localhost:8101/health >/dev/null 2>&1; then
        log_success "Whisper STT already running"
        return 0
    fi

    log_info "Starting Whisper STT server (tiny model for fast responses)..."
    WHISPER_MODEL=tiny python3 "$SCRIPT_DIR/whisper_server.py" > /tmp/whisper_server.log 2>&1 &
    WHISPER_PID=$!

    wait_for_service "http://localhost:8101/health" "Whisper STT" 60
}

# Start Chatterbox TTS server
start_chatterbox() {
    if curl -s http://localhost:8100/health >/dev/null 2>&1; then
        log_success "Chatterbox TTS already running"
        return 0
    fi

    log_info "Starting Chatterbox TTS server..."
    python3 "$SCRIPT_DIR/chatterbox_server.py" > /tmp/chatterbox_server.log 2>&1 &
    CHATTERBOX_PID=$!

    wait_for_service "http://localhost:8100/health" "Chatterbox TTS" 60
}

# Export environment variables
setup_environment() {
    log_info "Setting up environment..."

    # Load .env file
    if [ -f "$PROJECT_DIR/.env" ]; then
        set -a
        source "$PROJECT_DIR/.env"
        set +a
        log_success "Loaded .env file"
    fi

    # Ensure voice service URLs are set
    export WHISPER_URL="${WHISPER_URL:-http://localhost:8101}"
    export CHATTERBOX_URL="${CHATTERBOX_URL:-http://localhost:8100}"

    log_info "WHISPER_URL=$WHISPER_URL"
    log_info "CHATTERBOX_URL=$CHATTERBOX_URL"
}

# Print status summary
print_status() {
    echo ""
    echo -e "${CYAN}═══════════════════════════════════════════════════════════${NC}"
    echo -e "${CYAN}             SOVEREIGN STACK - SERVICE STATUS              ${NC}"
    echo -e "${CYAN}═══════════════════════════════════════════════════════════${NC}"
    echo ""

    # Check each service
    if curl -s http://localhost:11434/api/tags >/dev/null 2>&1; then
        echo -e "  Ollama LLM       ${GREEN}● Running${NC}  (port 11434)"
    else
        echo -e "  Ollama LLM       ${RED}○ Stopped${NC}"
    fi

    if curl -s http://localhost:8101/health >/dev/null 2>&1; then
        local whisper_info=$(curl -s http://localhost:8101/health | jq -r '.model // "unknown"')
        echo -e "  Whisper STT      ${GREEN}● Running${NC}  (port 8101, model: $whisper_info)"
    else
        echo -e "  Whisper STT      ${RED}○ Stopped${NC}"
    fi

    if curl -s http://localhost:8100/health >/dev/null 2>&1; then
        local tts_device=$(curl -s http://localhost:8100/health | jq -r '.device // "unknown"')
        echo -e "  Chatterbox TTS   ${GREEN}● Running${NC}  (port 8100, device: $tts_device)"
    else
        echo -e "  Chatterbox TTS   ${RED}○ Stopped${NC}"
    fi

    echo ""
    echo -e "${CYAN}═══════════════════════════════════════════════════════════${NC}"
    echo ""
}

# Main startup sequence
main() {
    echo ""
    echo -e "${CYAN}╔═══════════════════════════════════════════════════════════╗${NC}"
    echo -e "${CYAN}║           SOVEREIGN STACK - STARTUP SEQUENCE              ║${NC}"
    echo -e "${CYAN}║                  PCG Dashboard v0.0.96                    ║${NC}"
    echo -e "${CYAN}╚═══════════════════════════════════════════════════════════╝${NC}"
    echo ""

    check_dependencies
    setup_environment

    # Start auxiliary services first
    log_info "Starting auxiliary services..."
    start_ollama
    start_whisper
    start_chatterbox

    print_status

    # Start the main dashboard
    log_info "Starting PCG Dashboard..."
    echo ""

    # Run pnpm dev in foreground so Ctrl+C works
    exec pnpm run dev
}

main "$@"

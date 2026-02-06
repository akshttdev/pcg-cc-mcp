#!/bin/bash
# Start All Dashboard Agent Services

echo "╔══════════════════════════════════════════════════════════╗"
echo "║  Starting All Dashboard Services                        ║"
echo "╚══════════════════════════════════════════════════════════╝"
echo ""

# Get script directory
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# 1. Check/Start Ollama (LLM Service)
echo "→ Checking Ollama (LLM Service)..."
if curl -s http://localhost:11434/api/tags >/dev/null 2>&1; then
    echo "  ✅ Ollama already running on port 11434"
else
    echo "  → Starting Ollama..."
    ollama serve > /tmp/ollama.log 2>&1 &
    echo $! > /tmp/ollama.pid
    sleep 3
    echo "  ✅ Ollama started"
fi

# 2. Check/Start ComfyUI (Image Generation)
echo "→ Checking ComfyUI (Image Generation)..."
if lsof -i :8188 >/dev/null 2>&1; then
    echo "  ✅ ComfyUI already running on port 8188"
else
    if [ -d "/home/pythia/ComfyUI" ]; then
        echo "  → Starting ComfyUI..."
        cd /home/pythia/ComfyUI
        python3 main.py --listen 0.0.0.0 --port 8188 > /tmp/comfyui.log 2>&1 &
        echo $! > /tmp/comfyui.pid
        sleep 5
        echo "  ✅ ComfyUI started"
    else
        echo "  ⚠️  ComfyUI not installed (image generation disabled)"
    fi
fi

# 3. Check/Start Chatterbox TTS (Voice Service)
echo "→ Checking Chatterbox TTS (Voice Service)..."
if curl -s http://localhost:8100/health >/dev/null 2>&1; then
    echo "  ✅ Chatterbox already running on port 8100"
else
    echo "  → Chatterbox running in Docker (should be auto-started)"
fi

# 4. Check/Start Redis
echo "→ Checking Redis..."
if redis-cli ping >/dev/null 2>&1; then
    echo "  ✅ Redis running"
else
    echo "  ⚠️  Redis not running (may need sudo to start)"
fi

# 5. Check/Start PostgreSQL
echo "→ Checking PostgreSQL..."
if pg_isready -q 2>/dev/null; then
    echo "  ✅ PostgreSQL running"
else
    echo "  ⚠️  PostgreSQL not running (may need sudo to start)"
fi

# 6. Start APN Node
echo "→ Checking APN Node..."
if lsof -i :4001 >/dev/null 2>&1; then
    echo "  ✅ APN Node already running on port 4001"
else
    echo "  → Starting APN Node..."
    cd "$SCRIPT_DIR"
    nohup ./target/release/apn_node \
        --port 4001 \
        --relay nats://nonlocal.info:4222 \
        --heartbeat-interval 30 \
        --import "slush index float shaft flight citizen swear chunk correct veteran eyebrow blind" \
        > /tmp/apn_node.log 2>&1 &
    echo $! > /tmp/apn_master.pid
    sleep 3
    echo "  ✅ APN Node started"
fi

# 7. Start APN Bridge Server
echo "→ Checking APN Bridge Server..."
if lsof -i :8000 >/dev/null 2>&1; then
    echo "  ✅ APN Bridge already running on port 8000"
else
    echo "  → Starting APN Bridge..."
    cd "$SCRIPT_DIR"
    python3 apn_bridge_server.py > /tmp/apn_bridge.log 2>&1 &
    echo $! > /tmp/apn_bridge.pid
    sleep 2
    echo "  ✅ APN Bridge started"
fi

# 8. Start PCG Backend Server
echo "→ Checking PCG Backend Server..."
if lsof -i :58297 >/dev/null 2>&1; then
    echo "  ✅ PCG Backend already running on port 58297"
else
    echo "  → Starting PCG Backend..."
    cd "$SCRIPT_DIR"
    BACKEND_PORT=58297 nohup ./target/release/server > /tmp/pcg_backend.log 2>&1 &
    echo $! > /tmp/pcg_backend.pid
    sleep 4
    echo "  ✅ PCG Backend started"
fi

echo ""
echo "╔══════════════════════════════════════════════════════════╗"
echo "║  ✅ All Services Started!                               ║"
echo "╠══════════════════════════════════════════════════════════╣"
echo "║  AI Services:                                           ║"
echo "║    • Ollama (LLM):        http://localhost:11434        ║"
echo "║    • ComfyUI (Images):    http://localhost:8188         ║"
echo "║    • Chatterbox (Voice):  http://localhost:8100         ║"
echo "║                                                          ║"
echo "║  Network Services:                                      ║"
echo "║    • APN Node:            Port 4001                     ║"
echo "║    • APN Bridge:          http://localhost:8000         ║"
echo "║    • PCG Backend:         http://localhost:58297        ║"
echo "║                                                          ║"
echo "║  Dashboard:                                             ║"
echo "║    • http://dashboard.powerclubglobal.com               ║"
echo "║                                                          ║"
echo "║  Available Agents:                                      ║"
echo "║    • Nora (Executive Assistant)                         ║"
echo "║    • Maci (Social Media Manager)                        ║"
echo "║    • Editron (Video Editor)                             ║"
echo "║    • Genesis (Brand Architect)                          ║"
echo "║    • Astra (Research Analyst)                           ║"
echo "║    • Scout (Social Intelligence)                        ║"
echo "║    • Auri (Developer Architect)                         ║"
echo "╚══════════════════════════════════════════════════════════╝"

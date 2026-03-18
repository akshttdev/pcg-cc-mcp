#!/bin/bash
# Test all agent services

echo "╔══════════════════════════════════════════════════════════╗"
echo "║  Testing All Agent Services                             ║"
echo "╚══════════════════════════════════════════════════════════╝"
echo ""

# Test Ollama
echo -n "Ollama (LLM):           "
curl -s http://localhost:11434/api/tags >/dev/null 2>&1 && echo "✅ Running" || echo "❌ Not running"

# Test ComfyUI
echo -n "ComfyUI (Images):       "
curl -s http://localhost:8188/ >/dev/null 2>&1 && echo "✅ Running" || echo "❌ Not running"

# Test Chatterbox
echo -n "Chatterbox (Voice):     "
curl -s http://localhost:8100/health >/dev/null 2>&1 && echo "✅ Running" || echo "❌ Not running"

# Test APN Node
echo -n "APN Node:               "
lsof -i :4001 >/dev/null 2>&1 && echo "✅ Running" || echo "❌ Not running"

# Test APN Bridge
echo -n "APN Bridge:             "
lsof -i :8000 >/dev/null 2>&1 && echo "✅ Running" || echo "❌ Not running"

# Test PCG Backend
echo -n "PCG Backend:            "
curl -s http://localhost:58297/api/health >/dev/null 2>&1 && echo "✅ Running" || echo "❌ Not running"

# Test Redis
echo -n "Redis:                  "
redis-cli ping >/dev/null 2>&1 && echo "✅ Running" || echo "❌ Not running"

# Test PostgreSQL  
echo -n "PostgreSQL:             "
pg_isready -q 2>/dev/null && echo "✅ Running" || echo "❌ Not running"

echo ""
echo "Dashboard: http://dashboard.powerclubglobal.com"

#!/bin/bash
# Start PCG Backend with Local Whisper STT

cd /home/pythia/pcg-cc-mcp

# Export Whisper configuration
export WHISPER_URL=http://localhost:8101
export WHISPER_MODEL=base
export WHISPER_DEVICE=cuda

# Source other env vars from .env
set -a
source .env
set +a

# Unset OpenAI key to force local Whisper
unset OPENAI_API_KEY

# Override port for production (nginx expects 3002)
export PORT=3002
export BACKEND_PORT=3002

echo "Starting PCG Backend with Local Whisper STT..."
echo "WHISPER_URL: $WHISPER_URL"
echo "PORT: $PORT"

exec ./target/release/server

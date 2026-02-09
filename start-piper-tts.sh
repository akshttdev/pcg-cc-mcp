#!/bin/bash
# Start Piper TTS Server for Nora Voice System
# Uses the semaine British female voice (selected by user)

cd /home/pythia/pcg-cc-mcp

# Load conda
source /home/pythia/miniconda3/etc/profile.d/conda.sh
conda activate chatterbox

# Set configuration
export PIPER_PORT=8102
export PIPER_MODEL=/home/pythia/pcg-cc-mcp/piper_voices/en_GB-semaine-medium.onnx

echo "Starting Piper TTS Server..."
echo "Model: en_GB-semaine-medium (British female - semaine)"
echo "Port: $PIPER_PORT"
echo "Speakers: prudence (default), spike, obadiah, poppy"

# Start the server
exec python scripts/piper_server.py

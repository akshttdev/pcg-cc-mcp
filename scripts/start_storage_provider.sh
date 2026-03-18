#!/bin/bash
# Start Pythia Storage Provider Server
# This runs on Pythia Master Node to earn VIBE for storage services

echo "======================================================================"
echo "Starting Pythia Storage Provider Server"
echo "======================================================================"

cd /home/pythia/pcg-cc-mcp

# Get Pythia device ID
DEVICE_ID="d903c0e7-f3a6-4967-80e7-0da9d0fe7632"

echo "Device ID: $DEVICE_ID"
echo "Listening for storage requests..."
echo ""

# Start server
python3 sovereign_storage/storage_provider_server.py "$DEVICE_ID"

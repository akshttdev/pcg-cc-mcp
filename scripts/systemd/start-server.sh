#!/bin/bash
cd /home/pythia/pcg-cc-mcp
export PATH="/home/pythia/.nvm/versions/node/v22.20.0/bin:/home/pythia/.local/bin:/home/pythia/miniconda3/bin:/usr/local/bin:/usr/bin:/bin"
set -a
source /home/pythia/pcg-cc-mcp/.env
set +a
exec ./target/debug/server

#!/bin/bash
cd /home/pythia/pcg-cc-mcp
set -a
source /home/pythia/pcg-cc-mcp/.env
set +a
exec /home/pythia/.nvm/versions/node/v22.20.0/bin/node discord-bot-js/src/index.js

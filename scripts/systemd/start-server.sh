#!/bin/bash
cd /home/pythia/pcg-cc-mcp
set -a
source /home/pythia/pcg-cc-mcp/.env
set +a
exec ./target/debug/server

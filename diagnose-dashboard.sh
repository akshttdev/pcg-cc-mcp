#!/bin/bash
# PCG Dashboard (pcg-cc-mcp) Diagnostic Script
# Run this on devices that should be running the Dashboard

echo "======================================"
echo "PCG DASHBOARD CONNECTION DIAGNOSTIC"
echo "======================================"
echo ""

# 1. Check if in pcg-cc-mcp directory
echo "1. Current directory:"
pwd
echo ""

# 2. Check if pcg-cc-mcp exists
echo "2. PCG Dashboard repository status:"
if [ -d ".git" ]; then
    echo "✓ Git repository found"
    git remote -v | head -2
    git branch --show-current
else
    echo "✗ Not in PCG Dashboard directory!"
fi
echo ""

# 3. Check Rust installation
echo "3. Rust toolchain:"
rustc --version 2>&1 || echo "✗ Rust NOT installed"
cargo --version 2>&1 || echo "✗ Cargo NOT installed"
echo ""

# 4. Check if Dashboard is built
echo "4. Dashboard build status:"
if [ -f "target/release/pcg-cc-mcp" ] || [ -f "target/debug/pcg-cc-mcp" ]; then
    echo "✓ Dashboard binary exists"
    ls -lh target/*/pcg-cc-mcp 2>/dev/null
else
    echo "✗ Dashboard NOT built"
    echo "  Run: cargo build --release"
fi
echo ""

# 5. Check if Dashboard server is running
echo "5. Dashboard server status:"
if lsof -i :8081 >/dev/null 2>&1; then
    echo "✓ Dashboard server RUNNING on port 8081"
    lsof -i :8081 | grep LISTEN
else
    echo "✗ Dashboard server NOT running on port 8081"
fi
echo ""

# 6. Check Dashboard API health
echo "6. Dashboard API health check:"
curl -s http://localhost:8081/ 2>&1 | head -10 || echo "✗ Cannot connect to Dashboard API"
echo ""

# 7. Check NATS server status
echo "7. NATS server status:"
if lsof -i :4222 >/dev/null 2>&1; then
    echo "✓ NATS server RUNNING on port 4222"
    lsof -i :4222 | grep LISTEN
else
    echo "✗ NATS server NOT running on port 4222"
    echo "  Dashboard should start its own NATS server"
fi
echo ""

# 8. Check database
echo "8. Database status:"
if [ -f "dev_assets/db.sqlite" ]; then
    echo "✓ Database exists"
    echo "Size: $(du -h dev_assets/db.sqlite | cut -f1)"
    echo "Peer nodes: $(sqlite3 dev_assets/db.sqlite "SELECT COUNT(*) FROM peer_nodes;" 2>/dev/null || echo 'Cannot query')"
else
    echo "✗ Database NOT found at dev_assets/db.sqlite"
fi
echo ""

# 9. Check config files
echo "9. Configuration:"
if [ -f ".env" ]; then
    echo "✓ .env file exists"
    grep -E "NATS_URL|DATABASE_URL|PORT" .env | sed 's/=.*/=***/' || echo "No relevant config found"
else
    echo "✗ No .env file found"
fi
echo ""

# 10. Check recent logs
echo "10. Recent Dashboard logs:"
if [ -f "/tmp/apn_node.log" ]; then
    echo "Last 15 lines from /tmp/apn_node.log:"
    tail -15 /tmp/apn_node.log
else
    echo "✗ No log file at /tmp/apn_node.log"
fi
echo ""

# 11. Check Dashboard processes
echo "11. Dashboard-related processes:"
ps aux | grep -E "pcg-cc-mcp|target.*release.*pcg|cargo run" | grep -v grep || echo "No Dashboard processes found"
echo ""

# 12. Test network connectivity
echo "12. Network connectivity:"
echo "Can reach main network relay:"
timeout 3 bash -c "echo 'PING' | nc nonlocal.info 4222" >/dev/null 2>&1
if [ $? -eq 0 ]; then
    echo "✓ Can connect to nonlocal.info:4222"
else
    echo "✗ CANNOT connect to nonlocal.info:4222"
fi
echo ""

# 13. System info
echo "13. System information:"
echo "Hostname: $(hostname)"
echo "CPU cores: $(nproc)"
echo "RAM: $(free -h | awk '/^Mem:/ {print $2}')"
echo "GPU: $(lspci | grep -i vga | head -1)"
if command -v nvidia-smi &> /dev/null; then
    echo "NVIDIA GPU: $(nvidia-smi --query-gpu=name --format=csv,noheader 2>/dev/null)"
fi
echo ""

# 14. Port checks
echo "14. Port availability:"
for port in 8081 4222; do
    if lsof -i :$port >/dev/null 2>&1; then
        echo "Port $port: IN USE"
    else
        echo "Port $port: AVAILABLE"
    fi
done
echo ""

echo "======================================"
echo "DIAGNOSTIC COMPLETE"
echo "======================================"
echo ""
echo "Quick fixes:"
echo "  - If Rust not installed: curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh"
echo "  - If not built: cargo build --release"
echo "  - If not running: cargo run --release"
echo "  - Check logs in /tmp/apn_*.log for errors"
echo ""
echo "To start Dashboard:"
echo "  cargo run --release"

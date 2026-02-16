#!/bin/bash
# Quick Health Check for ORCHA System
# Run anytime to verify system status

DB_PATH="$HOME/pcg-cc-mcp/dev_assets/db.sqlite"

echo "═══════════════════════════════════════════════════════════════"
echo "ORCHA System Health Check"
echo "═══════════════════════════════════════════════════════════════"
echo ""

# Check 1: Database exists
echo "📁 Database Status"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
if [ -f "$DB_PATH" ]; then
    DB_SIZE=$(du -h "$DB_PATH" | cut -f1)
    echo "✅ Database found: $DB_PATH ($DB_SIZE)"
else
    echo "❌ Database not found: $DB_PATH"
    exit 1
fi
echo ""

# Check 2: Server status
echo "🖥️  Server Status"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
if pgrep -f "target/release/server" > /dev/null; then
    PID=$(pgrep -f "target/release/server")
    UPTIME=$(ps -p "$PID" -o etime= | xargs)
    echo "✅ ORCHA server running (PID: $PID, Uptime: $UPTIME)"
else
    echo "❌ ORCHA server not running"
    echo "   Start with: cd ~/pcg-cc-mcp && RUST_LOG=info ./target/release/server &"
fi
echo ""

# Check 3: Users
echo "👥 Users"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
sqlite3 "$DB_PATH" <<'SQL'
.mode column
.headers on
SELECT
    username,
    CASE
        WHEN LENGTH(hex(id)) = 32 THEN '✅ Fixed'
        ELSE '❌ Corrupted'
    END as uuid_status,
    printf('%.2f', vibe_balance) as vibe_balance
FROM users
WHERE username IN ('admin', 'Bonomotion', 'Sirak')
ORDER BY vibe_balance DESC;
SQL
echo ""

# Check 4: Master Nodes
echo "🔧 Master Nodes"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
NODE_COUNT=$(sqlite3 "$DB_PATH" "SELECT COUNT(*) FROM devices WHERE device_tier = 'master_node';")

if [ "$NODE_COUNT" -gt 0 ]; then
    echo "✅ $NODE_COUNT Master Node(s) registered"
    sqlite3 "$DB_PATH" <<'SQL'
.mode column
.headers on
SELECT
    hostname,
    total_cores as cores,
    total_ram_gb as ram_gb,
    CASE WHEN is_online = 1 THEN '✅ Online' ELSE '⚠️ Offline' END as status
FROM devices
WHERE device_tier = 'master_node'
ORDER BY hostname;
SQL
else
    echo "⚠️  No Master Nodes registered"
fi
echo ""

# Check 5: Projects
echo "📂 Projects"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
sqlite3 "$DB_PATH" <<'SQL'
.mode column
.headers on
SELECT
    u.username,
    COUNT(p.id) as owned_projects,
    COUNT(pm.id) as shared_projects
FROM users u
LEFT JOIN projects p ON p.owner_id = u.id AND p.deleted_at IS NULL
LEFT JOIN project_members pm ON pm.user_id = u.id
WHERE u.username IN ('admin', 'Bonomotion', 'Sirak')
GROUP BY u.username
ORDER BY owned_projects DESC;
SQL
echo ""

# Check 6: Recent executions
echo "⚡ Recent Task Executions (Last 5)"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
EXECUTION_COUNT=$(sqlite3 "$DB_PATH" "SELECT COUNT(*) FROM task_executions;")

if [ "$EXECUTION_COUNT" -gt 0 ]; then
    echo "Total executions: $EXECUTION_COUNT"
    echo ""
    sqlite3 "$DB_PATH" <<'SQL'
.mode column
.headers on
SELECT
    execution_type,
    status,
    printf('%.2f', cost_vibe) as cost_vibe,
    datetime(created_at) as executed_at
FROM task_executions
ORDER BY created_at DESC
LIMIT 5;
SQL
else
    echo "⚠️  No task executions yet"
fi
echo ""

# Check 7: VIBE Transactions
echo "💰 VIBE Ledger (Last 5 Transactions)"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
VIBE_COUNT=$(sqlite3 "$DB_PATH" "SELECT COUNT(*) FROM vibe_ledger;")

if [ "$VIBE_COUNT" -gt 0 ]; then
    echo "Total transactions: $VIBE_COUNT"
    echo ""
    sqlite3 "$DB_PATH" <<'SQL'
.mode column
.headers on
SELECT
    transaction_type,
    printf('%.2f', amount) as amount,
    printf('%.2f', balance_after) as balance,
    datetime(created_at) as date
FROM vibe_ledger
ORDER BY created_at DESC
LIMIT 5;
SQL
else
    echo "⚠️  No VIBE transactions yet"
fi
echo ""

# Check 8: System tables
echo "📊 Database Schema"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
REQUIRED_TABLES=("users" "projects" "tasks" "devices" "nodes" "project_members" "task_executions" "vibe_ledger")
MISSING_TABLES=0

for table in "${REQUIRED_TABLES[@]}"; do
    if sqlite3 "$DB_PATH" "SELECT name FROM sqlite_master WHERE type='table' AND name='$table';" | grep -q "$table"; then
        echo "✅ $table"
    else
        echo "❌ $table (missing)"
        MISSING_TABLES=$((MISSING_TABLES + 1))
    fi
done

if [ $MISSING_TABLES -gt 0 ]; then
    echo ""
    echo "⚠️  Warning: $MISSING_TABLES required table(s) missing"
    echo "   Run fix_database.sh to create missing tables"
fi
echo ""

# Summary
echo "═══════════════════════════════════════════════════════════════"
echo "HEALTH CHECK SUMMARY"
echo "═══════════════════════════════════════════════════════════════"
echo ""

# Determine overall health
OVERALL_HEALTH="✅ Healthy"

if [ ! -f "$DB_PATH" ]; then
    OVERALL_HEALTH="❌ Critical - Database missing"
elif [ $MISSING_TABLES -gt 0 ]; then
    OVERALL_HEALTH="⚠️  Warning - Incomplete schema"
elif ! pgrep -f "target/release/server" > /dev/null; then
    OVERALL_HEALTH="⚠️  Warning - Server not running"
fi

echo "Overall Status: $OVERALL_HEALTH"
echo ""

if [ "$OVERALL_HEALTH" != "✅ Healthy" ]; then
    echo "Recommended actions:"
    if [ ! -f "$DB_PATH" ]; then
        echo "  - Run deployment script to initialize database"
    fi
    if [ $MISSING_TABLES -gt 0 ]; then
        echo "  - Run scripts/fix_database.sh to create missing tables"
    fi
    if ! pgrep -f "target/release/server" > /dev/null; then
        echo "  - Start server: cd ~/pcg-cc-mcp && RUST_LOG=info ./target/release/server &"
    fi
    echo ""
fi

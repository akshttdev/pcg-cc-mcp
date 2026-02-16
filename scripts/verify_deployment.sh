#!/bin/bash
# Post-Deployment Verification Script for Bonomotion
# Run this after fix_database.sh to verify everything is correct

set -e

DB_PATH="$HOME/pcg-cc-mcp/dev_assets/db.sqlite"

echo "═══════════════════════════════════════════════════════════════"
echo "ORCHA Deployment Verification - Bonomotion Device"
echo "═══════════════════════════════════════════════════════════════"
echo ""

if [ ! -f "$DB_PATH" ]; then
    echo "❌ Database not found at $DB_PATH"
    exit 1
fi

PASS=0
FAIL=0

# Test 1: Check UUIDs are fixed (32 hex chars, not double-encoded)
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "Test 1: UUID Integrity"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

ADMIN_UUID=$(sqlite3 "$DB_PATH" "SELECT hex(id) FROM users WHERE username = 'admin';")
BONOMOTION_UUID=$(sqlite3 "$DB_PATH" "SELECT hex(id) FROM users WHERE username = 'Bonomotion';")
SIRAK_UUID=$(sqlite3 "$DB_PATH" "SELECT hex(id) FROM users WHERE username = 'Sirak';")

if [ ${#ADMIN_UUID} -eq 32 ] && [ ${#BONOMOTION_UUID} -eq 32 ] && [ ${#SIRAK_UUID} -eq 32 ]; then
    echo "✅ PASS: All UUIDs are 32 hex chars (16 bytes)"
    PASS=$((PASS + 1))
else
    echo "❌ FAIL: UUIDs are still corrupted"
    echo "   Admin: ${#ADMIN_UUID} chars"
    echo "   Bonomotion: ${#BONOMOTION_UUID} chars"
    echo "   Sirak: ${#SIRAK_UUID} chars"
    FAIL=$((FAIL + 1))
fi
echo ""

# Test 2: Check VIBE balances initialized
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "Test 2: VIBE Balance Initialization"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

BONOMOTION_VIBE=$(sqlite3 "$DB_PATH" "SELECT vibe_balance FROM users WHERE username = 'Bonomotion';")

if [ "$(echo "$BONOMOTION_VIBE >= 100.0" | bc)" -eq 1 ]; then
    echo "✅ PASS: Bonomotion VIBE balance initialized ($BONOMOTION_VIBE VIBE)"
    PASS=$((PASS + 1))
else
    echo "❌ FAIL: Bonomotion VIBE balance not initialized ($BONOMOTION_VIBE VIBE)"
    FAIL=$((FAIL + 1))
fi
echo ""

# Test 3: Check new tables exist
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "Test 3: Database Schema (6 New Tables)"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

REQUIRED_TABLES=("devices" "nodes" "project_members" "task_executions" "vibe_ledger" "apn_cloud_capacity")
TABLES_PASS=0

for table in "${REQUIRED_TABLES[@]}"; do
    if sqlite3 "$DB_PATH" "SELECT name FROM sqlite_master WHERE type='table' AND name='$table';" | grep -q "$table"; then
        echo "✅ Table exists: $table"
        TABLES_PASS=$((TABLES_PASS + 1))
    else
        echo "❌ Table missing: $table"
    fi
done

if [ $TABLES_PASS -eq 6 ]; then
    echo "✅ PASS: All 6 new tables created"
    PASS=$((PASS + 1))
else
    echo "❌ FAIL: Only $TABLES_PASS/6 tables created"
    FAIL=$((FAIL + 1))
fi
echo ""

# Test 4: Check Master Node registration
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "Test 4: Master Node Registration"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

BONOMOTION_DEVICE=$(sqlite3 "$DB_PATH" "SELECT COUNT(*) FROM devices WHERE owner_id = X'82EF3C4E943C4678925180629208A183' AND device_tier = 'master_node';")

if [ "$BONOMOTION_DEVICE" -eq 1 ]; then
    DEVICE_INFO=$(sqlite3 "$DB_PATH" "SELECT id, hostname, total_cores, total_ram_gb FROM devices WHERE owner_id = X'82EF3C4E943C4678925180629208A183';")
    echo "✅ PASS: Bonomotion Mac Studio registered as Master Node"
    echo "   $DEVICE_INFO"
    PASS=$((PASS + 1))
else
    echo "❌ FAIL: Bonomotion Master Node not registered"
    FAIL=$((FAIL + 1))
fi
echo ""

# Test 5: Check NO project sharing (privacy boundary)
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "Test 5: Multi-Tenant Privacy (No Cross-Tenant Sharing)"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

BONOMOTION_SHARED=$(sqlite3 "$DB_PATH" "SELECT COUNT(*) FROM project_members WHERE user_id = X'82EF3C4E943C4678925180629208A183';")

if [ "$BONOMOTION_SHARED" -eq 0 ]; then
    echo "✅ PASS: No projects shared with Bonomotion (correct - multi-tenant privacy)"
    PASS=$((PASS + 1))
else
    echo "❌ FAIL: $BONOMOTION_SHARED projects incorrectly shared with Bonomotion"
    FAIL=$((FAIL + 1))
fi
echo ""

# Test 6: Check project ownership counts
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "Test 6: Project Ownership Distribution"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

sqlite3 "$DB_PATH" <<'SQL'
SELECT
    u.username,
    COUNT(p.id) as owned_projects,
    CASE
        WHEN u.username = 'admin' AND COUNT(p.id) = 32 THEN '✅'
        WHEN u.username = 'Bonomotion' AND COUNT(p.id) <= 1 THEN '✅'
        WHEN u.username = 'Sirak' AND COUNT(p.id) = 1 THEN '✅'
        ELSE '❌'
    END as status
FROM users u
LEFT JOIN projects p ON p.owner_id = u.id AND p.deleted_at IS NULL
WHERE u.username IN ('admin', 'Bonomotion', 'Sirak')
GROUP BY u.username;
SQL

BONOMOTION_PROJECTS=$(sqlite3 "$DB_PATH" "SELECT COUNT(p.id) FROM users u LEFT JOIN projects p ON p.owner_id = u.id AND p.deleted_at IS NULL WHERE u.username = 'Bonomotion' GROUP BY u.username;")

if [ "$BONOMOTION_PROJECTS" -le 1 ]; then
    echo "✅ PASS: Bonomotion has $BONOMOTION_PROJECTS projects (expected 0-1)"
    PASS=$((PASS + 1))
else
    echo "❌ FAIL: Bonomotion has $BONOMOTION_PROJECTS projects (expected 0-1)"
    FAIL=$((FAIL + 1))
fi
echo ""

# Test 7: Check server is running
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "Test 7: ORCHA Server Status"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

if pgrep -f "target/release/server" > /dev/null; then
    echo "✅ PASS: ORCHA server is running"
    PASS=$((PASS + 1))
else
    echo "⚠️  WARNING: ORCHA server is not running"
    echo "   Start with: cd ~/pcg-cc-mcp && RUST_LOG=info ./target/release/server &"
fi
echo ""

# Summary
echo "═══════════════════════════════════════════════════════════════"
echo "VERIFICATION SUMMARY"
echo "═══════════════════════════════════════════════════════════════"
echo ""
echo "Tests Passed: $PASS"
echo "Tests Failed: $FAIL"
echo ""

if [ $FAIL -eq 0 ]; then
    echo "✅ ALL TESTS PASSED - DEPLOYMENT SUCCESSFUL"
    echo ""
    echo "Next steps:"
    echo "1. Open browser and navigate to ORCHA dashboard"
    echo "2. Sign in as: bonomotion@powerclubglobal.com"
    echo "3. Verify you see 0-1 projects (your own)"
    echo "4. Verify you do NOT see Admin's ORCHA or Powerclub Global"
    echo ""
    exit 0
else
    echo "❌ DEPLOYMENT VERIFICATION FAILED"
    echo ""
    echo "Please review failed tests above and:"
    echo "1. Check fix_database.sh ran successfully"
    echo "2. Check for error messages in server logs"
    echo "3. Consider running rollback script if needed"
    echo ""
    exit 1
fi

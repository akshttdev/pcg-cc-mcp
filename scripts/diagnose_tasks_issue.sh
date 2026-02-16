#!/bin/bash
# Diagnostic Script for Tasks Not Loading Issue
# Checks topology filtering and API query structure

DB_PATH="$HOME/pcg-cc-mcp/dev_assets/db.sqlite"

echo "═══════════════════════════════════════════════════════════════"
echo "ORCHA Tasks Loading Diagnostic"
echo "═══════════════════════════════════════════════════════════════"
echo ""

if [ ! -f "$DB_PATH" ]; then
    echo "❌ Database not found at $DB_PATH"
    exit 1
fi

# Test 1: Check if tasks table exists
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "Test 1: Tasks Table Structure"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
sqlite3 "$DB_PATH" ".schema tasks"
echo ""

# Test 2: Total task counts
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "Test 2: Task Counts by User"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
sqlite3 "$DB_PATH" <<'SQL'
.mode column
.headers on
SELECT
    u.username,
    COUNT(DISTINCT p.id) as projects,
    COUNT(t.id) as tasks,
    COUNT(CASE WHEN t.status = 'done' THEN 1 END) as done,
    COUNT(CASE WHEN t.status = 'in_progress' THEN 1 END) as in_progress,
    COUNT(CASE WHEN t.status = 'todo' THEN 1 END) as todo
FROM users u
LEFT JOIN projects p ON p.owner_id = u.id AND p.deleted_at IS NULL
LEFT JOIN tasks t ON t.project_id = p.id AND t.deleted_at IS NULL
GROUP BY u.username
ORDER BY tasks DESC;
SQL
echo ""

# Test 3: Test topology-aware task query for Admin
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "Test 3: Topology-Aware Task Query (Admin)"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

ADMIN_UUID="F8EB8F0268963FDB698AB9DFE836E7BE"

sqlite3 "$DB_PATH" <<SQL
.mode column
.headers on

-- This simulates the topology-aware query from topology.rs
SELECT
    t.id,
    p.name as project,
    t.title,
    t.status,
    t.priority
FROM tasks t
JOIN projects p ON t.project_id = p.id
WHERE t.deleted_at IS NULL
  AND p.deleted_at IS NULL
  AND (
      p.owner_id = X'$ADMIN_UUID'
      OR EXISTS (
          SELECT 1 FROM project_members pm
          WHERE pm.project_id = p.id
            AND pm.user_id = X'$ADMIN_UUID'
      )
  )
ORDER BY t.created_at DESC
LIMIT 10;
SQL
echo ""

# Test 4: Check for tasks with missing project relationships
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "Test 4: Data Integrity Checks"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

sqlite3 "$DB_PATH" <<'SQL'
-- Tasks with invalid project_id
SELECT 'Tasks with invalid project_id:' as check, COUNT(*) as count
FROM tasks t
WHERE t.deleted_at IS NULL
AND NOT EXISTS (SELECT 1 FROM projects p WHERE p.id = t.project_id);

-- Projects with invalid owner_id
SELECT 'Projects with invalid owner_id:' as check, COUNT(*) as count
FROM projects p
WHERE p.deleted_at IS NULL
AND NOT EXISTS (SELECT 1 FROM users u WHERE u.id = p.owner_id);

-- Tasks created by deleted users
SELECT 'Tasks by non-existent users:' as check, COUNT(*) as count
FROM tasks t
WHERE t.deleted_at IS NULL
AND NOT EXISTS (SELECT 1 FROM users u WHERE u.id = t.created_by_user_id);
SQL
echo ""

# Test 5: Check if project_id is binary vs text
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "Test 5: ID Format Check (should be 32 hex chars)"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

sqlite3 "$DB_PATH" <<'SQL'
.mode list
SELECT
    'Task ID length:' as type,
    LENGTH(hex(id)) as hex_length,
    COUNT(*) as count
FROM tasks
WHERE deleted_at IS NULL
GROUP BY LENGTH(hex(id))
UNION ALL
SELECT
    'Project ID length:',
    LENGTH(hex(id)),
    COUNT(*)
FROM projects
WHERE deleted_at IS NULL
GROUP BY LENGTH(hex(id))
UNION ALL
SELECT
    'User ID length:',
    LENGTH(hex(id)),
    COUNT(*)
FROM users
GROUP BY LENGTH(hex(id));
SQL
echo ""

# Test 6: Check server logs for errors
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "Test 6: Recent Server Errors (if log exists)"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

if [ -f ~/pcg-cc-mcp/orcha.log ]; then
    echo "Last 10 errors from orcha.log:"
    grep -i "error\|warn\|fail" ~/pcg-cc-mcp/orcha.log | tail -10
elif [ -f /tmp/orcha-server.log ]; then
    echo "Last 10 errors from /tmp/orcha-server.log:"
    grep -i "error\|warn\|fail" /tmp/orcha-server.log | tail -10
else
    echo "No server log found at ~/pcg-cc-mcp/orcha.log or /tmp/orcha-server.log"
    echo "Check where server is writing logs"
fi
echo ""

echo "═══════════════════════════════════════════════════════════════"
echo "DIAGNOSTIC COMPLETE"
echo "═══════════════════════════════════════════════════════════════"
echo ""
echo "Summary:"
echo "  • Tasks exist in database: Check Test 2"
echo "  • Topology filtering working: Check Test 3"
echo "  • Data integrity: Check Test 4"
echo "  • ID formats correct: Check Test 5"
echo "  • Server errors: Check Test 6"
echo ""
echo "If tasks exist but don't show in dashboard:"
echo "  1. Check server logs for API errors"
echo "  2. Check browser console for JavaScript errors"
echo "  3. Verify API endpoints use topology-aware queries"
echo "  4. Check if frontend is querying correct endpoints"
echo ""

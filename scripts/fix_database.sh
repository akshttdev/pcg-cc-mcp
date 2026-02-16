#!/bin/bash
# Complete Database Fix for Bonomotion Device (CORRECTED)
# Fixes UUIDs, creates tables, initializes VIBE balances
# NO PROJECT SHARING - Each user sees only their own projects

set -e

DB_PATH="$HOME/pcg-cc-mcp/dev_assets/db.sqlite"

echo "═══════════════════════════════════════════════════════════════"
echo "ORCHA Database Fix for Bonomotion Device (CORRECTED)"
echo "═══════════════════════════════════════════════════════════════"
echo ""

if [ ! -f "$DB_PATH" ]; then
    echo "❌ Database not found at $DB_PATH"
    exit 1
fi

echo "✓ Database found at $DB_PATH"
echo ""

# Backup database first
cp "$DB_PATH" "$DB_PATH.backup.$(date +%s)"
echo "✓ Database backed up"
echo ""

echo "Applying fixes..."
echo ""

sqlite3 "$DB_PATH" <<'SQL'
-- ═══════════════════════════════════════════════════════════════════════════
-- CRITICAL FIX: Corrupted UUIDs
-- ═══════════════════════════════════════════════════════════════════════════

-- Fix Sirak's UUID (was double-encoded)
UPDATE users
SET id = X'93EE4745203B4FED8F9184315E5C3B3B'
WHERE hex(id) = '39336565343734352D323033622D346665642D386639312D383433313565356333623362';

-- Fix Bonomotion's UUID (was double-encoded)
UPDATE users
SET id = X'82EF3C4E943C4678925180629208A183'
WHERE hex(id) = '38326566336334652D393433632D343637382D393235312D383036323932303861313833';

-- Fix project ownership for Sirak's project
UPDATE projects
SET owner_id = X'93EE4745203B4FED8F9184315E5C3B3B'
WHERE hex(owner_id) = '39336565343734352D323033622D346665642D386639312D383433313565356333623362';

-- Clear corrupted sessions
DELETE FROM sessions;

-- ═══════════════════════════════════════════════════════════════════════════
-- Create New Tables
-- ═══════════════════════════════════════════════════════════════════════════

CREATE TABLE IF NOT EXISTS devices (
    id TEXT PRIMARY KEY,
    owner_id BLOB NOT NULL,
    hostname TEXT NOT NULL,
    wallet_address TEXT NOT NULL,
    device_tier TEXT NOT NULL CHECK (device_tier IN ('master_node', 'relay', 'client')),
    uptime_percent REAL DEFAULT 0.0,
    is_online INTEGER DEFAULT 0,
    total_cores INTEGER NOT NULL,
    total_ram_gb INTEGER NOT NULL,
    gpu_available INTEGER DEFAULT 0,
    gpu_model TEXT,
    active_contribution_percent REAL DEFAULT 10.0,
    idle_contribution_percent REAL DEFAULT 80.0,
    is_primary_node INTEGER DEFAULT 0,
    vibe_earned_total REAL DEFAULT 0.0,
    last_seen DATETIME,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (owner_id) REFERENCES users(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS nodes (
    id TEXT PRIMARY KEY,
    owner_id BLOB NOT NULL,
    hostname TEXT NOT NULL,
    is_primary INTEGER DEFAULT 0,
    node_type TEXT DEFAULT 'master' CHECK (node_type IN ('master', 'backup')),
    last_seen DATETIME,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (owner_id) REFERENCES users(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS project_members (
    id BLOB PRIMARY KEY,
    project_id BLOB NOT NULL,
    user_id BLOB NOT NULL,
    role TEXT NOT NULL CHECK (role IN ('owner', 'editor', 'contributor', 'viewer')),
    permissions TEXT,
    granted_by BLOB,
    granted_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(project_id, user_id),
    FOREIGN KEY (project_id) REFERENCES projects(id) ON DELETE CASCADE,
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS task_executions (
    id BLOB PRIMARY KEY,
    task_id BLOB NOT NULL,
    user_id BLOB NOT NULL,
    execution_type TEXT NOT NULL CHECK (execution_type IN ('master_node', 'apn_cloud')),
    executed_on_device_ids TEXT,
    cost_vibe REAL DEFAULT 0.0,
    vibe_balance_before REAL,
    vibe_balance_after REAL,
    status TEXT NOT NULL CHECK (status IN ('pending', 'running', 'completed', 'failed', 'cancelled')),
    started_at DATETIME,
    completed_at DATETIME,
    duration_seconds INTEGER,
    cores_used INTEGER,
    ram_used_gb REAL,
    error_message TEXT,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (task_id) REFERENCES tasks(id) ON DELETE CASCADE,
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS vibe_ledger (
    id BLOB PRIMARY KEY,
    user_id BLOB NOT NULL,
    device_id TEXT,
    amount REAL NOT NULL,
    transaction_type TEXT NOT NULL CHECK (transaction_type IN ('earn_contribution', 'earn_relay', 'spend_compute', 'bonus_uptime', 'network_fee', 'initial_balance', 'transfer_in', 'transfer_out')),
    balance_before REAL NOT NULL,
    balance_after REAL NOT NULL,
    task_execution_id BLOB,
    description TEXT,
    metadata TEXT,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS apn_cloud_capacity (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    measured_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    total_cores REAL NOT NULL,
    available_cores REAL NOT NULL,
    total_ram_gb REAL NOT NULL,
    available_ram_gb REAL NOT NULL,
    gpus_available INTEGER NOT NULL,
    master_nodes_online INTEGER NOT NULL,
    relay_devices_online INTEGER NOT NULL,
    tasks_queued INTEGER DEFAULT 0,
    tasks_executing INTEGER DEFAULT 0
);

-- ═══════════════════════════════════════════════════════════════════════════
-- Initialize VIBE Balances
-- ═══════════════════════════════════════════════════════════════════════════

-- Add vibe_balance column (will fail silently if exists)
.bail off
ALTER TABLE users ADD COLUMN vibe_balance REAL DEFAULT 100.0;
.bail on

-- Set initial balances
UPDATE users SET vibe_balance = 1000.0 WHERE id = X'F8EB8F0268963FDB698AB9DFE836E7BE'; -- Admin
UPDATE users SET vibe_balance = 100.0 WHERE id = X'82EF3C4E943C4678925180629208A183';  -- Bonomotion
UPDATE users SET vibe_balance = 50.0 WHERE id = X'93EE4745203B4FED8F9184315E5C3B3B';   -- Sirak

-- ═══════════════════════════════════════════════════════════════════════════
-- Register Bonomotion's Master Node
-- ═══════════════════════════════════════════════════════════════════════════

-- Bonomotion Mac Studio
INSERT OR REPLACE INTO devices (
    id, owner_id, hostname, wallet_address, device_tier,
    uptime_percent, is_online, total_cores, total_ram_gb,
    gpu_available, gpu_model, is_primary_node
) VALUES (
    'bonomotion-mac-studio',
    X'82EF3C4E943C4678925180629208A183',
    'bonomotion-mac-studio',
    'bonomotion-mac-studio',
    'master_node',
    97.5, 1, 16, 128,
    0, 'Apple M2 Ultra (integrated)', 1
);

INSERT OR REPLACE INTO nodes (id, owner_id, hostname, is_primary, node_type)
VALUES ('bonomotion-mac-studio', X'82EF3C4E943C4678925180629208A183', 'bonomotion-mac-studio', 1, 'master');

-- ═══════════════════════════════════════════════════════════════════════════
-- NO PROJECT SHARING - Each user sees only their own projects
-- ═══════════════════════════════════════════════════════════════════════════

-- Admin's projects (ORCHA, Powerclub Global, etc.) stay PRIVATE to Admin
-- Bonomotion's local project (if exists) stays PRIVATE to Bonomotion
-- Sirak's project stays PRIVATE to Sirak

-- ═══════════════════════════════════════════════════════════════════════════
-- Verification
-- ═══════════════════════════════════════════════════════════════════════════

SELECT '=== USERS (Fixed UUIDs) ===' as info;
SELECT hex(id) as user_id, username, vibe_balance FROM users
WHERE username IN ('admin', 'Sirak', 'Bonomotion');

SELECT '' as info;
SELECT '=== BONOMOTION DEVICE ===' as info;
SELECT id, hostname, device_tier, total_cores, total_ram_gb FROM devices
WHERE owner_id = X'82EF3C4E943C4678925180629208A183';

SELECT '' as info;
SELECT '=== PROJECT OWNERSHIP (No Sharing) ===' as info;
SELECT u.username, COUNT(p.id) as owned_projects
FROM users u
LEFT JOIN projects p ON p.owner_id = u.id AND p.deleted_at IS NULL
WHERE u.username IN ('admin', 'Sirak', 'Bonomotion')
GROUP BY u.username;

SQL

echo ""
echo "═══════════════════════════════════════════════════════════════"
echo "✅ DATABASE FIX COMPLETE"
echo "═══════════════════════════════════════════════════════════════"
echo ""
echo "Next steps:"
echo "1. Restart ORCHA server: pkill -f server && cd ~/pcg-cc-mcp && RUST_LOG=info ./target/release/server &"
echo "2. Open dashboard and test sign-in"
echo ""
echo "Expected results:"
echo "  • Bonomotion: Can sign in, sees own projects only (0 or 1)"
echo "  • Sirak: Can sign in, sees own project (1)"
echo "  • Admin: Can sign in, sees own projects (32)"
echo "  • NO PROJECT SHARING - Each user's projects are PRIVATE"
echo ""

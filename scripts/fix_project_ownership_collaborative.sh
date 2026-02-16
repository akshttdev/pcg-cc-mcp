#!/bin/bash
# Fix Project Ownership and Enable Project-Centric Collaboration
# Implements mesh networking model where project members share access

set -e

DB_PATH="$HOME/pcg-cc-mcp/dev_assets/db.sqlite"

echo "═══════════════════════════════════════════════════════════════"
echo "ORCHA Project-Centric Collaboration Model"
echo "═══════════════════════════════════════════════════════════════"
echo ""

if [ ! -f "$DB_PATH" ]; then
    echo "❌ Database not found at $DB_PATH"
    exit 1
fi

# Backup database
BACKUP_FILE="$DB_PATH.backup_collaborative_model_$(date +%s)"
cp "$DB_PATH" "$BACKUP_FILE"
echo "✓ Database backed up to: $BACKUP_FILE"
echo ""

echo "Implementing Project-Centric Collaboration..."
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""

sqlite3 "$DB_PATH" <<'SQL'
-- ═══════════════════════════════════════════════════════════════════════════
-- FIX PROJECT OWNERSHIP
-- ═══════════════════════════════════════════════════════════════════════════

-- Fix: Powerclub Global should be owned by Admin (not Sirak)
UPDATE projects
SET owner_id = X'F8EB8F0268963FDB698AB9DFE836E7BE'
WHERE name = 'Powerclub Global'
  AND deleted_at IS NULL;

-- Transfer: Prime Hospitality to Sirak
UPDATE projects
SET owner_id = X'93EE4745203B4FED8F9184315E5C3B3B'
WHERE name = 'Prime Hospitality'
  AND deleted_at IS NULL;

-- Transfer: sirak-studios to Sirak
UPDATE projects
SET owner_id = X'93EE4745203B4FED8F9184315E5C3B3B'
WHERE (name = 'sirak-studios' OR name = 'Sirak Studios')
  AND deleted_at IS NULL;

-- ═══════════════════════════════════════════════════════════════════════════
-- IMPLEMENT PROJECT-CENTRIC COLLABORATION
-- Devices/users linked through projects share access by default
-- ═══════════════════════════════════════════════════════════════════════════

-- Clear old isolation model
DELETE FROM project_members WHERE 1=1;

-- Add collaborative project memberships
-- These enable mesh networking and resource pooling

-- ORCHA Project: Collaborative development (Admin, Sirak, Bonomotion)
INSERT OR IGNORE INTO project_members (id, project_id, user_id, role, granted_by)
SELECT
    randomblob(16),
    p.id,
    u.id,
    'editor',
    p.owner_id
FROM projects p
CROSS JOIN users u
WHERE p.name = 'ORCHA'
  AND u.username IN ('admin', 'Sirak', 'Bonomotion')
  AND p.deleted_at IS NULL;

-- Prime Hospitality: Sirak's project, shared with Admin for oversight
INSERT OR IGNORE INTO project_members (id, project_id, user_id, role, granted_by)
SELECT
    randomblob(16),
    p.id,
    (SELECT id FROM users WHERE username = 'admin'),
    'editor',
    p.owner_id
FROM projects p
WHERE p.name = 'Prime Hospitality'
  AND p.deleted_at IS NULL;

-- Powerclub Global: Admin's project, shared with all team members
INSERT OR IGNORE INTO project_members (id, project_id, user_id, role, granted_by)
SELECT
    randomblob(16),
    p.id,
    u.id,
    CASE
        WHEN u.username = 'Sirak' THEN 'editor'
        WHEN u.username = 'Bonomotion' THEN 'contributor'
        ELSE 'viewer'
    END,
    p.owner_id
FROM projects p
CROSS JOIN users u
WHERE p.name = 'Powerclub Global'
  AND u.username IN ('Sirak', 'Bonomotion', 'Andre', 'Eric', 'Madhav', 'Prat')
  AND p.deleted_at IS NULL;

-- Sirak Studios: Sirak's project, shared with Admin
INSERT OR IGNORE INTO project_members (id, project_id, user_id, role, granted_by)
SELECT
    randomblob(16),
    p.id,
    (SELECT id FROM users WHERE username = 'admin'),
    'viewer',
    p.owner_id
FROM projects p
WHERE (p.name = 'sirak-studios' OR p.name = 'Sirak Studios')
  AND p.deleted_at IS NULL;

-- ═══════════════════════════════════════════════════════════════════════════
-- ENABLE DEVICE RESOURCE POOLING
-- Devices in same project pool compute resources via APN
-- ═══════════════════════════════════════════════════════════════════════════

-- Create device_project_access table for mesh networking
CREATE TABLE IF NOT EXISTS device_project_access (
    id BLOB PRIMARY KEY,
    device_id TEXT NOT NULL,
    project_id BLOB NOT NULL,
    access_level TEXT NOT NULL CHECK (access_level IN ('compute', 'storage', 'full')),
    granted_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (device_id) REFERENCES devices(id) ON DELETE CASCADE,
    FOREIGN KEY (project_id) REFERENCES projects(id) ON DELETE CASCADE,
    UNIQUE(device_id, project_id)
);

-- Grant all devices access to projects their owners are members of
INSERT OR IGNORE INTO device_project_access (id, device_id, project_id, access_level)
SELECT
    randomblob(16),
    d.id,
    pm.project_id,
    'full'
FROM devices d
JOIN project_members pm ON pm.user_id = d.owner_id;

-- ═══════════════════════════════════════════════════════════════════════════
-- VERIFICATION
-- ═══════════════════════════════════════════════════════════════════════════

SELECT '=== PROJECT OWNERSHIP ===' as info;
SELECT
    p.name,
    u.username as owner,
    COUNT(DISTINCT pm.user_id) as members
FROM projects p
JOIN users u ON p.owner_id = u.id
LEFT JOIN project_members pm ON pm.project_id = p.id
WHERE p.deleted_at IS NULL
  AND p.name IN ('ORCHA', 'Prime Hospitality', 'Powerclub Global', 'sirak-studios', 'Sirak Studios')
GROUP BY p.name, u.username;

SELECT '' as info;
SELECT '=== SIRAK ACCESS ===' as info;
SELECT
    p.name as project,
    CASE
        WHEN p.owner_id = X'93EE4745203B4FED8F9184315E5C3B3B' THEN 'Owner'
        ELSE pm.role
    END as access_level
FROM projects p
LEFT JOIN project_members pm ON pm.project_id = p.id AND pm.user_id = X'93EE4745203B4FED8F9184315E5C3B3B'
WHERE p.deleted_at IS NULL
  AND (p.owner_id = X'93EE4745203B4FED8F9184315E5C3B3B'
       OR pm.user_id = X'93EE4745203B4FED8F9184315E5C3B3B')
ORDER BY p.name;

SELECT '' as info;
SELECT '=== DEVICE RESOURCE POOLING ===' as info;
SELECT
    d.hostname,
    p.name as project,
    dpa.access_level
FROM device_project_access dpa
JOIN devices d ON dpa.device_id = d.id
JOIN projects p ON dpa.project_id = p.id
WHERE p.deleted_at IS NULL
ORDER BY d.hostname, p.name;

SQL

echo ""
echo "═══════════════════════════════════════════════════════════════"
echo "✅ PROJECT-CENTRIC COLLABORATION ENABLED"
echo "═══════════════════════════════════════════════════════════════"
echo ""
echo "Architecture:"
echo "  • Projects are collaboration boundaries"
echo "  • Users in same project share data access"
echo "  • Devices pool compute resources via APN mesh"
echo "  • Distributed execution across project members' devices"
echo ""
echo "Sirak now has:"
echo "  ✅ Prime Hospitality (Owner)"
echo "  ✅ Sirak Studios (Owner)"
echo "  ✅ Access to shared collaborative projects"
echo ""
echo "Next steps:"
echo "1. Restart ORCHA server:"
echo "   pkill -f server"
echo "   cd ~/pcg-cc-mcp"
echo "   RUST_LOG=info ./target/release/server &"
echo ""
echo "2. Verify in dashboard:"
echo "   • Sirak sees 2+ projects (owns 2, member of others)"
echo "   • Project members can access shared data"
echo "   • Tasks can execute on any project member's device"
echo ""

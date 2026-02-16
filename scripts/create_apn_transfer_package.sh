#!/bin/bash
# Create APN Transfer Package for Bonomotion Device
# Packages database export for transfer via APN

set -e

PACKAGE_DIR="/tmp/apn_transfer_bonomotion_$(date +%s)"
DB_PATH="$HOME/pcg-cc-mcp/dev_assets/db.sqlite"

echo "═══════════════════════════════════════════════════════════════"
echo "Creating APN Transfer Package for Bonomotion"
echo "═══════════════════════════════════════════════════════════════"
echo ""

if [ ! -f "$DB_PATH" ]; then
    echo "❌ Database not found at $DB_PATH"
    exit 1
fi

# Create package directory
mkdir -p "$PACKAGE_DIR"
echo "✓ Created package directory: $PACKAGE_DIR"
echo ""

# Export database schema only (no Admin's private data)
echo "📦 Creating minimal database export (schema + VIBE balances only)..."

sqlite3 "$DB_PATH" > "$PACKAGE_DIR/schema_export.sql" <<'SQL'
.output
.mode insert

-- Export only essential data (NO PROJECT DATA)
-- This preserves Bonomotion's local projects while fixing structure

-- Export users table (UUIDs and VIBE balances)
SELECT 'DELETE FROM users WHERE username IN (''admin'', ''Sirak'', ''Bonomotion'');';
.mode insert users
SELECT * FROM users WHERE username IN ('admin', 'Sirak', 'Bonomotion');

-- Export new table schemas
.schema devices
.schema nodes
.schema project_members
.schema task_executions
.schema vibe_ledger
.schema apn_cloud_capacity

-- Export Bonomotion's device registration
.mode insert devices
SELECT * FROM devices WHERE owner_id = X'82EF3C4E943C4678925180629208A183';

.mode insert nodes
SELECT * FROM nodes WHERE owner_id = X'82EF3C4E943C4678925180629208A183';

SQL

echo "✓ Schema export created"
echo ""

# Copy deployment scripts
echo "📄 Copying deployment scripts..."
cp "$HOME/pcg-cc-mcp/scripts/fix_database.sh" "$PACKAGE_DIR/"
cp "$HOME/pcg-cc-mcp/scripts/BONOMOTION_REMOTE_DEPLOY.md" "$PACKAGE_DIR/README.md"
echo "✓ Scripts copied"
echo ""

# Create manifest
cat > "$PACKAGE_DIR/MANIFEST.txt" <<EOF
APN Transfer Package for Bonomotion Device
═══════════════════════════════════════════

Created: $(date)
Target: Bonomotion Mac Studio
Branch: bonomotion
Commit: $(cd ~/pcg-cc-mcp && git rev-parse --short HEAD)

Contents:
─────────────────────────────────────────
• schema_export.sql          - Minimal database schema + user data
• fix_database.sh            - Complete database fix script
• README.md                  - Deployment instructions

Deployment Method:
─────────────────────────────────────────
OPTION 1: Run fix_database.sh (Recommended)
  - Preserves Bonomotion's local project
  - Fixes UUIDs, creates tables, initializes VIBE
  - Does NOT import Admin's private projects

OPTION 2: Import schema_export.sql
  - Minimal data transfer
  - Requires manual application

Expected Results:
─────────────────────────────────────────
• Bonomotion: Sees 0-1 projects (own only)
• Sirak: Sees 1 project (own only)
• Admin: Sees 32 projects (own only)
• NO PROJECT SHARING between users

Size: $(du -sh "$PACKAGE_DIR" | cut -f1)
EOF

echo "✓ Manifest created"
echo ""

# Create compressed archive
echo "📦 Creating compressed archive for APN transfer..."
cd "$(dirname "$PACKAGE_DIR")"
PACKAGE_NAME="bonomotion_deployment_$(date +%Y%m%d_%H%M%S).tar.gz"
tar -czf "$PACKAGE_NAME" "$(basename "$PACKAGE_DIR")"

PACKAGE_PATH="$(dirname "$PACKAGE_DIR")/$PACKAGE_NAME"
PACKAGE_SIZE=$(du -h "$PACKAGE_PATH" | cut -f1)

echo "✓ Package created"
echo ""

# Cleanup temp directory
rm -rf "$PACKAGE_DIR"

echo "═══════════════════════════════════════════════════════════════"
echo "✅ APN TRANSFER PACKAGE READY"
echo "═══════════════════════════════════════════════════════════════"
echo ""
echo "Package: $PACKAGE_PATH"
echo "Size: $PACKAGE_SIZE"
echo ""
echo "Next steps:"
echo "1. Transfer package to Bonomotion device via APN:"
echo "   apn-send --target bonomotion-mac-studio --file $PACKAGE_PATH"
echo ""
echo "2. On Bonomotion device, extract and deploy:"
echo "   cd /tmp"
echo "   tar -xzf $PACKAGE_NAME"
echo "   cd $(basename "$PACKAGE_DIR")"
echo "   chmod +x fix_database.sh"
echo "   ./fix_database.sh"
echo ""
echo "3. Restart ORCHA server on Bonomotion"
echo ""

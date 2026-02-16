#!/bin/bash
# Rollback Script for Bonomotion Deployment
# Use this if deployment fails or causes issues

set -e

DB_PATH="$HOME/pcg-cc-mcp/dev_assets/db.sqlite"
BACKUP_DIR="$HOME/pcg-cc-mcp/dev_assets"

echo "═══════════════════════════════════════════════════════════════"
echo "ORCHA Deployment Rollback - Bonomotion Device"
echo "═══════════════════════════════════════════════════════════════"
echo ""
echo "⚠️  WARNING: This will restore the database to its pre-deployment state"
echo ""

# Find the most recent backup
LATEST_BACKUP=$(ls -t "$BACKUP_DIR"/db.sqlite.backup.* 2>/dev/null | head -1)

if [ -z "$LATEST_BACKUP" ]; then
    echo "❌ No backup found in $BACKUP_DIR"
    echo "   Cannot rollback without a backup"
    exit 1
fi

BACKUP_DATE=$(echo "$LATEST_BACKUP" | grep -oP 'backup\.\K\d+')
BACKUP_TIME=$(date -d @"$BACKUP_DATE" '+%Y-%m-%d %H:%M:%S' 2>/dev/null || echo "timestamp: $BACKUP_DATE")

echo "Found backup: $LATEST_BACKUP"
echo "Created: $BACKUP_TIME"
echo ""

# Confirm rollback
read -p "Are you sure you want to rollback? (yes/no): " CONFIRM

if [ "$CONFIRM" != "yes" ]; then
    echo "Rollback cancelled"
    exit 0
fi

echo ""
echo "Starting rollback..."
echo ""

# Stop server
echo "1. Stopping ORCHA server..."
pkill -f "target/release/server" 2>/dev/null || echo "   (Server was not running)"
sleep 2
echo "   ✓ Server stopped"
echo ""

# Create a backup of current state (just in case)
echo "2. Backing up current database state..."
CURRENT_BACKUP="$DB_PATH.pre-rollback.$(date +%s)"
cp "$DB_PATH" "$CURRENT_BACKUP"
echo "   ✓ Current state saved to: $CURRENT_BACKUP"
echo ""

# Restore backup
echo "3. Restoring database from backup..."
cp "$LATEST_BACKUP" "$DB_PATH"
echo "   ✓ Database restored from: $LATEST_BACKUP"
echo ""

# Verify restoration
echo "4. Verifying restoration..."
if [ -f "$DB_PATH" ]; then
    DB_SIZE=$(du -h "$DB_PATH" | cut -f1)
    echo "   ✓ Database restored successfully ($DB_SIZE)"
else
    echo "   ❌ Restoration failed - database file not found"
    exit 1
fi
echo ""

# Check git status
echo "5. Checking git status..."
cd "$HOME/pcg-cc-mcp"
CURRENT_BRANCH=$(git branch --show-current)
echo "   Current branch: $CURRENT_BRANCH"

if [ "$CURRENT_BRANCH" = "bonomotion" ]; then
    echo ""
    echo "   To fully rollback code changes, run:"
    echo "   cd ~/pcg-cc-mcp"
    echo "   git checkout main"
    echo "   # or"
    echo "   git reset --hard HEAD~3  # to undo last 3 commits"
fi
echo ""

echo "═══════════════════════════════════════════════════════════════"
echo "✅ ROLLBACK COMPLETE"
echo "═══════════════════════════════════════════════════════════════"
echo ""
echo "Database restored to pre-deployment state"
echo ""
echo "Next steps:"
echo "1. Restart server: cd ~/pcg-cc-mcp && RUST_LOG=info ./target/release/server &"
echo "2. Test authentication to verify rollback successful"
echo "3. If issues persist, check server logs"
echo ""
echo "Backup locations:"
echo "   Original backup: $LATEST_BACKUP"
echo "   Pre-rollback state: $CURRENT_BACKUP"
echo ""

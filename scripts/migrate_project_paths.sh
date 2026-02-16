#!/bin/bash
# Migrate Project Paths from Space Terminal to Pythia
# Updates git_repo_path to point to Pythia locations

set -e

DB_PATH="$HOME/pcg-cc-mcp/dev_assets/db.sqlite"

echo "═══════════════════════════════════════════════════════════════"
echo "ORCHA Project Path Migration - Space Terminal → Pythia"
echo "═══════════════════════════════════════════════════════════════"
echo ""

if [ ! -f "$DB_PATH" ]; then
    echo "❌ Database not found at $DB_PATH"
    exit 1
fi

# Backup database first
BACKUP_FILE="$DB_PATH.backup_before_path_migration_$(date +%s)"
cp "$DB_PATH" "$BACKUP_FILE"
echo "✓ Database backed up to: $BACKUP_FILE"
echo ""

echo "Current project paths:"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
sqlite3 "$DB_PATH" "
SELECT
    SUBSTR(git_repo_path, 1, 30) || '...' as path_prefix,
    COUNT(*) as count
FROM projects
WHERE deleted_at IS NULL
GROUP BY path_prefix
ORDER BY count DESC;
"
echo ""

echo "Updating paths..."
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

sqlite3 "$DB_PATH" <<'SQL'
-- Update /home/spaceterminal/topos → /home/pythia/topos
UPDATE projects
SET
    git_repo_path = REPLACE(git_repo_path, '/home/spaceterminal/topos', '/home/pythia/topos'),
    updated_at = datetime('now', 'subsec')
WHERE git_repo_path LIKE '/home/spaceterminal/topos%'
  AND deleted_at IS NULL;

-- Update /Users/bodhi/Documents → /home/pythia/Documents
UPDATE projects
SET
    git_repo_path = REPLACE(git_repo_path, '/Users/bodhi/Documents', '/home/pythia/Documents'),
    updated_at = datetime('now', 'subsec')
WHERE git_repo_path LIKE '/Users/bodhi/Documents%'
  AND deleted_at IS NULL;

-- Update /app → /home/pythia/topos/apps
UPDATE projects
SET
    git_repo_path = REPLACE(git_repo_path, '/app/', '/home/pythia/topos/apps/'),
    updated_at = datetime('now', 'subsec')
WHERE git_repo_path LIKE '/app/%'
  AND deleted_at IS NULL;
SQL

echo "✓ Paths updated"
echo ""

echo "New project paths:"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
sqlite3 "$DB_PATH" "
SELECT
    SUBSTR(git_repo_path, 1, 30) || '...' as path_prefix,
    COUNT(*) as count
FROM projects
WHERE deleted_at IS NULL
GROUP BY path_prefix
ORDER BY count DESC;
"
echo ""

echo "Checking which paths exist on filesystem:"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

# Check existence of updated paths
sqlite3 "$DB_PATH" "
SELECT name, git_repo_path
FROM projects
WHERE deleted_at IS NULL
ORDER BY name;
" | while IFS='|' read -r name path; do
    if [ -d "$path" ]; then
        echo "✅ $name: $path"
    else
        echo "❌ $name: $path (MISSING - needs file migration)"
    fi
done

echo ""
echo "═══════════════════════════════════════════════════════════════"
echo "✅ PATH MIGRATION COMPLETE"
echo "═══════════════════════════════════════════════════════════════"
echo ""
echo "Next steps:"
echo ""
echo "1. Check which project files are missing (❌ above)"
echo ""
echo "2. Copy missing project files from Space Terminal to Pythia:"
echo "   rsync -av spaceterminal:/home/spaceterminal/topos/ /home/pythia/topos/"
echo "   rsync -av spaceterminal:/Users/bodhi/Documents/ /home/pythia/Documents/"
echo ""
echo "3. Restart ORCHA server:"
echo "   pkill -f server"
echo "   cd ~/pcg-cc-mcp"
echo "   RUST_LOG=info ./target/release/server &"
echo ""
echo "4. Verify projects load correctly in dashboard"
echo ""

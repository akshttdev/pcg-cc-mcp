#!/bin/bash
# Migration Generator — Creates SQLx migrations with high-precision timestamps
#
# Usage: ./scripts/new-migration.sh <migration_name>
#
# Generates: crates/db/migrations/YYYYMMDDHHMMSS_<migration_name>.sql
#
# The HHMMSS precision prevents same-day timestamp collisions that occur
# when multiple developers create migrations on the same day.

set -e

NAME="${1:?Usage: ./scripts/new-migration.sh <migration_name>}"

# Validate name (alphanumeric + underscores only)
if [[ ! "$NAME" =~ ^[a-z][a-z0-9_]*$ ]]; then
    echo "Error: Migration name must be lowercase alphanumeric with underscores, starting with a letter"
    echo "Example: ./scripts/new-migration.sh add_user_preferences"
    exit 1
fi

# Use full timestamp (YYYYMMDDHHMMSS) to prevent same-day collisions
TIMESTAMP=$(date +%Y%m%d%H%M%S)
FILENAME="crates/db/migrations/${TIMESTAMP}_${NAME}.sql"

# Check if file already exists (shouldn't happen with second precision, but safety first)
if [ -f "$FILENAME" ]; then
    echo "Error: Migration file already exists: $FILENAME"
    exit 1
fi

# Get current ISO date for header
ISO_DATE=$(date -Iseconds 2>/dev/null || date +%Y-%m-%dT%H:%M:%S%z)

cat > "$FILENAME" << EOF
-- Migration: ${NAME}
-- Created: ${ISO_DATE}
--
-- Guidelines:
-- - Use IF NOT EXISTS for CREATE TABLE/INDEX
-- - Wrap destructive operations (DROP, DELETE, ALTER) in explicit transactions
-- - For large data migrations, consider batching
-- - Test rollback scenarios in dev before merging
--
-- Note: SQLx runs each migration file in a transaction by default,
-- but explicit BEGIN/COMMIT helps with clarity for complex migrations.

-- Your migration SQL here

EOF

echo "Created: $FILENAME"
echo ""
echo "Next steps:"
echo "  1. Edit the migration file with your SQL"
echo "  2. Run: cargo sqlx prepare --workspace"
echo "  3. Test locally: cargo run -p server"

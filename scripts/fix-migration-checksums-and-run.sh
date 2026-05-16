#!/usr/bin/env bash
# Reseed the dev DB from dev_assets_seed, then realign the _sqlx_migrations
# checksums with the on-disk migration files (handles the case where someone
# edited an old migration after it was applied to the seed). Finally runs
# `sqlx migrate run` to apply any new migrations.
#
# Safe to re-run. Doesn't modify any committed file.
#
# Usage:  bash scripts/fix-migration-checksums-and-run.sh

set -euo pipefail

SEED="dev_assets_seed/db.sqlite"
DEV="dev_assets/db.sqlite"
MIGRATIONS="crates/db/migrations"

if [ ! -f "$SEED" ]; then
  echo "Seed not found at $SEED — run from project root." >&2
  exit 1
fi

echo "→ Resetting $DEV from $SEED"
mkdir -p "$(dirname "$DEV")"
cp "$SEED" "$DEV"

echo "→ Realigning _sqlx_migrations checksums to match disk"
for f in "$MIGRATIONS"/*.sql; do
  v=$(basename "$f" | cut -d_ -f1)
  c=$(sha384sum "$f" | awk '{print $1}')
  sqlite3 "$DEV" "UPDATE _sqlx_migrations SET checksum = X'$c' WHERE version = $v;"
done

echo "→ Running sqlx migrate"
DATABASE_URL="sqlite:$DEV" sqlx migrate run --source "$MIGRATIONS"

echo "✓ Done"

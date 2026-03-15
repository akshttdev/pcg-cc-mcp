#!/bin/bash
# Daily automated backup for dev_assets/db.sqlite
# Keeps the last 15 days of backups.
# Intended to be run via cron — non-interactive, logs to /tmp/pcg-backup.log

set -euo pipefail

DB_PATH="/home/pythia/pcg-cc-mcp/dev_assets/db.sqlite"
BACKUP_DIR="/home/pythia/pcg-cc-mcp/dev_assets/daily_backups"
KEEP_DAYS=15
LOG="/tmp/pcg-backup.log"

mkdir -p "$BACKUP_DIR"

TIMESTAMP=$(date +%Y%m%d_%H%M%S)
BACKUP_FILE="$BACKUP_DIR/db_$TIMESTAMP.sqlite"

echo "[$(date -Iseconds)] Starting daily backup" >> "$LOG"

if [ ! -f "$DB_PATH" ]; then
    echo "[$(date -Iseconds)] ERROR: DB not found at $DB_PATH" >> "$LOG"
    exit 1
fi

# sqlite3 .backup is WAL-safe — creates a consistent snapshot even under concurrent writes
sqlite3 "$DB_PATH" ".backup '$BACKUP_FILE'"

SIZE=$(du -h "$BACKUP_FILE" | cut -f1)
echo "[$(date -Iseconds)] Backup created: $BACKUP_FILE ($SIZE)" >> "$LOG"

# Prune backups older than KEEP_DAYS
PRUNED=$(find "$BACKUP_DIR" -name "db_*.sqlite" -type f -mtime +"$KEEP_DAYS" | wc -l | tr -d ' ')
find "$BACKUP_DIR" -name "db_*.sqlite" -type f -mtime +"$KEEP_DAYS" -delete

if [ "$PRUNED" -gt 0 ]; then
    echo "[$(date -Iseconds)] Pruned $PRUNED backup(s) older than $KEEP_DAYS days" >> "$LOG"
fi

TOTAL=$(find "$BACKUP_DIR" -name "db_*.sqlite" -type f | wc -l | tr -d ' ')
echo "[$(date -Iseconds)] Done. $TOTAL backup(s) retained." >> "$LOG"

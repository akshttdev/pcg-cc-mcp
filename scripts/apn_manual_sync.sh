#!/bin/bash
# APN Manual Sync - One-time sync between devices
# Use this for initial sync before enabling continuous sync

set -e

echo "═══════════════════════════════════════════════════════════════"
echo "ORCHA APN Manual Sync"
echo "═══════════════════════════════════════════════════════════════"
echo ""

# Get source and target from args
SOURCE_DEVICE="${1:-spaceterminal}"
TARGET_DEVICE="${2:-pythia}"
SYNC_MODE="${3:-bidirectional}"

echo "Source: $SOURCE_DEVICE"
echo "Target: $TARGET_DEVICE"
echo "Mode: $SYNC_MODE"
echo ""

# Check if current device matches one of the endpoints
CURRENT_DEVICE=$(hostname)
echo "Current device: $CURRENT_DEVICE"
echo ""

if [ "$CURRENT_DEVICE" != "$SOURCE_DEVICE" ] && [ "$CURRENT_DEVICE" != "$TARGET_DEVICE" ]; then
    echo "⚠️  Warning: Current device ($CURRENT_DEVICE) is not source or target"
    echo "   This script should be run on one of the sync endpoints"
    echo ""
fi

# Check if APN tools are available
if ! command -v apn-send &> /dev/null; then
    echo "⚠️  APN tools not found - using fallback methods"
    echo ""
    USE_FALLBACK=true
else
    USE_FALLBACK=false
fi

# Function to sync database
sync_database() {
    local from_device="$1"
    local to_device="$2"

    echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
    echo "Syncing Database: $from_device → $to_device"
    echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

    if [ "$CURRENT_DEVICE" = "$from_device" ]; then
        # We are the source - send database
        DB_PATH="$HOME/pcg-cc-mcp/dev_assets/db.sqlite"

        if [ ! -f "$DB_PATH" ]; then
            echo "❌ Database not found at $DB_PATH"
            return 1
        fi

        echo "Backing up database..."
        cp "$DB_PATH" "$DB_PATH.sync_backup_$(date +%s)"

        echo "Compressing database..."
        DB_ARCHIVE="/tmp/orcha_db_sync_$(date +%s).tar.gz"
        tar -czf "$DB_ARCHIVE" -C "$HOME/pcg-cc-mcp/dev_assets" db.sqlite

        echo "Sending to $to_device via APN..."

        if [ "$USE_FALLBACK" = true ]; then
            # Fallback: use rsync over ssh
            echo "  Using rsync fallback..."
            rsync -avz --progress "$DB_ARCHIVE" "$to_device:/tmp/" || {
                echo "❌ Rsync failed. Try: scp $DB_ARCHIVE $to_device:/tmp/"
                return 1
            }
        else
            # Use APN protocol
            apn-send --target "$to_device" --file "$DB_ARCHIVE" --type database || {
                echo "❌ APN send failed"
                return 1
            }
        fi

        echo "✓ Database sent to $to_device"
        echo "  Archive: $DB_ARCHIVE"
        rm -f "$DB_ARCHIVE"

    elif [ "$CURRENT_DEVICE" = "$to_device" ]; then
        # We are the target - receive and apply database
        echo "Waiting to receive database from $from_device..."
        echo ""
        echo "On $from_device, run:"
        echo "  cd ~/pcg-cc-mcp"
        echo "  scripts/apn_manual_sync.sh $from_device $to_device"
        echo ""
        echo "Or manually transfer:"
        echo "  rsync -avz $from_device:~/pcg-cc-mcp/dev_assets/db.sqlite ~/pcg-cc-mcp/dev_assets/db.sqlite.incoming"
        echo ""
    fi
}

# Function to sync project files
sync_projects() {
    local from_device="$1"
    local to_device="$2"

    echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
    echo "Syncing Projects: $from_device → $to_device"
    echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

    if [ "$CURRENT_DEVICE" = "$from_device" ]; then
        # We are the source
        echo "Preparing to sync topos directory..."

        if [ ! -d "$HOME/topos" ]; then
            echo "⚠️  Warning: $HOME/topos not found"
            echo "   Creating directory..."
            mkdir -p "$HOME/topos"
        fi

        echo "Creating project archive..."
        PROJECTS_ARCHIVE="/tmp/orcha_projects_sync_$(date +%s).tar.gz"

        # Archive topos directory (excluding large files)
        tar -czf "$PROJECTS_ARCHIVE" \
            --exclude='node_modules' \
            --exclude='target' \
            --exclude='.git/objects' \
            --exclude='dist' \
            --exclude='build' \
            -C "$HOME" topos 2>/dev/null || {
                echo "⚠️  Some files skipped during archive"
            }

        ARCHIVE_SIZE=$(du -h "$PROJECTS_ARCHIVE" | cut -f1)
        echo "Archive created: $ARCHIVE_SIZE"

        echo "Sending projects to $to_device..."

        if [ "$USE_FALLBACK" = true ]; then
            echo "  Using rsync fallback..."
            rsync -avz --progress \
                --exclude='node_modules' \
                --exclude='target' \
                --exclude='.git/objects' \
                "$HOME/topos/" "$to_device:~/topos/" || {
                    echo "❌ Rsync failed"
                    return 1
                }
        else
            apn-send --target "$to_device" --file "$PROJECTS_ARCHIVE" --type projects || {
                echo "❌ APN send failed"
                return 1
            }
        fi

        echo "✓ Projects sent to $to_device"
        rm -f "$PROJECTS_ARCHIVE"

    elif [ "$CURRENT_DEVICE" = "$to_device" ]; then
        echo "Ready to receive projects from $from_device"
        echo ""
        echo "Receiving via rsync..."
        rsync -avz --progress \
            --exclude='node_modules' \
            --exclude='target' \
            "$from_device:~/topos/" "$HOME/topos/" || {
                echo "❌ Rsync failed"
                echo ""
                echo "Manual alternative:"
                echo "  On $from_device: tar -czf /tmp/topos.tar.gz ~/topos"
                echo "  Transfer file to this machine"
                echo "  On this machine: tar -xzf /tmp/topos.tar.gz -C ~/"
                return 1
            }
        echo "✓ Projects received"
    fi
}

# Main sync logic
echo "Starting sync process..."
echo ""

case "$SYNC_MODE" in
    database)
        sync_database "$SOURCE_DEVICE" "$TARGET_DEVICE"
        ;;
    projects)
        sync_projects "$SOURCE_DEVICE" "$TARGET_DEVICE"
        ;;
    bidirectional)
        echo "Phase 1: Database sync"
        sync_database "$SOURCE_DEVICE" "$TARGET_DEVICE"
        echo ""
        echo "Phase 2: Projects sync"
        sync_projects "$SOURCE_DEVICE" "$TARGET_DEVICE"
        echo ""
        if [ "$SOURCE_DEVICE" != "$TARGET_DEVICE" ]; then
            echo "Phase 3: Reverse sync (for bidirectional)"
            echo "  Run on $TARGET_DEVICE:"
            echo "  scripts/apn_manual_sync.sh $TARGET_DEVICE $SOURCE_DEVICE"
        fi
        ;;
    *)
        echo "❌ Invalid sync mode: $SYNC_MODE"
        echo "   Valid modes: database, projects, bidirectional"
        exit 1
        ;;
esac

echo ""
echo "═══════════════════════════════════════════════════════════════"
echo "✅ SYNC COMPLETE"
echo "═══════════════════════════════════════════════════════════════"
echo ""
echo "Next steps:"
echo ""
echo "1. Verify data on both devices"
echo ""
echo "2. Restart ORCHA server on both:"
echo "   pkill -f server"
echo "   cd ~/pcg-cc-mcp"
echo "   RUST_LOG=info ./target/release/server &"
echo ""
echo "3. Test dashboard on both devices"
echo ""
echo "4. Enable continuous sync (optional):"
echo "   scripts/apn_sync_setup.sh"
echo ""

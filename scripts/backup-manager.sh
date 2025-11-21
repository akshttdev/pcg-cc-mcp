#!/bin/bash
# Database Backup Management Script
# Provides manual backup/restore operations for PCG-CC-MCP database

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"
DB_PATH="$PROJECT_DIR/dev_assets/db.sqlite"
BACKUP_DIR="$PROJECT_DIR/backups"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Helper functions
error() {
    echo -e "${RED}❌ Error: $1${NC}" >&2
    exit 1
}

success() {
    echo -e "${GREEN}✅ $1${NC}"
}

info() {
    echo -e "${BLUE}ℹ️  $1${NC}"
}

warning() {
    echo -e "${YELLOW}⚠️  $1${NC}"
}

# Ensure backup directory exists
mkdir -p "$BACKUP_DIR"

# Show usage
usage() {
    cat << EOF
${BLUE}Database Backup Manager${NC}

Usage: $0 <command> [options]

Commands:
    backup              Create a manual backup now
    restore <file>      Restore from a specific backup file
    list                List all available backups
    latest              Show the latest backup
    clean [days]        Remove backups older than N days (default: 30)
    verify              Verify database integrity
    status              Show backup service status
    help                Show this help message

Examples:
    $0 backup
    $0 restore backups/backup_20251119_143000.sqlite
    $0 list
    $0 clean 14
    $0 verify

Environment Variables:
    BACKUP_DIR          Override backup directory location
    DB_PATH             Override database file location

EOF
}

# Create manual backup
backup() {
    info "Creating manual backup..."
    
    if [ ! -f "$DB_PATH" ]; then
        error "Database file not found at: $DB_PATH"
    fi
    
    TIMESTAMP=$(date +%Y%m%d_%H%M%S)
    BACKUP_FILE="$BACKUP_DIR/manual_backup_$TIMESTAMP.sqlite"
    
    # Use SQLite's backup command for consistency
    if command -v sqlite3 &> /dev/null; then
        sqlite3 "$DB_PATH" ".backup '$BACKUP_FILE'"
        success "Backup created: $BACKUP_FILE"
    else
        # Fallback to simple copy
        cp "$DB_PATH" "$BACKUP_FILE"
        warning "SQLite3 not found, used simple file copy"
        success "Backup created: $BACKUP_FILE"
    fi
    
    # Update latest symlink
    ln -sf "$(basename "$BACKUP_FILE")" "$BACKUP_DIR/backup_latest.sqlite"
    
    # Show backup info
    BACKUP_SIZE=$(du -h "$BACKUP_FILE" | cut -f1)
    info "Backup size: $BACKUP_SIZE"
}

# Restore from backup
restore() {
    local BACKUP_FILE="$1"
    
    if [ -z "$BACKUP_FILE" ]; then
        error "Please specify a backup file to restore from"
    fi
    
    # Handle relative paths
    if [ ! -f "$BACKUP_FILE" ]; then
        BACKUP_FILE="$BACKUP_DIR/$BACKUP_FILE"
    fi
    
    if [ ! -f "$BACKUP_FILE" ]; then
        error "Backup file not found: $BACKUP_FILE"
    fi
    
    warning "This will REPLACE the current database!"
    echo -n "Are you sure? Type 'yes' to continue: "
    read -r CONFIRM
    
    if [ "$CONFIRM" != "yes" ]; then
        info "Restore cancelled"
        exit 0
    fi
    
    # Create safety backup of current database
    if [ -f "$DB_PATH" ]; then
        SAFETY_BACKUP="$BACKUP_DIR/pre_restore_$(date +%Y%m%d_%H%M%S).sqlite"
        cp "$DB_PATH" "$SAFETY_BACKUP"
        info "Created safety backup: $SAFETY_BACKUP"
    fi
    
    # Stop the app (if using docker-compose)
    if command -v docker-compose &> /dev/null && [ -f "$PROJECT_DIR/docker-compose.yml" ]; then
        info "Stopping application..."
        (cd "$PROJECT_DIR" && docker-compose stop app)
    fi
    
    # Restore the backup
    cp "$BACKUP_FILE" "$DB_PATH"
    success "Database restored from: $BACKUP_FILE"
    
    # Restart the app
    if command -v docker-compose &> /dev/null && [ -f "$PROJECT_DIR/docker-compose.yml" ]; then
        info "Restarting application..."
        (cd "$PROJECT_DIR" && docker-compose start app)
    fi
    
    success "Restore complete!"
}

# List all backups
list_backups() {
    info "Available backups in: $BACKUP_DIR"
    echo ""
    
    if [ ! -d "$BACKUP_DIR" ] || [ -z "$(ls -A "$BACKUP_DIR"/*.sqlite 2>/dev/null)" ]; then
        warning "No backups found"
        exit 0
    fi
    
    echo "Name                                Size      Date"
    echo "────────────────────────────────────────────────────────────"
    
    ls -lt "$BACKUP_DIR"/*.sqlite 2>/dev/null | grep -v "backup_latest.sqlite" | while read -r line; do
        SIZE=$(echo "$line" | awk '{print $5}')
        DATE=$(echo "$line" | awk '{print $6, $7, $8}')
        FILE=$(basename "$(echo "$line" | awk '{print $9}')")
        
        # Convert size to human readable
        if [ "$SIZE" -gt 1048576 ]; then
            SIZE_HR="$(awk "BEGIN {printf \"%.1fMB\", $SIZE/1048576}")"
        elif [ "$SIZE" -gt 1024 ]; then
            SIZE_HR="$(awk "BEGIN {printf \"%.1fKB\", $SIZE/1024}")"
        else
            SIZE_HR="${SIZE}B"
        fi
        
        printf "%-35s %-9s %s\n" "$FILE" "$SIZE_HR" "$DATE"
    done
    
    echo ""
    TOTAL=$(find "$BACKUP_DIR" -name "*.sqlite" -type f | grep -v "backup_latest.sqlite" | wc -l | tr -d ' ')
    info "Total backups: $TOTAL"
}

# Show latest backup
show_latest() {
    if [ -L "$BACKUP_DIR/backup_latest.sqlite" ]; then
        LATEST=$(readlink "$BACKUP_DIR/backup_latest.sqlite")
        LATEST_PATH="$BACKUP_DIR/$LATEST"
        
        if [ -f "$LATEST_PATH" ]; then
            SIZE=$(du -h "$LATEST_PATH" | cut -f1)
            DATE=$(stat -f "%Sm" -t "%Y-%m-%d %H:%M:%S" "$LATEST_PATH" 2>/dev/null || stat -c "%y" "$LATEST_PATH" 2>/dev/null)
            
            success "Latest backup:"
            echo "  File: $LATEST"
            echo "  Size: $SIZE"
            echo "  Date: $DATE"
            echo "  Path: $LATEST_PATH"
        else
            warning "Latest backup symlink exists but points to missing file"
        fi
    else
        warning "No latest backup found"
    fi
}

# Clean old backups
clean_backups() {
    local DAYS=${1:-30}
    
    info "Removing backups older than $DAYS days..."
    
    FOUND=$(find "$BACKUP_DIR" -name "backup_*.sqlite" -type f -mtime +"$DAYS" | wc -l | tr -d ' ')
    
    if [ "$FOUND" -eq 0 ]; then
        info "No backups older than $DAYS days found"
        exit 0
    fi
    
    warning "Found $FOUND backup(s) to delete"
    echo -n "Continue? (y/N): "
    read -r CONFIRM
    
    if [ "$CONFIRM" = "y" ] || [ "$CONFIRM" = "Y" ]; then
        find "$BACKUP_DIR" -name "backup_*.sqlite" -type f -mtime +"$DAYS" -delete
        success "Removed $FOUND old backup(s)"
    else
        info "Cleanup cancelled"
    fi
}

# Verify database integrity
verify_database() {
    if [ ! -f "$DB_PATH" ]; then
        error "Database file not found at: $DB_PATH"
    fi
    
    if ! command -v sqlite3 &> /dev/null; then
        error "sqlite3 command not found. Please install SQLite3."
    fi
    
    info "Verifying database integrity..."
    
    # Run integrity check
    RESULT=$(sqlite3 "$DB_PATH" "PRAGMA integrity_check;" 2>&1)
    
    if [ "$RESULT" = "ok" ]; then
        success "Database integrity check passed"
        
        # Show some stats
        info "Database statistics:"
        echo "  Size: $(du -h "$DB_PATH" | cut -f1)"
        
        TABLE_COUNT=$(sqlite3 "$DB_PATH" "SELECT COUNT(*) FROM sqlite_master WHERE type='table';" 2>/dev/null)
        echo "  Tables: $TABLE_COUNT"
        
        PAGE_COUNT=$(sqlite3 "$DB_PATH" "PRAGMA page_count;" 2>/dev/null)
        echo "  Pages: $PAGE_COUNT"
    else
        error "Database integrity check FAILED:\n$RESULT"
    fi
}

# Show backup service status
show_status() {
    info "Backup Service Status"
    echo ""
    
    # Check if docker-compose is running
    if command -v docker-compose &> /dev/null && [ -f "$PROJECT_DIR/docker-compose.yml" ]; then
        (cd "$PROJECT_DIR" && docker-compose ps db-backup)
        echo ""
        
        # Show last few log lines
        info "Recent backup service logs:"
        (cd "$PROJECT_DIR" && docker-compose logs --tail=10 db-backup)
    else
        warning "Docker Compose not found or not configured"
    fi
    
    # Show backup directory info
    echo ""
    info "Backup directory: $BACKUP_DIR"
    
    if [ -d "$BACKUP_DIR" ]; then
        TOTAL_SIZE=$(du -sh "$BACKUP_DIR" 2>/dev/null | cut -f1)
        BACKUP_COUNT=$(find "$BACKUP_DIR" -name "*.sqlite" -type f | wc -l | tr -d ' ')
        
        echo "  Total size: $TOTAL_SIZE"
        echo "  Total backups: $BACKUP_COUNT"
    else
        warning "Backup directory does not exist"
    fi
}

# Main command dispatcher
case "${1:-}" in
    backup)
        backup
        ;;
    restore)
        restore "$2"
        ;;
    list)
        list_backups
        ;;
    latest)
        show_latest
        ;;
    clean)
        clean_backups "$2"
        ;;
    verify)
        verify_database
        ;;
    status)
        show_status
        ;;
    help|--help|-h)
        usage
        ;;
    *)
        usage
        exit 1
        ;;
esac

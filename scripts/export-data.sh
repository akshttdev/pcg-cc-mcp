#!/bin/bash

# PCG-CC-MCP Data Export Script
# Exports database and repositories from current instance for migration

set -e  # Exit on error

# Add common binary paths
export PATH="/bin:/usr/bin:/usr/local/bin:$PATH"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Configuration
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
DB_PATH="$PROJECT_ROOT/dev_assets/db.sqlite"
EXPORT_DIR="$PROJECT_ROOT/pcg_export_$(date +%Y%m%d_%H%M%S)"

echo -e "${BLUE}╔════════════════════════════════════════════════════════╗${NC}"
echo -e "${BLUE}║   PCG-CC-MCP Data Export Tool                         ║${NC}"
echo -e "${BLUE}╚════════════════════════════════════════════════════════╝${NC}"
echo ""

# Function to print step
print_step() {
    echo -e "${GREEN}➜${NC} $1"
}

# Function to print warning
print_warning() {
    echo -e "${YELLOW}⚠${NC} $1"
}

# Function to print error
print_error() {
    echo -e "${RED}✗${NC} $1"
}

# Function to print success
print_success() {
    echo -e "${GREEN}✓${NC} $1"
}

# Function to get human-readable size
get_size() {
    if [[ "$OSTYPE" == "darwin"* ]]; then
        # macOS - use du without cut
        local size_output=$(du -sh "$1" 2>/dev/null)
        echo "${size_output%%$'\t'*}"  # Remove everything after tab
    else
        # Linux
        local size_output=$(du -sh "$1" 2>/dev/null)
        echo "${size_output%%$'\t'*}"  # Remove everything after tab
    fi
}

# Check if database exists
if [ ! -f "$DB_PATH" ]; then
    print_error "Database not found: $DB_PATH"
    exit 1
fi

# Create export directory
mkdir -p "$EXPORT_DIR"
print_step "Created export directory: $EXPORT_DIR"

# Step 1: Show current database stats
echo ""
echo -e "${BLUE}════════════════════════════════════════════════════════${NC}"
echo -e "${BLUE}Database Statistics${NC}"
echo -e "${BLUE}════════════════════════════════════════════════════════${NC}"
echo ""

PROJECT_COUNT=$(sqlite3 "$DB_PATH" "SELECT COUNT(*) FROM projects;")
TASK_COUNT=$(sqlite3 "$DB_PATH" "SELECT COUNT(*) FROM tasks;")
ATTEMPT_COUNT=$(sqlite3 "$DB_PATH" "SELECT COUNT(*) FROM task_attempts;")
DB_SIZE=$(get_size "$DB_PATH")

echo "Projects: $PROJECT_COUNT"
echo "Tasks: $TASK_COUNT"
echo "Task Attempts: $ATTEMPT_COUNT"
echo "Database Size: $DB_SIZE"
echo ""

print_step "Projects to export:"
echo ""
sqlite3 -header -column "$DB_PATH" "SELECT name, git_repo_path FROM projects;"
echo ""

# Step 2: Export database
echo -e "${BLUE}════════════════════════════════════════════════════════${NC}"
echo -e "${BLUE}Step 1: Export Database${NC}"
echo -e "${BLUE}════════════════════════════════════════════════════════${NC}"
echo ""

print_step "Exporting database to SQL dump..."
sqlite3 "$DB_PATH" ".dump" > "$EXPORT_DIR/database_backup.sql"
print_success "Database exported to database_backup.sql"

print_step "Copying binary database file..."
cp "$DB_PATH" "$EXPORT_DIR/db.sqlite"
print_success "Database copied to db.sqlite"

# Step 3: Export repositories
echo ""
echo -e "${BLUE}════════════════════════════════════════════════════════${NC}"
echo -e "${BLUE}Step 2: Export Repositories${NC}"
echo -e "${BLUE}════════════════════════════════════════════════════════${NC}"
echo ""

REPOS_DIR="$EXPORT_DIR/repositories"
mkdir -p "$REPOS_DIR"

TOTAL_REPOS=0
EXPORTED_REPOS=0
MISSING_REPOS=0
TOTAL_SIZE=0

# Get repository paths from database
while IFS='|' read -r NAME PATH; do
    ((TOTAL_REPOS++))
    
    if [ -d "$PATH" ]; then
        print_step "Exporting: $NAME"
        REPO_SIZE=$(get_size "$PATH")
        echo "  Path: $PATH"
        echo "  Size: $REPO_SIZE"
        
        # Copy repository
        REPO_NAME="${PATH##*/}"  # Extract basename without external command
        
        # Use absolute path for cp command
        if command -v /bin/cp >/dev/null 2>&1; then
            /bin/cp -R "$PATH" "$REPOS_DIR/$REPO_NAME" 2>&1 || {
                print_error "Failed to copy repository"
                echo "  Trying alternative method..."
                /usr/bin/ditto "$PATH" "$REPOS_DIR/$REPO_NAME" 2>&1
            }
        else
            print_error "cp command not found in /bin/cp"
            exit 1
        fi
        
        # Create metadata file
        cat > "$REPOS_DIR/$REPO_NAME/.pcg_metadata.json" <<EOF
{
  "original_name": "$NAME",
  "original_path": "$PATH",
  "exported_at": "$(date -u +"%Y-%m-%dT%H:%M:%SZ")",
  "hostname": "$(hostname)",
  "user": "$(whoami)"
}
EOF
        
        print_success "Exported: $REPO_NAME ($REPO_SIZE)"
        ((EXPORTED_REPOS++))
        echo ""
    else
        print_warning "Repository not found: $NAME"
        echo "  Expected path: $PATH"
        ((MISSING_REPOS++))
        echo ""
    fi
done < <(sqlite3 "$DB_PATH" "SELECT name, git_repo_path FROM projects;")

# Step 4: Export config files
echo -e "${BLUE}════════════════════════════════════════════════════════${NC}"
echo -e "${BLUE}Step 3: Export Configuration${NC}"
echo -e "${BLUE}════════════════════════════════════════════════════════${NC}"
echo ""

CONFIG_DIR="$EXPORT_DIR/config"
mkdir -p "$CONFIG_DIR"

# Export dev_assets config
if [ -f "$PROJECT_ROOT/dev_assets/config.json" ]; then
    cp "$PROJECT_ROOT/dev_assets/config.json" "$CONFIG_DIR/config.json"
    print_success "Exported config.json"
fi

# Export .env if exists (with warning)
if [ -f "$PROJECT_ROOT/.env" ]; then
    print_warning "Found .env file - copying (contains sensitive data!)"
    cp "$PROJECT_ROOT/.env" "$CONFIG_DIR/.env"
    print_success "Exported .env (REVIEW BEFORE SHARING!)"
fi

# Create migration guide
cat > "$EXPORT_DIR/MIGRATION_GUIDE.md" <<'EOF'
# PCG-CC-MCP Migration Guide

## Files Exported

- `database_backup.sql` - SQL dump of the database
- `db.sqlite` - Binary database file (backup)
- `repositories/` - All project repositories
- `config/` - Configuration files

## Migration Steps

### 1. Transfer Files

Copy the entire export directory to your new device:

```bash
# Example: Using rsync
rsync -avz pcg_export_* user@new-device:/path/to/new/pcg-cc-mcp/

# Or compress first
tar -czf pcg_export.tar.gz pcg_export_*
# Transfer pcg_export.tar.gz and extract on new device
```

### 2. Run Import Script

On the new device:

```bash
cd /path/to/new/pcg-cc-mcp
./scripts/migrate-data.sh
```

Follow the interactive prompts to:
- Import the database
- Update repository paths
- Copy repositories to new locations

### 3. Verify

After migration, verify all projects appear correctly:

```bash
pnpm run dev
# Navigate to http://localhost:3000
```

## Important Notes

- **Environment Variables**: Review `config/.env` and update paths/tokens for new device
- **Git Remotes**: Repository git remotes are preserved
- **Absolute Paths**: Database paths will be updated during import
- **Backup**: Keep this export until migration is verified successful

## Troubleshooting

### Missing Repositories
If any repos were missing during export, you'll need to copy them manually or re-clone from git.

### Permission Issues
Ensure you have read/write permissions for all repository directories.

### Database Errors
If import fails, use the binary `db.sqlite` file directly:
```bash
cp db.sqlite /path/to/new/pcg-cc-mcp/dev_assets/db.sqlite
```

## Support

For issues, check the PCG-CC-MCP documentation or repository issues.
EOF

print_success "Created MIGRATION_GUIDE.md"
echo ""

# Step 5: Create manifest
cat > "$EXPORT_DIR/manifest.json" <<EOF
{
  "export_date": "$(date -u +"%Y-%m-%dT%H:%M:%SZ")",
  "source_hostname": "$(hostname)",
  "source_user": "$(whoami)",
  "source_path": "$PROJECT_ROOT",
  "database": {
    "projects": $PROJECT_COUNT,
    "tasks": $TASK_COUNT,
    "task_attempts": $ATTEMPT_COUNT,
    "size": "$DB_SIZE"
  },
  "repositories": {
    "total": $TOTAL_REPOS,
    "exported": $EXPORTED_REPOS,
    "missing": $MISSING_REPOS
  }
}
EOF

print_success "Created manifest.json"

# Step 6: Create compressed archive
echo ""
echo -e "${BLUE}════════════════════════════════════════════════════════${NC}"
echo -e "${BLUE}Step 4: Create Archive${NC}"
echo -e "${BLUE}════════════════════════════════════════════════════════${NC}"
echo ""

ARCHIVE_NAME="${EXPORT_DIR##*/}.tar.gz"  # Extract basename without external command

print_step "Compressing export to $ARCHIVE_NAME..."
tar -czf "$EXPORT_DIR.tar.gz" -C "${EXPORT_DIR%/*}" "${EXPORT_DIR##*/}"  # Use parameter expansion

ARCHIVE_SIZE=$(get_size "$EXPORT_DIR.tar.gz")
print_success "Archive created: $ARCHIVE_NAME ($ARCHIVE_SIZE)"

# Step 7: Summary
echo ""
echo -e "${BLUE}════════════════════════════════════════════════════════${NC}"
echo -e "${BLUE}Export Summary${NC}"
echo -e "${BLUE}════════════════════════════════════════════════════════${NC}"
echo ""

EXPORT_SIZE=$(get_size "$EXPORT_DIR")

echo "Export Directory: $EXPORT_DIR"
echo "Export Size: $EXPORT_SIZE"
echo "Archive: $EXPORT_DIR.tar.gz ($ARCHIVE_SIZE)"
echo ""
echo "Database:"
echo "  - Projects: $PROJECT_COUNT"
echo "  - Tasks: $TASK_COUNT"
echo "  - Attempts: $ATTEMPT_COUNT"
echo ""
echo "Repositories:"
echo "  - Total: $TOTAL_REPOS"
echo "  - Exported: $EXPORTED_REPOS"
echo "  - Missing: $MISSING_REPOS"
echo ""

if [ $MISSING_REPOS -gt 0 ]; then
    print_warning "$MISSING_REPOS repositories were not found and not exported!"
fi

echo ""
print_success "Export complete!"
echo ""
echo "Next steps:"
echo "  1. Transfer $ARCHIVE_NAME to new device"
echo "  2. Extract: tar -xzf $ARCHIVE_NAME"
echo "  3. Run migration script: ./scripts/migrate-data.sh"
echo ""
echo "See $EXPORT_DIR/MIGRATION_GUIDE.md for detailed instructions"
echo ""

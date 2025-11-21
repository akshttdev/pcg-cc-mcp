#!/bin/bash

# PCG-CC-MCP Data Migration Script
# Migrates database and repositories from old instance to new instance

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
BACKUP_DIR="$PROJECT_ROOT/migration_backup_$(date +%Y%m%d_%H%M%S)"

echo -e "${BLUE}╔════════════════════════════════════════════════════════╗${NC}"
echo -e "${BLUE}║   PCG-CC-MCP Data Migration Tool                      ║${NC}"
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

# Check if running from correct directory
if [ ! -f "$PROJECT_ROOT/package.json" ]; then
    print_error "Not in PCG-CC-MCP project root!"
    exit 1
fi

# Create backup directory
mkdir -p "$BACKUP_DIR"
print_step "Created backup directory: $BACKUP_DIR"

# Step 1: Backup current database (if exists)
if [ -f "$DB_PATH" ]; then
    print_warning "Found existing database, backing up..."
    sqlite3 "$DB_PATH" ".dump" > "$BACKUP_DIR/current_db_backup.sql"
    cp "$DB_PATH" "$BACKUP_DIR/db.sqlite.backup"
    print_success "Current database backed up"
fi

# Step 2: Import old database
echo ""
echo -e "${BLUE}════════════════════════════════════════════════════════${NC}"
echo -e "${BLUE}Step 1: Import Old Database${NC}"
echo -e "${BLUE}════════════════════════════════════════════════════════${NC}"
echo ""

read -p "Enter path to export directory or SQL backup file: " IMPORT_SOURCE

# Check if it's a directory (from export script) or SQL file
if [ -d "$IMPORT_SOURCE" ]; then
    # It's an export directory
    EXPORT_DIR="$IMPORT_SOURCE"
    OLD_DB_BACKUP="$EXPORT_DIR/database_backup.sql"
    REPOS_DIR="$EXPORT_DIR/repositories"
    
    if [ ! -f "$OLD_DB_BACKUP" ]; then
        print_error "Database backup not found in export directory: $OLD_DB_BACKUP"
        exit 1
    fi
    
    print_step "Using export directory: $EXPORT_DIR"
    
    # Show manifest if exists
    if [ -f "$EXPORT_DIR/manifest.json" ]; then
        print_step "Export manifest:"
        cat "$EXPORT_DIR/manifest.json" | python3 -m json.tool 2>/dev/null || cat "$EXPORT_DIR/manifest.json"
        echo ""
    fi
elif [ -f "$IMPORT_SOURCE" ]; then
    # It's a SQL file
    OLD_DB_BACKUP="$IMPORT_SOURCE"
    REPOS_DIR=""
    print_step "Using SQL backup: $OLD_DB_BACKUP"
else
    print_error "Not found: $IMPORT_SOURCE"
    exit 1
fi

print_step "Importing database from $OLD_DB_BACKUP..."
rm -f "$DB_PATH"
sqlite3 "$DB_PATH" < "$OLD_DB_BACKUP"
print_success "Database imported successfully"

# Step 3: Show current projects
echo ""
echo -e "${BLUE}════════════════════════════════════════════════════════${NC}"
echo -e "${BLUE}Step 2: Review Current Projects${NC}"
echo -e "${BLUE}════════════════════════════════════════════════════════${NC}"
echo ""

print_step "Current projects in database:"
echo ""
sqlite3 -header -column "$DB_PATH" "SELECT name, git_repo_path FROM projects;"
echo ""

# Step 4: Update repository paths
echo -e "${BLUE}════════════════════════════════════════════════════════${NC}"
echo -e "${BLUE}Step 3: Update Repository Paths${NC}"
echo -e "${BLUE}════════════════════════════════════════════════════════${NC}"
echo ""

read -p "Do you need to update repository paths? (y/n): " UPDATE_PATHS

if [ "$UPDATE_PATHS" = "y" ] || [ "$UPDATE_PATHS" = "Y" ]; then
    read -p "Enter OLD base path (e.g., /Users/bodhi/Documents/GitHub): " OLD_BASE_PATH
    read -p "Enter NEW base path (e.g., /Users/madhav/development/GitHub): " NEW_BASE_PATH
    
    print_step "Updating paths from '$OLD_BASE_PATH' to '$NEW_BASE_PATH'..."
    
    sqlite3 "$DB_PATH" <<EOF
UPDATE projects 
SET git_repo_path = REPLACE(git_repo_path, '$OLD_BASE_PATH', '$NEW_BASE_PATH');
EOF
    
    print_success "Paths updated successfully"
    echo ""
    print_step "Updated projects:"
    echo ""
    sqlite3 -header -column "$DB_PATH" "SELECT name, git_repo_path FROM projects;"
    echo ""
fi

# Step 5: Copy repositories
echo -e "${BLUE}════════════════════════════════════════════════════════${NC}"
echo -e "${BLUE}Step 4: Copy Repository Files${NC}"
echo -e "${BLUE}════════════════════════════════════════════════════════${NC}"
echo ""

if [ -n "$REPOS_DIR" ] && [ -d "$REPOS_DIR" ]; then
    print_step "Found exported repositories in: $REPOS_DIR"
    REPO_COUNT=$(find "$REPOS_DIR" -mindepth 1 -maxdepth 1 -type d | wc -l)
    print_step "Found $REPO_COUNT exported repositories"
    echo ""
    
    read -p "Copy repositories from export directory? (y/n): " COPY_FROM_EXPORT
    
    if [ "$COPY_FROM_EXPORT" = "y" ] || [ "$COPY_FROM_EXPORT" = "Y" ]; then
        # Get list of repositories from database
        while IFS='|' read -r NAME PATH; do
            REPO_NAME="${PATH##*/}"  # Extract basename without external command
            TARGET_DIR="${PATH%/*}"  # Extract dirname without external command
            EXPORT_REPO="$REPOS_DIR/$REPO_NAME"
            
            if [ -d "$EXPORT_REPO" ]; then
                print_step "Copying $REPO_NAME to $PATH..."
                mkdir -p "$TARGET_DIR"
                cp -r "$EXPORT_REPO" "$PATH"
                
                # Remove metadata file if exists
                rm -f "$PATH/.pcg_metadata.json"
                
                print_success "$REPO_NAME copied"
            else
                print_warning "Repository not found in export: $REPO_NAME"
            fi
        done < <(sqlite3 "$DB_PATH" "SELECT name, git_repo_path FROM projects;")
    fi
else
    read -p "Do you need to copy repository files from old location? (y/n): " COPY_REPOS
    
    if [ "$COPY_REPOS" = "y" ] || [ "$COPY_REPOS" = "Y" ]; then
        read -p "Enter path to old repositories directory: " OLD_REPOS_DIR
        
        if [ ! -d "$OLD_REPOS_DIR" ]; then
            print_warning "Directory not found: $OLD_REPOS_DIR"
            print_warning "Skipping repository copy"
        else
            # Get list of repositories from database
            REPOS=$(sqlite3 "$DB_PATH" "SELECT git_repo_path FROM projects;")
            
            for REPO_PATH in $REPOS; do
                REPO_NAME="${REPO_PATH##*/}"  # Extract basename without external command
                TARGET_DIR="${REPO_PATH%/*}"  # Extract dirname without external command
                OLD_REPO="$OLD_REPOS_DIR/$REPO_NAME"
                
                if [ -d "$OLD_REPO" ]; then
                    print_step "Copying $REPO_NAME..."
                    mkdir -p "$TARGET_DIR"
                    cp -r "$OLD_REPO" "$TARGET_DIR/"
                    print_success "$REPO_NAME copied to $TARGET_DIR/"
                else
                    print_warning "Repository not found: $OLD_REPO"
                fi
            done
        fi
    fi
fi

# Step 6: Verify repositories exist
echo ""
echo -e "${BLUE}════════════════════════════════════════════════════════${NC}"
echo -e "${BLUE}Step 5: Verify Repositories${NC}"
echo -e "${BLUE}════════════════════════════════════════════════════════${NC}"
echo ""

print_step "Verifying all repositories exist..."
echo ""

MISSING=0
while IFS='|' read -r NAME PATH; do
    if [ -d "$PATH" ]; then
        print_success "$NAME: $PATH ✓"
    else
        print_error "$NAME: $PATH ✗ (NOT FOUND)"
        ((MISSING++))
    fi
done < <(sqlite3 "$DB_PATH" "SELECT name, git_repo_path FROM projects;" | tr '|' '|')

echo ""

if [ $MISSING -gt 0 ]; then
    print_warning "$MISSING repositories not found!"
    echo ""
    echo "You can manually update paths with:"
    echo "  sqlite3 $DB_PATH"
    echo "  UPDATE projects SET git_repo_path = '/new/path' WHERE name = 'project-name';"
    echo ""
fi

# Step 7: Summary
echo -e "${BLUE}════════════════════════════════════════════════════════${NC}"
echo -e "${BLUE}Migration Summary${NC}"
echo -e "${BLUE}════════════════════════════════════════════════════════${NC}"
echo ""

PROJECT_COUNT=$(sqlite3 "$DB_PATH" "SELECT COUNT(*) FROM projects;")
TASK_COUNT=$(sqlite3 "$DB_PATH" "SELECT COUNT(*) FROM tasks;")
ATTEMPT_COUNT=$(sqlite3 "$DB_PATH" "SELECT COUNT(*) FROM task_attempts;")

echo "Database: $DB_PATH"
echo "Projects: $PROJECT_COUNT"
echo "Tasks: $TASK_COUNT"
echo "Task Attempts: $ATTEMPT_COUNT"
echo ""
echo "Backup saved to: $BACKUP_DIR"
echo ""

if [ $MISSING -eq 0 ]; then
    print_success "All repositories verified!"
    echo ""
    echo -e "${GREEN}Migration complete! You can now start the dashboard:${NC}"
    echo "  pnpm run dev"
else
    print_warning "Some repositories are missing. Please verify paths before starting."
fi

echo ""

#!/bin/bash
# Initialize Per-User Topsi Databases for ORCHA
# Creates sovereign database instances for each user

set -e

echo "======================================================================"
echo "ORCHA Per-User Topsi Database Initialization"
echo "======================================================================"
echo ""

# Database locations from orcha_config.toml
ADMIN_DB="/home/pythia/.local/share/pcg/data/admin/topsi.db"
SIRAK_DB="/home/sirak/topos/.topsi/db.sqlite"
BONOMOTION_DB="/home/bonomotion/.local/share/pcg/data/bonomotion/topsi.db"

# Migration path
MIGRATIONS_DIR="./crates/db/migrations"

init_database() {
    local db_path="$1"
    local owner="$2"
    local description="$3"

    echo "📁 Initializing Topsi database for: $owner"
    echo "   Path: $db_path"
    echo "   Description: $description"

    # Create directory if it doesn't exist
    local db_dir=$(dirname "$db_path")
    if [ ! -d "$db_dir" ]; then
        echo "   Creating directory: $db_dir"
        mkdir -p "$db_dir"
    fi

    # Check if database already exists
    if [ -f "$db_path" ]; then
        echo "   ⚠️  Database already exists, skipping creation"
        return
    fi

    # Create database and run migrations
    echo "   Running migrations..."

    # Use sqlx-cli if available, otherwise use sqlite3
    if command -v sqlx &> /dev/null; then
        DATABASE_URL="sqlite://$db_path" sqlx database create
        DATABASE_URL="sqlite://$db_path" sqlx migrate run --source "$MIGRATIONS_DIR"
    else
        # Create empty database
        sqlite3 "$db_path" "SELECT 1;"

        # Apply migrations manually
        for migration in "$MIGRATIONS_DIR"/*.sql; do
            if [ -f "$migration" ]; then
                echo "   Applying: $(basename $migration)"
                sqlite3 "$db_path" < "$migration" || echo "   ⚠️  Migration may have failed, continuing..."
            fi
        done
    fi

    echo "   ✅ Database initialized"
    echo ""
}

# Initialize admin's Topsi on Pythia Master Node
init_database "$ADMIN_DB" "admin" "Primary master node with multi-device orchestration"

# Note: Sirak and Bonomotion databases would be initialized on their respective devices
echo "📝 Note: User databases for Sirak and Bonomotion should be initialized on their devices:"
echo "   - Sirak: $SIRAK_DB (on sirak-studios-laptop-001)"
echo "   - Bonomotion: $BONOMOTION_DB (on bonomotion-device-001)"
echo ""

# Create projects directories if they don't exist
echo "📂 Creating projects directories..."

ADMIN_PROJECTS="/home/pythia/topos"
if [ ! -d "$ADMIN_PROJECTS" ]; then
    echo "   Creating: $ADMIN_PROJECTS"
    mkdir -p "$ADMIN_PROJECTS"
fi
echo "   ✅ Admin projects directory ready: $ADMIN_PROJECTS"

echo ""
echo "======================================================================"
echo "✅ Admin Topsi Database Initialization Complete!"
echo "======================================================================"
echo ""
echo "Next steps:"
echo "  1. Start ORCHA with: ORCHA_CONFIG=orcha_config.toml cargo run --bin server"
echo "  2. Initialize Sirak and Bonomotion databases on their devices"
echo "  3. Configure APN networking for device-to-device communication"
echo "  4. Test federated routing by logging in as different users"
echo ""
echo "Database locations:"
echo "  - Admin:      $ADMIN_DB"
echo "  - Sirak:      $SIRAK_DB (remote)"
echo "  - Bonomotion: $BONOMOTION_DB (remote)"
echo ""

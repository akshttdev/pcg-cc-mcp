#!/bin/sh
set -e

# Database initialization script
# Only copies seed database if no database exists

DB_PATH="/app/dev_assets/db.sqlite"
SEED_PATH="/app/dev_assets_seed/db.sqlite"
BACKUP_LATEST="/app/backups/backup_latest.sqlite"

echo "🚀 Starting PCG-CC-MCP..."

# Start Ollama service in background
echo "🤖 Starting Ollama service..."
ollama serve > /tmp/ollama.log 2>&1 &
OLLAMA_PID=$!
echo "✅ Ollama started (PID: $OLLAMA_PID)"

# Wait for Ollama to be ready
echo "⏳ Waiting for Ollama to be ready..."
for i in $(seq 1 30); do
    if curl -s http://localhost:11434/api/tags >/dev/null 2>&1; then
        echo "✅ Ollama is ready"
        break
    fi
    sleep 1
done

# Start Chatterbox TTS server in background
echo "🎤 Starting Chatterbox TTS service..."
python3 /app/scripts/chatterbox_server.py > /tmp/chatterbox.log 2>&1 &
CHATTERBOX_PID=$!
echo "✅ Chatterbox started (PID: $CHATTERBOX_PID)"

# Ensure directories exist
mkdir -p /app/dev_assets

# Check if database already exists
if [ -f "$DB_PATH" ]; then
    echo "✅ Existing database found - preserving data"
    # Verify database is valid
    if command -v sqlite3 >/dev/null 2>&1; then
        if sqlite3 "$DB_PATH" "SELECT 1;" >/dev/null 2>&1; then
            echo "✅ Database integrity OK"
        else
            echo "⚠️  Database appears corrupted"
            if [ -f "$BACKUP_LATEST" ]; then
                echo "🔄 Restoring from latest backup..."
                cp "$BACKUP_LATEST" "$DB_PATH"
                echo "✅ Restored from backup"
            fi
        fi
    fi
else
    echo "📦 No database found - initializing..."
    
    # Priority: Latest backup > Seed database
    if [ -f "$BACKUP_LATEST" ]; then
        echo "🔄 Restoring from latest backup: $BACKUP_LATEST"
        cp "$BACKUP_LATEST" "$DB_PATH"
        echo "✅ Database restored from backup"
    elif [ -f "$SEED_PATH" ]; then
        echo "🌱 Copying seed database..."
        cp "$SEED_PATH" "$DB_PATH"
        echo "✅ Database initialized from seed"
    else
        echo "⚠️  No seed database found - app will create new database"
    fi
fi

# Show database info
if [ -f "$DB_PATH" ]; then
    DB_SIZE=$(du -h "$DB_PATH" | cut -f1)
    echo "📊 Database size: $DB_SIZE"
fi

echo "🎯 Starting server..."
exec "$@"

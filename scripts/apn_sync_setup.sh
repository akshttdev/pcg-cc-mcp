#!/bin/bash
# APN Data Sync Setup - Space Terminal ↔ Pythia Master Node
# Enables bidirectional sync of projects and database via APN

set -e

DEVICE_NAME="${DEVICE_NAME:-$(hostname)}"
DB_PATH="$HOME/pcg-cc-mcp/dev_assets/db.sqlite"

echo "═══════════════════════════════════════════════════════════════"
echo "APN Data Sync Setup - Multi-Device Dashboard"
echo "═══════════════════════════════════════════════════════════════"
echo ""
echo "Device: $DEVICE_NAME"
echo ""

# Detect if this is Pythia or Space Terminal
if [ "$DEVICE_NAME" = "pythia" ] || [ "$DEVICE_NAME" = "pop-os" ]; then
    DEVICE_ROLE="master_node"
    DEVICE_ID="pythia"
    echo "Role: Master Node (Primary)"
elif [ "$DEVICE_NAME" = "spaceterminal" ] || [ -d "/Users/bodhi" ]; then
    DEVICE_ROLE="relay"
    DEVICE_ID="spaceterminal"
    echo "Role: Relay Device (Secondary)"
else
    DEVICE_ROLE="client"
    DEVICE_ID="$DEVICE_NAME"
    echo "Role: Client Device"
fi
echo ""

# Check if database exists
if [ ! -f "$DB_PATH" ]; then
    echo "❌ Database not found at $DB_PATH"
    echo "   Run initial setup first"
    exit 1
fi

echo "✓ Database found"
echo ""

# Register this device in the database
echo "Registering device in ORCHA network..."
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

# Get Admin user ID (default owner for system devices)
ADMIN_UUID=$(sqlite3 "$DB_PATH" "SELECT hex(id) FROM users WHERE username = 'admin' LIMIT 1;")

if [ -z "$ADMIN_UUID" ]; then
    echo "❌ Admin user not found in database"
    exit 1
fi

# Detect system specs
TOTAL_CORES=$(nproc 2>/dev/null || sysctl -n hw.ncpu 2>/dev/null || echo "4")
TOTAL_RAM_GB=$(free -g 2>/dev/null | awk '/^Mem:/{print $2}' || sysctl -n hw.memsize 2>/dev/null | awk '{print int($1/1024/1024/1024)}' || echo "8")
GPU_AVAILABLE=0
GPU_MODEL="None"

# Check for GPU
if command -v nvidia-smi &> /dev/null; then
    GPU_AVAILABLE=1
    GPU_MODEL=$(nvidia-smi --query-gpu=name --format=csv,noheader | head -1)
elif [ -d "/System/Library/Extensions/AppleIntelGraphics.kext" ] || [ -d "/System/Library/Extensions/AMDRadeon*.kext" ]; then
    GPU_AVAILABLE=1
    GPU_MODEL="Apple Silicon / AMD (integrated)"
fi

echo "System specs:"
echo "  Cores: $TOTAL_CORES"
echo "  RAM: ${TOTAL_RAM_GB}GB"
echo "  GPU: $GPU_MODEL"
echo ""

# Register device
sqlite3 "$DB_PATH" <<SQL
-- Register device
INSERT OR REPLACE INTO devices (
    id, owner_id, hostname, wallet_address, device_tier,
    uptime_percent, is_online, total_cores, total_ram_gb,
    gpu_available, gpu_model, is_primary_node
) VALUES (
    '$DEVICE_ID',
    X'$ADMIN_UUID',
    '$DEVICE_NAME',
    '$DEVICE_ID',
    '$DEVICE_ROLE',
    99.0, 1, $TOTAL_CORES, $TOTAL_RAM_GB,
    $GPU_AVAILABLE, '$GPU_MODEL',
    CASE WHEN '$DEVICE_ROLE' = 'master_node' THEN 1 ELSE 0 END
);

-- Register as node
INSERT OR REPLACE INTO nodes (id, owner_id, hostname, is_primary, node_type)
VALUES (
    '$DEVICE_ID',
    X'$ADMIN_UUID',
    '$DEVICE_NAME',
    CASE WHEN '$DEVICE_ROLE' = 'master_node' THEN 1 ELSE 0 END,
    CASE WHEN '$DEVICE_ROLE' = 'master_node' THEN 'master' ELSE 'backup' END
);
SQL

echo "✓ Device registered in ORCHA network"
echo ""

# Create sync configuration
echo "Creating APN sync configuration..."
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

SYNC_CONFIG="$HOME/pcg-cc-mcp/apn_sync_config.json"

cat > "$SYNC_CONFIG" <<EOF
{
  "device_id": "$DEVICE_ID",
  "device_role": "$DEVICE_ROLE",
  "sync_enabled": true,
  "sync_interval_seconds": 300,
  "peers": [
    {
      "device_id": "pythia",
      "hostname": "pythia",
      "role": "master_node",
      "priority": 1
    },
    {
      "device_id": "spaceterminal",
      "hostname": "spaceterminal",
      "role": "relay",
      "priority": 2
    },
    {
      "device_id": "bonomotion-mac-studio",
      "hostname": "bonomotion-mac-studio",
      "role": "master_node",
      "priority": 2
    }
  ],
  "sync_paths": [
    {
      "local": "$HOME/pcg-cc-mcp/dev_assets/db.sqlite",
      "remote": "dev_assets/db.sqlite",
      "direction": "bidirectional",
      "conflict_resolution": "master_wins"
    },
    {
      "local": "$HOME/topos",
      "remote": "topos",
      "direction": "bidirectional",
      "conflict_resolution": "newest_wins"
    }
  ],
  "database_sync": {
    "enabled": true,
    "master_device": "pythia",
    "replication_mode": "incremental",
    "conflict_resolution": "master_wins",
    "sync_tables": ["projects", "tasks", "users", "devices", "nodes", "project_members", "task_executions", "vibe_ledger"]
  }
}
EOF

echo "✓ Sync configuration created: $SYNC_CONFIG"
echo ""

# Create sync script
echo "Creating APN sync daemon..."
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

SYNC_SCRIPT="$HOME/pcg-cc-mcp/scripts/apn_sync_daemon.sh"

cat > "$SYNC_SCRIPT" <<'SYNCEOF'
#!/bin/bash
# APN Sync Daemon - Continuous bidirectional sync

SYNC_CONFIG="$HOME/pcg-cc-mcp/apn_sync_config.json"
SYNC_LOG="$HOME/pcg-cc-mcp/logs/apn_sync.log"
SYNC_INTERVAL=300  # 5 minutes

mkdir -p "$(dirname "$SYNC_LOG")"

log() {
    echo "[$(date '+%Y-%m-%d %H:%M:%S')] $*" | tee -a "$SYNC_LOG"
}

log "APN Sync Daemon starting..."

# Get device info from config
DEVICE_ID=$(jq -r '.device_id' "$SYNC_CONFIG")
DEVICE_ROLE=$(jq -r '.device_role' "$SYNC_CONFIG")
MASTER_DEVICE=$(jq -r '.database_sync.master_device' "$SYNC_CONFIG")

log "Device: $DEVICE_ID ($DEVICE_ROLE)"
log "Master: $MASTER_DEVICE"

while true; do
    log "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
    log "Starting sync cycle..."

    # Sync database
    if [ "$DEVICE_ROLE" = "master_node" ]; then
        log "Master node - broadcasting database updates..."

        # Export database changes
        DB_EXPORT="/tmp/orcha_db_export_$(date +%s).sql"
        sqlite3 "$HOME/pcg-cc-mcp/dev_assets/db.sqlite" ".dump" > "$DB_EXPORT"

        # Broadcast to all peers
        for peer in $(jq -r '.peers[].device_id' "$SYNC_CONFIG"); do
            if [ "$peer" != "$DEVICE_ID" ]; then
                log "Syncing database to $peer..."
                apn-send --target "$peer" --file "$DB_EXPORT" --type db_sync || log "Failed to sync to $peer"
            fi
        done

        rm -f "$DB_EXPORT"
    else
        log "Secondary device - pulling from master..."

        # Request database from master
        apn-request --from "$MASTER_DEVICE" --type db_sync || log "Failed to pull from master"
    fi

    # Sync project files
    log "Syncing project files..."

    # Get list of projects that need sync
    PROJECTS=$(sqlite3 "$HOME/pcg-cc-mcp/dev_assets/db.sqlite" "
        SELECT git_repo_path FROM projects WHERE deleted_at IS NULL;
    ")

    for project_path in $PROJECTS; do
        if [ -d "$project_path" ]; then
            # Calculate checksum
            CHECKSUM=$(find "$project_path" -type f -exec md5sum {} \; | sort | md5sum | cut -d' ' -f1)

            # Check if peers have different version
            for peer in $(jq -r '.peers[].device_id' "$SYNC_CONFIG"); do
                if [ "$peer" != "$DEVICE_ID" ]; then
                    PEER_CHECKSUM=$(apn-query --target "$peer" --path "$project_path" --query checksum 2>/dev/null || echo "")

                    if [ "$CHECKSUM" != "$PEER_CHECKSUM" ]; then
                        log "Project out of sync: $(basename "$project_path") with $peer"

                        # Sync based on modification time
                        LOCAL_MTIME=$(stat -c %Y "$project_path" 2>/dev/null || stat -f %m "$project_path")
                        PEER_MTIME=$(apn-query --target "$peer" --path "$project_path" --query mtime 2>/dev/null || echo "0")

                        if [ "$LOCAL_MTIME" -gt "$PEER_MTIME" ]; then
                            log "Pushing $(basename "$project_path") to $peer..."
                            apn-sync-push --target "$peer" --path "$project_path" || log "Push failed"
                        else
                            log "Pulling $(basename "$project_path") from $peer..."
                            apn-sync-pull --from "$peer" --path "$project_path" || log "Pull failed"
                        fi
                    fi
                fi
            done
        fi
    done

    log "Sync cycle complete. Next sync in ${SYNC_INTERVAL}s"
    sleep $SYNC_INTERVAL
done
SYNCEOF

chmod +x "$SYNC_SCRIPT"

echo "✓ Sync daemon created: $SYNC_SCRIPT"
echo ""

# Create systemd service (Linux) or launchd plist (macOS)
echo "Setting up auto-start service..."
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

if [ -d "/etc/systemd/system" ]; then
    # Linux systemd
    SERVICE_FILE="/etc/systemd/system/orcha-apn-sync.service"

    sudo tee "$SERVICE_FILE" > /dev/null <<SERVICEEOF
[Unit]
Description=ORCHA APN Sync Daemon
After=network.target

[Service]
Type=simple
User=$USER
WorkingDirectory=$HOME/pcg-cc-mcp
ExecStart=$SYNC_SCRIPT
Restart=always
RestartSec=10

[Install]
WantedBy=multi-user.target
SERVICEEOF

    sudo systemctl daemon-reload
    sudo systemctl enable orcha-apn-sync.service

    echo "✓ Systemd service created: $SERVICE_FILE"
    echo "  Start with: sudo systemctl start orcha-apn-sync"

elif [ -d "/Library/LaunchDaemons" ]; then
    # macOS launchd
    PLIST_FILE="$HOME/Library/LaunchAgents/com.orcha.apn-sync.plist"
    mkdir -p "$HOME/Library/LaunchAgents"

    cat > "$PLIST_FILE" <<PLISTEOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>com.orcha.apn-sync</string>
    <key>ProgramArguments</key>
    <array>
        <string>$SYNC_SCRIPT</string>
    </array>
    <key>RunAtLoad</key>
    <true/>
    <key>KeepAlive</key>
    <true/>
    <key>StandardOutPath</key>
    <string>$HOME/pcg-cc-mcp/logs/apn-sync-stdout.log</string>
    <key>StandardErrorPath</key>
    <string>$HOME/pcg-cc-mcp/logs/apn-sync-stderr.log</string>
</dict>
</plist>
PLISTEOF

    launchctl load "$PLIST_FILE" 2>/dev/null || echo "  (Service will start on next login)"

    echo "✓ LaunchAgent created: $PLIST_FILE"
    echo "  Start with: launchctl start com.orcha.apn-sync"
fi
echo ""

echo "═══════════════════════════════════════════════════════════════"
echo "✅ APN SYNC SETUP COMPLETE"
echo "═══════════════════════════════════════════════════════════════"
echo ""
echo "Device registered: $DEVICE_ID ($DEVICE_ROLE)"
echo "Sync configuration: $SYNC_CONFIG"
echo "Sync daemon: $SYNC_SCRIPT"
echo ""
echo "Next steps:"
echo ""
echo "1. Run this script on BOTH machines:"
echo "   • Pythia (Master Node): ./apn_sync_setup.sh"
echo "   • Space Terminal (Relay): ./apn_sync_setup.sh"
echo ""
echo "2. Start sync daemon on both:"
if [ -d "/etc/systemd/system" ]; then
    echo "   sudo systemctl start orcha-apn-sync"
elif [ -d "/Library/LaunchDaemons" ]; then
    echo "   launchctl start com.orcha.apn-sync"
else
    echo "   $SYNC_SCRIPT &"
fi
echo ""
echo "3. Verify devices can communicate:"
echo "   apn-ping pythia"
echo "   apn-ping spaceterminal"
echo ""
echo "4. Monitor sync activity:"
echo "   tail -f $HOME/pcg-cc-mcp/logs/apn_sync.log"
echo ""
echo "Both dashboards will stay synchronized! 🚀"
echo ""

#!/bin/bash
# Setup Auto-Sync Service for PCG Dashboard
# Configures automatic database syncing to storage provider

set -e

echo "======================================================================"
echo "PCG Dashboard - Auto-Sync Setup"
echo "======================================================================"
echo ""

# Configuration directory
CONFIG_DIR="$HOME/.config/pcg-dashboard"
CONFIG_FILE="$CONFIG_DIR/sovereign-sync.json"

# Create config directory
mkdir -p "$CONFIG_DIR"

echo "This script will configure automatic database syncing to a storage provider."
echo ""

# Check if config already exists
if [ -f "$CONFIG_FILE" ]; then
    echo "⚠️  Config file already exists: $CONFIG_FILE"
    read -p "Overwrite? (y/N): " -n 1 -r
    echo
    if [[ ! $REPLY =~ ^[Yy]$ ]]; then
        echo "Cancelled."
        exit 0
    fi
fi

# Gather configuration
echo ""
echo "Enter configuration details:"
echo ""

# Database path
read -p "Database path [~/.local/share/duck-kanban/db.sqlite]: " DB_PATH
DB_PATH=${DB_PATH:-~/.local/share/duck-kanban/db.sqlite}

# Expand tilde
DB_PATH_EXPANDED="${DB_PATH/#\~/$HOME}"

if [ ! -f "$DB_PATH_EXPANDED" ]; then
    echo "⚠️  Warning: Database not found at $DB_PATH"
    read -p "Continue anyway? (y/N): " -n 1 -r
    echo
    if [[ ! $REPLY =~ ^[Yy]$ ]]; then
        exit 1
    fi
fi

# Device ID
echo ""
echo "Device ID options:"
echo "  1. Sirak Laptop: c5ff23d2-1bdc-4479-b614-dfc103e8aa67"
echo "  2. Bonomotion Studio: ac5eefa8-6ccb-4181-a72d-062be107c338"
echo "  3. Custom"
read -p "Select device (1-3): " DEVICE_CHOICE

case $DEVICE_CHOICE in
    1)
        DEVICE_ID="c5ff23d2-1bdc-4479-b614-dfc103e8aa67"
        DEVICE_NAME="Sirak Laptop"
        ;;
    2)
        DEVICE_ID="ac5eefa8-6ccb-4181-a72d-062be107c338"
        DEVICE_NAME="Bonomotion Studio"
        ;;
    3)
        read -p "Enter device ID: " DEVICE_ID
        DEVICE_NAME="Custom Device"
        ;;
    *)
        echo "Invalid choice"
        exit 1
        ;;
esac

# Provider device ID
echo ""
echo "Storage Provider:"
echo "  1. Pythia Master Node: d903c0e7-f3a6-4967-80e7-0da9d0fe7632"
echo "  2. Custom"
read -p "Select provider (1-2): " PROVIDER_CHOICE

case $PROVIDER_CHOICE in
    1)
        PROVIDER_ID="d903c0e7-f3a6-4967-80e7-0da9d0fe7632"
        ;;
    2)
        read -p "Enter provider device ID: " PROVIDER_ID
        ;;
    *)
        echo "Invalid choice"
        exit 1
        ;;
esac

# Encryption password
echo ""
read -sp "Encryption password: " ENCRYPTION_PASSWORD
echo ""
read -sp "Confirm password: " ENCRYPTION_PASSWORD_CONFIRM
echo ""

if [ "$ENCRYPTION_PASSWORD" != "$ENCRYPTION_PASSWORD_CONFIRM" ]; then
    echo "❌ Passwords don't match"
    exit 1
fi

if [ -z "$ENCRYPTION_PASSWORD" ]; then
    echo "❌ Password cannot be empty"
    exit 1
fi

# Sync interval
echo ""
read -p "Sync interval in minutes [5]: " SYNC_INTERVAL
SYNC_INTERVAL=${SYNC_INTERVAL:-5}

# Enable by default
read -p "Enable auto-sync now? (Y/n): " ENABLE
ENABLE=${ENABLE:-Y}

if [[ $ENABLE =~ ^[Yy]$ ]]; then
    ENABLED="true"
else
    ENABLED="false"
fi

# Create config file
cat > "$CONFIG_FILE" <<EOF
{
  "enabled": $ENABLED,
  "database_path": "$DB_PATH",
  "device_id": "$DEVICE_ID",
  "provider_device_id": "$PROVIDER_ID",
  "encryption_password": "$ENCRYPTION_PASSWORD",
  "sync_interval_minutes": $SYNC_INTERVAL,
  "nats_url": "nats://nonlocal.info:4222"
}
EOF

# Secure the config file (contains password)
chmod 600 "$CONFIG_FILE"

echo ""
echo "======================================================================"
echo "✅ Configuration saved"
echo "======================================================================"
echo "Device: $DEVICE_NAME"
echo "Provider: Pythia Master Node"
echo "Database: $DB_PATH"
echo "Sync interval: $SYNC_INTERVAL minutes"
echo "Enabled: $ENABLED"
echo ""
echo "Config file: $CONFIG_FILE"
echo ""

# Ask about systemd service
echo "======================================================================"
echo "Systemd Service Setup (Optional)"
echo "======================================================================"
echo ""
echo "Would you like to install a systemd service to run auto-sync on startup?"
read -p "Install systemd service? (y/N): " INSTALL_SERVICE

if [[ $INSTALL_SERVICE =~ ^[Yy]$ ]]; then
    SERVICE_FILE="$HOME/.config/systemd/user/pcg-auto-sync.service"

    mkdir -p "$HOME/.config/systemd/user"

    cat > "$SERVICE_FILE" <<EOF
[Unit]
Description=PCG Dashboard Auto-Sync Service
Documentation=file://$HOME/pcg-cc-mcp/SOVEREIGN_DEPLOYMENT_GUIDE.md
After=network-online.target
Wants=network-online.target

[Service]
Type=simple
WorkingDirectory=$HOME/pcg-cc-mcp
ExecStart=$(which python3) -u $HOME/pcg-cc-mcp/sovereign_storage/auto_sync_service.py $CONFIG_FILE
Restart=always
RestartSec=10
StandardOutput=journal
StandardError=journal

[Install]
WantedBy=default.target
EOF

    # Reload systemd
    systemctl --user daemon-reload

    echo "✅ Systemd service installed: $SERVICE_FILE"
    echo ""
    echo "To enable and start the service:"
    echo "  systemctl --user enable pcg-auto-sync"
    echo "  systemctl --user start pcg-auto-sync"
    echo ""
    echo "To check status:"
    echo "  systemctl --user status pcg-auto-sync"
    echo ""
    echo "To view logs:"
    echo "  journalctl --user -u pcg-auto-sync -f"
    echo ""
fi

# Test the configuration
echo "======================================================================"
echo "Test Auto-Sync"
echo "======================================================================"
echo ""
read -p "Test the auto-sync now? (Y/n): " TEST_NOW

if [[ ! $TEST_NOW =~ ^[Nn]$ ]]; then
    echo ""
    echo "Running test sync..."
    echo ""
    python3 "$HOME/pcg-cc-mcp/sovereign_storage/auto_sync_service.py" "$CONFIG_FILE" &
    TEST_PID=$!

    # Let it run for 30 seconds
    sleep 30

    # Stop it
    kill $TEST_PID 2>/dev/null || true

    echo ""
    echo "Test complete. Check output above for any errors."
fi

echo ""
echo "======================================================================"
echo "✅ Setup Complete"
echo "======================================================================"
echo ""
echo "To run manually:"
echo "  python3 $HOME/pcg-cc-mcp/sovereign_storage/auto_sync_service.py"
echo ""
echo "To edit config:"
echo "  nano $CONFIG_FILE"
echo ""

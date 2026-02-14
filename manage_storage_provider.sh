#!/bin/bash
# Pythia Storage Provider Management Script

SCRIPT_DIR="/home/pythia/pcg-cc-mcp"
PID_FILE="/var/tmp/pythia_storage_provider.pid"
LOG_FILE="/var/tmp/pythia_storage_provider.log"
DEVICE_ID="d903c0e7-f3a6-4967-80e7-0da9d0fe7632"

case "$1" in
  start)
    if [ -f "$PID_FILE" ]; then
      PID=$(cat "$PID_FILE")
      if ps -p "$PID" > /dev/null 2>&1; then
        echo "❌ Storage provider already running (PID: $PID)"
        exit 1
      fi
    fi

    echo "🚀 Starting Pythia Storage Provider..."
    cd "$SCRIPT_DIR"
    nohup python3 -u sovereign_storage/storage_provider_server.py "$DEVICE_ID" > "$LOG_FILE" 2>&1 &
    PID=$!
    echo $PID > "$PID_FILE"

    sleep 2

    if ps -p "$PID" > /dev/null 2>&1; then
      echo "✅ Storage provider started successfully"
      echo "   PID: $PID"
      echo "   Log: $LOG_FILE"
    else
      echo "❌ Failed to start storage provider"
      cat "$LOG_FILE"
      exit 1
    fi
    ;;

  stop)
    if [ ! -f "$PID_FILE" ]; then
      echo "❌ PID file not found. Searching for process..."
      pkill -f "storage_provider_server.py"
      echo "✅ Killed any running instances"
      exit 0
    fi

    PID=$(cat "$PID_FILE")
    echo "🛑 Stopping storage provider (PID: $PID)..."

    if ps -p "$PID" > /dev/null 2>&1; then
      kill "$PID"
      sleep 2

      if ps -p "$PID" > /dev/null 2>&1; then
        echo "⚠️  Process still running, forcing..."
        kill -9 "$PID"
      fi

      echo "✅ Storage provider stopped"
    else
      echo "⚠️  Process not running"
    fi

    rm -f "$PID_FILE"
    ;;

  restart)
    $0 stop
    sleep 2
    $0 start
    ;;

  status)
    if [ ! -f "$PID_FILE" ]; then
      echo "❌ Storage provider not running (no PID file)"
      exit 1
    fi

    PID=$(cat "$PID_FILE")

    if ps -p "$PID" > /dev/null 2>&1; then
      echo "✅ Storage provider is running"
      echo "   PID: $PID"
      echo "   Device ID: $DEVICE_ID"
      echo ""

      # Check NATS connection
      if lsof -p "$PID" 2>/dev/null | grep -q "4222.*ESTABLISHED"; then
        echo "🌐 NATS Connection: ✅ CONNECTED"
      else
        echo "🌐 NATS Connection: ❌ NOT CONNECTED"
      fi

      echo ""
      echo "📊 Recent activity (last 10 lines):"
      tail -10 "$LOG_FILE"
    else
      echo "❌ Storage provider not running (stale PID file)"
      rm -f "$PID_FILE"
      exit 1
    fi
    ;;

  logs)
    if [ ! -f "$LOG_FILE" ]; then
      echo "❌ Log file not found"
      exit 1
    fi

    if [ "$2" = "follow" ] || [ "$2" = "-f" ]; then
      tail -f "$LOG_FILE"
    else
      tail -50 "$LOG_FILE"
    fi
    ;;

  stats)
    echo "📊 Storage Provider Statistics"
    echo "======================================================================"

    # Device registry stats
    sqlite3 /home/pythia/.local/share/duck-kanban/db.sqlite << 'EOF'
.mode column
.headers on
SELECT
  device_name as "Device",
  device_type as "Type",
  CASE WHEN is_online THEN '🟢 Online' ELSE '🔴 Offline' END as "Status",
  last_seen as "Last Seen"
FROM device_registry
ORDER BY device_name;
EOF

    echo ""
    echo "💾 Storage Replicas:"
    if [ -d "/home/pythia/.sovereign_storage" ]; then
      ls -lh /home/pythia/.sovereign_storage/ 2>/dev/null || echo "   No replicas stored yet"
    else
      echo "   No replicas stored yet"
    fi

    echo ""
    echo "💰 Storage Contracts:"
    sqlite3 /home/pythia/.local/share/duck-kanban/db.sqlite << 'EOF'
.mode column
.headers on
SELECT
  COUNT(*) as "Active Contracts",
  COALESCE(SUM(actual_storage_used_gb), 0) as "Storage Used (GB)",
  COALESCE(SUM(monthly_rate_vibe), 0) as "Monthly Revenue (VIBE)"
FROM storage_contracts
WHERE status = 'active';
EOF
    ;;

  *)
    echo "Pythia Storage Provider Management"
    echo ""
    echo "Usage: $0 {start|stop|restart|status|logs|stats}"
    echo ""
    echo "Commands:"
    echo "  start    - Start the storage provider server"
    echo "  stop     - Stop the storage provider server"
    echo "  restart  - Restart the storage provider server"
    echo "  status   - Show current status and recent activity"
    echo "  logs     - Show recent logs (add 'follow' to tail)"
    echo "  stats    - Show storage statistics"
    echo ""
    echo "Examples:"
    echo "  $0 start"
    echo "  $0 status"
    echo "  $0 logs follow"
    echo "  $0 stats"
    exit 1
    ;;
esac

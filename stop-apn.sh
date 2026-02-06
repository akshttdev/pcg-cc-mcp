#!/bin/bash
# Stop APN Master Node

echo "🛑 Stopping APN Master Node..."

if [ -f /tmp/apn_node.pid ]; then
    PID=$(cat /tmp/apn_node.pid)
    if ps -p $PID > /dev/null 2>&1; then
        kill $PID
        echo "✅ Sent stop signal to process $PID"
        sleep 2
        if ps -p $PID > /dev/null 2>&1; then
            echo "⚠️  Process still running, force killing..."
            kill -9 $PID
        fi
        rm /tmp/apn_node.pid
        echo "✅ Node stopped"
    else
        echo "⚠️  Process not running (stale PID file)"
        rm /tmp/apn_node.pid
    fi
else
    echo "⚠️  No PID file found (node not running?)"
fi

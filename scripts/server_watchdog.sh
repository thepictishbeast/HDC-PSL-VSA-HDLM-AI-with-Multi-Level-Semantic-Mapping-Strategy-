#!/bin/bash
# PlausiDen AI Server Watchdog
# Restarts the server automatically if it crashes or stops responding.
# Run: nohup bash /root/LFI/scripts/server_watchdog.sh &

SERVER_BIN="/home/user/cargo-target/release/server"
SERVER_DIR="/root/LFI/lfi_vsa_core"
LOG="/tmp/lfi_server.log"
CHECK_INTERVAL=10

echo "[watchdog] Starting PlausiDen AI server watchdog"

while true; do
    # Check if server is responding
    if ! curl -s --max-time 3 http://localhost:3000/api/health > /dev/null 2>&1; then
        echo "[watchdog] $(date) Server not responding — checking process..."

        # Kill any stuck server processes
        pkill -f "target/release/server" 2>/dev/null
        sleep 2

        # Check if binary exists
        if [ ! -f "$SERVER_BIN" ]; then
            echo "[watchdog] $(date) Binary missing — waiting for rebuild..."
            sleep 30
            continue
        fi

        # Start server
        echo "[watchdog] $(date) Starting server..."
        cd "$SERVER_DIR"
        nohup "$SERVER_BIN" > "$LOG" 2>&1 &
        echo "[watchdog] $(date) Server started PID: $!"
        sleep 15  # Give it time to start

        # Verify
        if curl -s --max-time 5 http://localhost:3000/api/health > /dev/null 2>&1; then
            echo "[watchdog] $(date) Server is UP"
        else
            echo "[watchdog] $(date) Server failed to start — will retry in ${CHECK_INTERVAL}s"
        fi
    fi

    sleep "$CHECK_INTERVAL"
done

#!/bin/bash
# ============================================================
# LFI IPC Daemon v5.6 — Workflow Delta (The Watchdog)
# Monitors lfi_bus.json and lfi_audit.json via inotifywait.
# Section 6: IPC Ledger communication between Alpha and Beta.
# ============================================================

SOVEREIGN_ROOT="$(cd "$(dirname "$0")" && pwd)"
BUS_FILE="$SOVEREIGN_ROOT/lfi_bus.json"
AUDIT_FILE="$SOVEREIGN_ROOT/lfi_audit.json"
LOG_FILE="$SOVEREIGN_ROOT/LFI.log"

log() {
    local ts
    ts="$(date -u +"%Y-%m-%dT%H:%M:%SZ")"
    echo "[$ts] [DAEMON] $1" | tee -a "$LOG_FILE"
}

log "LFI IPC Daemon v5.6 starting. Sovereign root: $SOVEREIGN_ROOT"
log "Enforcing Zero-Trust routing on bus files."

# Hostile Witness Check: verify inotifywait exists in PATH
if ! command -v inotifywait &>/dev/null; then
    log "FATAL: inotifywait not found. Install: apt-get install inotify-tools"
    exit 1
fi

# Verify bus files are material
for f in "$BUS_FILE" "$AUDIT_FILE"; do
    if [ ! -f "$f" ]; then
        log "FATAL: Required file missing: $f"
        exit 1
    fi
done

log "Watchdog active. Monitoring: $BUS_FILE, $AUDIT_FILE"

# The Kernel Watch Loop
inotifywait -m -e close_write --format "%w%f" "$BUS_FILE" "$AUDIT_FILE" 2>>"$LOG_FILE" | while read -r CHANGED_FILE
do
    ts="$(date -u +"%Y-%m-%dT%H:%M:%SZ")"

    if [ "$CHANGED_FILE" = "$BUS_FILE" ]; then
        log "EVENT [ALPHA -> BUS]: Payload injected. Awaiting Beta audit."
    elif [ "$CHANGED_FILE" = "$AUDIT_FILE" ]; then
        log "EVENT [BETA -> AUDIT]: Resolution posted. Alpha cleared to read."
    else
        log "EVENT [UNKNOWN]: $CHANGED_FILE modified."
    fi
done

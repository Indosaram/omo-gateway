#!/bin/bash
# Restore the pre-deploy binary and database, then restart.
set -euo pipefail
B="$1"   # backup dir printed by the deploy step
M=/Users/indo/code/project/omon-gateway
launchctl bootout gui/501/ai.omon.gateway 2>/dev/null || true
cp -p "$B/omo-gateway.prev" "$M/target/release/omo-gateway"
cp -p "$B/omon_gateway.db" "$M/omon_gateway.db"
rm -f "$M/omon_gateway.db-wal" "$M/omon_gateway.db-shm"
launchctl bootstrap gui/501 ~/Library/LaunchAgents/ai.omon.gateway.plist
echo ROLLED_BACK

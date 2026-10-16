#!/usr/bin/env bash
# Exercises scripts/gateway_watchdog.sh against fake endpoints; never touches the live gateway.
set -euo pipefail
REPO="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
WD="$REPO/scripts/gateway_watchdog.sh"
tmp="$(mktemp -d)"
trap 'kill "$srv" 2>/dev/null || true; rm -rf "$tmp"' EXIT

port=$(python3 -c 'import socket;s=socket.socket();s.bind(("127.0.0.1",0));print(s.getsockname()[1])')
python3 -m http.server "$port" --bind 127.0.0.1 --directory "$tmp" >/dev/null 2>&1 &
srv=$!
for _ in $(seq 1 50); do curl -s -o /dev/null "http://127.0.0.1:$port/" && break; sleep 0.1; done
mkdir -p "$tmp/chunks"; touch "$tmp/up"

run() {
    WATCHDOG_STATE_DIR="$tmp/state" WATCHDOG_DRY_ALERT=1 WATCHDOG_FAIL_LIMIT=3 \
    WATCHDOG_ALERT_COOLDOWN=1800 WATCHDOG_ENV_FILE=/dev/null \
    WATCHDOG_RESTART_CMD="echo RESTART >> $tmp/restarts" \
    WATCHDOG_SENPI_CHUNKS="$tmp/chunks" "$@" bash "$WD"
}
fail() { echo "FAIL: $*"; exit 1; }

# Healthy gateway: no restart, no alert.
echo 'x._turnEngineSubscribed' > "$tmp/chunks/session-worker.js"
out=$(run env WATCHDOG_GATEWAY_PATTERN="http.server $port" \
    WATCHDOG_DASHBOARD_URL="http://127.0.0.1:$port/up" WATCHDOG_APPSERVER_URL="http://127.0.0.1:$port/up")
[[ ! -f "$tmp/restarts" && "$out" != *DRY_ALERT* ]] || fail "healthy run restarted or alerted: $out"

# Dead gateway: restart every run, alert once at the limit, cooldown suppresses repeats.
for i in 1 2 3 4; do
    out=$(run env WATCHDOG_GATEWAY_PATTERN="no-such-process-$$" \
        WATCHDOG_DASHBOARD_URL="http://127.0.0.1:$port/up" WATCHDOG_APPSERVER_URL="http://127.0.0.1:$port/up")
    echo "$out" >> "$tmp/alerts"
done
[[ "$(wc -l < "$tmp/restarts" | tr -d ' ')" == 4 ]] || fail "expected 4 restarts"
[[ "$(grep -c 'DRY_ALERT gateway' "$tmp/alerts")" == 1 ]] || fail "expected exactly one gateway alert"

# Unhealthy app-server under a live gateway: alert at the limit, no gateway restart.
rm -f "$tmp/restarts" "$tmp/alerts"
for i in 1 2 3; do
    out=$(run env WATCHDOG_GATEWAY_PATTERN="http.server $port" \
        WATCHDOG_DASHBOARD_URL="http://127.0.0.1:$port/up" WATCHDOG_APPSERVER_URL="http://127.0.0.1:$port/missing")
    echo "$out" >> "$tmp/alerts"
done
[[ ! -f "$tmp/restarts" ]] || fail "app-server failure must not restart the gateway"
grep -q 'DRY_ALERT appserver' "$tmp/alerts" || fail "expected app-server alert"

# senpi patch wiped: alert.
echo 'x.subscribedThreadIds' > "$tmp/chunks/session-worker.js"
out=$(run env WATCHDOG_GATEWAY_PATTERN="http.server $port" \
    WATCHDOG_DASHBOARD_URL="http://127.0.0.1:$port/up" WATCHDOG_APPSERVER_URL="http://127.0.0.1:$port/up")
[[ "$out" == *"DRY_ALERT senpi_patch"* ]] || fail "expected senpi patch alert: $out"

# Default GATEWAY_PATTERN assertions: verify decoupled path and fallback behavior.
default_pat="$(sed -n 's/^GATEWAY_PATTERN=.*\${WATCHDOG_GATEWAY_PATTERN:-\([^}]*\)}.*/\1/p' "$WD")"
for path in "$HOME/.omon/bin/omo-gateway run" "/Users/x/code/project/omon-gateway/target/release/omo-gateway run"; do
    grep -Eq "$default_pat" <<<"$path" || fail "default GATEWAY_PATTERN '$default_pat' must match '$path'"
done

echo "test_gateway_watchdog: ok"

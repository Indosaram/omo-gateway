#!/usr/bin/env bash
# One-shot production deploy: build -> sign -> restart -> verify -> rollback on failure.
#
#   scripts/deploy.sh            build, sign, restart, verify
#   scripts/deploy.sh --verify   only run the post-deploy checks against the running gateway
#
# Verification passes only when, within VERIFY_TIMEOUT seconds of the restart:
#   - a new gateway process is running,
#   - it logged "Discord client ready" after the restart,
#   - the app-server answers readyz 200,
#   - exactly one app-server listens, and its parent is the gateway.
# On failure the previous binary is restored and the gateway restarted with it.
set -euo pipefail

REPO="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
BIN="$REPO/target/release/omo-gateway"
LABEL="ai.omon.gateway"
SIGN_IDENTITY="${SIGN_IDENTITY:-Developer ID Application: Indo Yoon (5DUM8WPB4C)}"
APPSERVER_URL="${OMON_OMO_APPSERVER_URL:-ws://127.0.0.1:19742}"
APPSERVER_HTTP="${APPSERVER_URL/ws:\/\//http://}"
LOG="$HOME/.omon/gateway.log"
VERIFY_TIMEOUT="${VERIFY_TIMEOUT:-120}"
BACKUP_DIR="$HOME/.omon/deploy-backups"

say() { printf '[deploy] %s\n' "$*"; }
die() { printf '[deploy] FAIL: %s\n' "$*" >&2; exit 1; }

gateway_pid() { pgrep -f "$BIN run" | head -1 || true; }

# Checks the running gateway. $1 = earliest acceptable log timestamp (UTC, ISO prefix).
verify() {
    local since="$1" old_pid="${2:-}" deadline=$((SECONDS + VERIFY_TIMEOUT))
    local pid="" ready="" code="" servers="" parent=""
    while ((SECONDS < deadline)); do
        pid="$(gateway_pid)"
        ready="$(sed 's/\x1b\[[0-9;]*m//g' "$LOG" 2>/dev/null \
            | grep -a 'Discord client ready' | awk -v s="$since" '$1 >= s' | tail -1 || true)"
        code="$(curl -s -m 3 -o /dev/null -w '%{http_code}' "$APPSERVER_HTTP/readyz" || true)"
        servers="$(pgrep -f "app-server --listen $APPSERVER_URL" || true)"
        if [[ -n "$pid" && "$pid" != "$old_pid" && -n "$ready" && "$code" == 200 \
            && "$(wc -l <<<"$servers" | tr -d ' ')" == 1 && -n "$servers" ]]; then
            parent="$(ps -o ppid= -p "$servers" | tr -d ' ')"
            if [[ "$parent" == "$pid" ]]; then
                say "verified: gateway=$pid discord=ready appserver=$servers(ppid=$parent) readyz=200"
                return 0
            fi
        fi
        sleep 2
    done
    say "verify timed out: gateway=${pid:-none} discord=${ready:+ready} readyz=${code:-none} appservers=[$(tr '\n' ' ' <<<"$servers")] parent=${parent:-?}"
    return 1
}

restart() {
    launchctl kickstart -k "gui/$(id -u)/$LABEL" >/dev/null 2>&1 \
        || launchctl bootstrap "gui/$(id -u)" "$HOME/Library/LaunchAgents/$LABEL.plist"
}

if [[ "${1:-}" == "--verify" ]]; then
    verify "0000" "" || die "running gateway is not healthy"
    exit 0
fi

cd "$REPO"
[[ -z "$(pgrep -f 'cargo (build|test|clippy).*omo-gateway' || true)" ]] \
    || die "another cargo build of omo-gateway holds the build lock; stop it first"

mkdir -p "$BACKUP_DIR"
backup=""
if [[ -x "$BIN" ]]; then
    backup="$BACKUP_DIR/omo-gateway.$(date +%Y%m%d-%H%M%S)"
    cp -p "$BIN" "$backup"
    say "backed up current binary to $backup"
fi

say "building release (sccache bypassed: a corrupt cache entry once broke every build)"
cargo build --release --bin omo-gateway --config 'build.rustc-wrapper=""' \
    || die "release build failed; running gateway untouched"

say "signing with $SIGN_IDENTITY"
codesign --force --options runtime --timestamp --sign "$SIGN_IDENTITY" \
    --identifier omo-gateway "$BIN"
codesign --verify --strict "$BIN" || die "signature verification failed"

old_pid="$(gateway_pid)"
since="$(date -u +%Y-%m-%dT%H:%M:%S)"
say "restarting $LABEL (old pid ${old_pid:-none})"
restart

if verify "$since" "$old_pid"; then
    say "deploy OK"
    exit 0
fi

if [[ -n "$backup" ]]; then
    say "rolling back to $backup"
    # Replace by rename: rewriting the running, signed binary in place can get it killed at exec.
    { cp -p "$backup" "$BIN.rollback" && mv -f "$BIN.rollback" "$BIN"; } \
        || die "rollback copy failed; $BIN still holds the failed build"
    since="$(date -u +%Y-%m-%dT%H:%M:%S)"
    old_pid="$(gateway_pid)"
    restart
    verify "$since" "$old_pid" && die "new build failed verification; rolled back to the previous binary" \
        || die "new build failed verification AND rollback did not come up healthy"
fi
die "new build failed verification and there was no previous binary to roll back to"

#!/usr/bin/env bash
# launchd watchdog for the omon gateway (ai.omon.gateway-watchdog, every 60s).
#
# Restarts the gateway when it is gone or unhealthy, and alerts the owner out of
# band (macOS notification + Discord DM via the bot token, not via the gateway)
# when restarts keep failing, the agent backend stays down, or the senpi
# app-server patch disappeared after an omo/senpi update.
#
# Every input is overridable so the logic can be exercised without touching the
# running gateway (see `tests/test_gateway_watchdog.sh`).
set -u

LABEL="${WATCHDOG_LABEL:-ai.omon.gateway}"
# Incident 2026-10-16: launchd runs the signed copy in ~/.omon/bin; the legacy target/release
# path is still accepted so this script (run live from the repo) never restarts a healthy
# gateway that has not been redeployed yet. pgrep -f takes an extended regex.
GATEWAY_PATTERN="${WATCHDOG_GATEWAY_PATTERN:-(/\.omon/bin|/target/release)/omo-gateway run}"
DASHBOARD_URL="${WATCHDOG_DASHBOARD_URL:-http://127.0.0.1:9119/api/health}"
APPSERVER_URL="${WATCHDOG_APPSERVER_URL:-http://127.0.0.1:19742/readyz}"
SENPI_CHUNKS="${WATCHDOG_SENPI_CHUNKS:-$HOME/.bun/install/global/node_modules/@code-yeongyu/senpi/dist/bundle/chunks}"
STATE_DIR="${WATCHDOG_STATE_DIR:-$HOME/.omon/watchdog-state}"
ENV_FILE="${WATCHDOG_ENV_FILE:-$HOME/code/project/omon-gateway/.env}"
OWNER_ID="${WATCHDOG_OWNER_ID:-414011632306618368}"
FAIL_LIMIT="${WATCHDOG_FAIL_LIMIT:-3}"
ALERT_COOLDOWN="${WATCHDOG_ALERT_COOLDOWN:-1800}"
DRY_ALERT="${WATCHDOG_DRY_ALERT:-0}"
RESTART_CMD="${WATCHDOG_RESTART_CMD:-}"

mkdir -p "$STATE_DIR"
now() { date +%s; }
log() { printf '%s watchdog: %s\n' "$(date '+%F %T')" "$*"; }

counter() { cat "$STATE_DIR/$1" 2>/dev/null || echo 0; }
set_counter() { echo "$2" >"$STATE_DIR/$1"; }

alert() {
    local key="$1" message="$2" last
    last="$(cat "$STATE_DIR/alert.$key" 2>/dev/null || echo 0)"
    (($(now) - last < ALERT_COOLDOWN)) && return 0
    now >"$STATE_DIR/alert.$key"
    log "ALERT[$key] $message"
    if [[ "$DRY_ALERT" == 1 ]]; then
        printf 'DRY_ALERT %s %s\n' "$key" "$message"
        return 0
    fi
    if command -v terminal-notifier >/dev/null 2>&1; then
        terminal-notifier -title "omon gateway" -message "$message" >/dev/null 2>&1
    else
        osascript -e "display notification \"${message//\"/}\" with title \"omon gateway\"" >/dev/null 2>&1
    fi
    discord_dm "⚠️ [gateway watchdog] $message"
}

discord_dm() {
    local token channel
    token="$(grep -E '^DISCORD_BOT_TOKEN=' "$ENV_FILE" 2>/dev/null | head -1 | cut -d= -f2- | tr -d '"'"'"' ')"
    [[ -z "$token" ]] && return 0
    channel="$(curl -s -m 10 -X POST https://discord.com/api/v10/users/@me/channels \
        -H "Authorization: Bot $token" -H 'Content-Type: application/json' \
        -d "{\"recipient_id\":\"$OWNER_ID\"}" | sed -n 's/.*"id": *"\([0-9]*\)".*/\1/p' | head -1)"
    [[ -z "$channel" ]] && return 0
    curl -s -m 10 -o /dev/null -X POST "https://discord.com/api/v10/channels/$channel/messages" \
        -H "Authorization: Bot $token" -H 'Content-Type: application/json' \
        --data-binary "$(printf '{"content":%s}' "$(printf '%s' "$1" | python3 -c 'import json,sys;print(json.dumps(sys.stdin.read()))')")"
}

restart_gateway() {
    if [[ -n "$RESTART_CMD" ]]; then
        eval "$RESTART_CMD"
        return
    fi
    local uid
    uid="$(id -u)"
    if launchctl print "gui/$uid/$LABEL" >/dev/null 2>&1; then
        launchctl kickstart -k "gui/$uid/$LABEL" >/dev/null 2>&1
    elif [[ -f "$HOME/Library/LaunchAgents/$LABEL.plist" ]]; then
        launchctl bootstrap "gui/$uid" "$HOME/Library/LaunchAgents/$LABEL.plist" >/dev/null 2>&1
    fi
}

http_ok() { [[ "$(curl -s -m 5 -o /dev/null -w '%{http_code}' "$1")" == 200 ]]; }

# 1. Gateway process and its own HTTP surface.
if pgrep -f "$GATEWAY_PATTERN" >/dev/null 2>&1 && http_ok "$DASHBOARD_URL"; then
    set_counter gateway_fail 0
else
    fails=$(($(counter gateway_fail) + 1))
    set_counter gateway_fail "$fails"
    log "gateway down or unhealthy (consecutive=$fails); restarting $LABEL"
    restart_gateway
    if ((fails >= FAIL_LIMIT)); then
        alert gateway "게이트웨이가 ${fails}분 연속 죽어 있거나 응답이 없어요. 자동 재시작이 계속 실패하는 중이에요. 확인: ~/.omon/gateway.log"
    fi
fi

# 2. Agent backend. The gateway restarts its own app-server; alert only if that keeps failing.
if pgrep -f "$GATEWAY_PATTERN" >/dev/null 2>&1; then
    if http_ok "$APPSERVER_URL"; then
        set_counter appserver_fail 0
    else
        fails=$(($(counter appserver_fail) + 1))
        set_counter appserver_fail "$fails"
        if ((fails >= FAIL_LIMIT)); then
            alert appserver "omo app-server가 ${fails}분 연속 응답이 없어요(디코 답장이 안 나갈 수 있어요). 확인: ~/.omon/omo-appserver-19742.log"
        fi
    fi
fi

# 3. senpi TurnEngine subscription patch, wiped by every omo/senpi reinstall.
if [[ -d "$SENPI_CHUNKS" ]]; then
    if grep -q '_turnEngineSubscribed' "$SENPI_CHUNKS"/session-worker.js 2>/dev/null; then
        rm -f "$STATE_DIR/alert.senpi_patch"
    else
        alert senpi_patch "senpi 업데이트로 app-server 구독 패치가 사라졌어요. 게이트웨이가 자동 재시작으로 버티지만 답장이 1분가량 늦어질 수 있어요. 패치 재적용이 필요해요."
    fi
fi
exit 0

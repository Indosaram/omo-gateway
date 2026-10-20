#!/usr/bin/env bash
# One-shot production deploy: build -> sign -> restart -> verify -> rollback on failure.
#
#   scripts/deploy.sh            build gateway + dashboard web, sign, restart, verify
#   scripts/deploy.sh --no-build skip both builds, install existing target/release binary and web/dist
#   scripts/deploy.sh --verify   only run the post-deploy checks against the running gateway
#
# The gateway binary and the dashboard web bundle are always built from the same source tree
# and installed together: the binary to $INSTALL_BIN, the web bundle to $INSTALL_WEB, which the
# launchd plist pins via DASHBOARD_WEB_ROOT. Incident 2026-10-17: a binary built from a clean
# commit worktree served a web/dist built from the dirty main tree, so the dashboard called API
# routes the binary did not have ("API endpoint not found") while every check passed.
#
# Verification passes only when, within VERIFY_TIMEOUT seconds of the restart:
#   - a new gateway process is running,
#   - the configured home transport connected after the restart,
#   - the app-server answers readyz 200,
#   - exactly one app-server listens, and its parent is the gateway,
#   - the dashboard serves the installed index.html and answers every DASHBOARD_PROBES path with 200.
# On failure the previous binary and web bundle are restored and the gateway restarted with them.
#
# Incident 2026-10-15: Plain `cargo build --release` rewrote target/release/omo-gateway
# with an ad-hoc signature (no TeamIdentifier), breaking macOS TCC Full Disk Access and
# causing iMessage collection failure. We decouple the launchd running binary ($INSTALL_BIN)
# from cargo's target/ output directory so that cargo builds never replace the signed binary
# launchd runs.
set -euo pipefail

REPO="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
TARGET_BIN="$REPO/target/release/omo-gateway"
INSTALL_BIN="${INSTALL_BIN:-$HOME/.omon/bin/omo-gateway}"
INSTALL_DIR="$(dirname "$INSTALL_BIN")"
INSTALL_WEB="${INSTALL_WEB:-$INSTALL_DIR/../web}"
WEB_SRC="$REPO/web"
BUN="${BUN:-$(command -v bun || printf '%s' "$HOME/.bun/bin/bun")}"
# Paths the dashboard web bundle depends on; each must answer 200 on the deployed gateway.
DASHBOARD_PROBES="${DASHBOARD_PROBES:-/api/health /api/tracker/bootstrap}"
LABEL="ai.omon.gateway"
PLIST="${LAUNCHD_PLIST:-$HOME/Library/LaunchAgents/$LABEL.plist}"
SIGN_IDENTITY="${SIGN_IDENTITY:-Developer ID Application: Indo Yoon (5DUM8WPB4C)}"
APPSERVER_URL="${OMON_OMO_APPSERVER_URL:-ws://127.0.0.1:19742}"
APPSERVER_HTTP="${APPSERVER_URL/ws:\/\//http://}"
LOG="$HOME/.omon/gateway.log"
VERIFY_TIMEOUT="${VERIFY_TIMEOUT:-120}"
# A restart cuts off every turn in flight; wait up to this long for them to finish first.
IDLE_WAIT_TIMEOUT="${IDLE_WAIT_TIMEOUT:-600}"
BACKUP_DIR="$HOME/.omon/deploy-backups"

say() { printf '[deploy] %s\n' "$*"; }
die() { printf '[deploy] FAIL: %s\n' "$*" >&2; exit 1; }

plist_value() { /usr/libexec/PlistBuddy -c "Print :$1" "$PLIST" 2>/dev/null || true; }

# The dashboard address the running gateway uses: DASHBOARD_URL, else the gateway's .env
# (launchd WorkingDirectory), else the built-in default port.
dashboard_url() {
    if [[ -n "${DASHBOARD_URL:-}" ]]; then printf '%s\n' "$DASHBOARD_URL"; return; fi
    local env_file port
    env_file="$(plist_value WorkingDirectory)/.env"
    port="$(grep -E '^DASHBOARD_PORT=' "$env_file" 2>/dev/null | tail -1 | cut -d= -f2 | tr -d "\"' " || true)"
    printf 'http://127.0.0.1:%s\n' "${port:-9119}"
}

# The web root the running gateway serves: the plist pin, else web/dist of its working directory.
served_web_root() {
    local pinned
    pinned="$(plist_value EnvironmentVariables:DASHBOARD_WEB_ROOT)"
    if [[ -n "$pinned" ]]; then printf '%s\n' "$pinned"; else printf '%s/web/dist\n' "$(plist_value WorkingDirectory)"; fi
}

# 0 when the dashboard serves the installed index.html and every probe path answers 200.
dashboard_ok() {
    local base path code served
    base="$(dashboard_url)"
    served="$(curl -s -m 3 "$base/" || true)"
    [[ -n "$served" && "$served" == "$(cat "$(served_web_root)/index.html" 2>/dev/null)" ]] || {
        dashboard_state="index.html differs from $(served_web_root)"
        return 1
    }
    for path in $DASHBOARD_PROBES; do
        code="$(curl -s -m 5 -o /dev/null -w '%{http_code}' "$base$path" || true)"
        [[ "$code" == 200 ]] || { dashboard_state="$path=$code"; return 1; }
    done
    dashboard_state="ok"
}

# Gateway PID matching the installed binary path.
gateway_pid() { pgrep -f "$INSTALL_BIN run" | head -1 || true; }

# During the FIRST deploy the old process still runs from target/release,
# so old_pid detection must accept either path (match both patterns).
detect_old_pid() {
    local pid
    pid="$(gateway_pid)"
    if [[ -z "$pid" ]]; then
        pid="$(pgrep -f "$TARGET_BIN run" | head -1 || true)"
    fi
    printf '%s\n' "$pid"
}

check_fda_warning() {
    local since="$1"
    local fda_denied
    fda_denied="$(sed 's/\x1b\[[0-9;]*m//g' "$LOG" 2>/dev/null \
        | grep -a 'full disk access: denied' | awk -v s="$since" '$1 >= s' | tail -1 || true)"
    if [[ -n "$fda_denied" ]]; then
        printf '\n*******************************************************************************\n' >&2
        printf '⚠️  WARNING: Full Disk Access is DENIED for %s\n' "$INSTALL_BIN" >&2
        printf 'Please enable Full Disk Access in macOS System Settings -> Privacy & Security -> Full Disk Access for:\n' >&2
        printf '  %s\n' "$INSTALL_BIN" >&2
        printf '*******************************************************************************\n\n' >&2
    fi
}

# Checks the running gateway. $1 = earliest acceptable log timestamp (UTC, ISO prefix).
verify() {
    local since="$1" old_pid="${2:-}" deadline=$((SECONDS + VERIFY_TIMEOUT))
    local pid="" ready="" code="" servers="" parent=""
    local transport="Discord" ready_pattern="Discord client ready"
    if [[ "$(plist_value EnvironmentVariables:OMON_HOME_PLATFORM)" == slack ]]; then
        transport="Slack"
        ready_pattern="slack socket: connected"
    fi
    dashboard_state="unchecked"
    while ((SECONDS < deadline)); do
        pid="$(gateway_pid)"
        ready="$(sed 's/\x1b\[[0-9;]*m//g' "$LOG" 2>/dev/null \
            | grep -a "$ready_pattern" | awk -v s="$since" '$1 >= s' | tail -1 || true)"
        code="$(curl -s -m 3 -o /dev/null -w '%{http_code}' "$APPSERVER_HTTP/readyz" || true)"
        servers="$(pgrep -f "app-server --listen $APPSERVER_URL" || true)"
        if [[ -n "$pid" && "$pid" != "$old_pid" && -n "$ready" && "$code" == 200 \
            && "$(wc -l <<<"$servers" | tr -d ' ')" == 1 && -n "$servers" ]]; then
            parent="$(ps -o ppid= -p "$servers" | tr -d ' ')"
            if [[ "$parent" == "$pid" ]] && dashboard_ok; then
                say "verified: gateway=$pid transport=$transport:ready appserver=$servers(ppid=$parent) readyz=200 dashboard=$(dashboard_url) probes=[$DASHBOARD_PROBES]"
                check_fda_warning "$since"
                return 0
            fi
        fi
        sleep 2
    done
    say "verify timed out: gateway=${pid:-none} transport=$transport:${ready:+ready} readyz=${code:-none} appservers=[$(tr '\n' ' ' <<<"$servers")] parent=${parent:-?} dashboard=$dashboard_state"
    check_fda_warning "$since"
    return 1
}

# Waits until the running gateway reports no session running a turn (busy_sessions=0) or
# IDLE_WAIT_TIMEOUT passes. A gateway that predates the field, or does not answer, is not waited on.
wait_for_idle_sessions() {
    local deadline=$((SECONDS + IDLE_WAIT_TIMEOUT)) busy announced=0
    while ((SECONDS < deadline)); do
        busy="$(curl -s -m 5 "$(dashboard_url)/api/status" 2>/dev/null \
            | python3 -c 'import json,sys
try: print(json.load(sys.stdin).get("busy_sessions", 0))
except Exception: print(0)' 2>/dev/null || echo 0)"
        [[ "$busy" =~ ^[0-9]+$ ]] || busy=0
        ((busy == 0)) && return 0
        if ((announced == 0)); then
            say "waiting for $busy running turn(s) to finish before restarting (up to ${IDLE_WAIT_TIMEOUT}s)"
            announced=1
        fi
        sleep 5
    done
    say "turns still running after ${IDLE_WAIT_TIMEOUT}s; restarting anyway (they resume on boot)"
}

plist_modified=0
restart() {
    local uid="${UID:-$(id -u)}"
    if [[ "$plist_modified" == 1 ]]; then
        say "reloading $LABEL with bootout and bootstrap (plist was updated)"
        launchctl bootout "gui/$uid/$LABEL" >/dev/null 2>&1 || true
        # bootout returns before launchd has removed the job; an immediate bootstrap then
        # fails with "5: Input/output error" and leaves the gateway down (2026-10-16).
        local tries=0
        while launchctl print "gui/$uid/$LABEL" >/dev/null 2>&1 && ((tries < 30)); do
            sleep 1
            tries=$((tries + 1))
        done
        tries=0
        until launchctl bootstrap "gui/$uid" "$PLIST"; do
            tries=$((tries + 1))
            ((tries >= 5)) && die "launchctl bootstrap failed $tries times; gateway is NOT running"
            sleep 2
        done
        plist_modified=0
    else
        launchctl kickstart -k "gui/$uid/$LABEL" >/dev/null 2>&1 \
            || launchctl bootstrap "gui/$uid" "$PLIST"
    fi
}

NO_BUILD=0
ONLY_VERIFY=0

for arg in "$@"; do
    case "$arg" in
        --verify)
            ONLY_VERIFY=1
            ;;
        --no-build)
            NO_BUILD=1
            ;;
        *)
            die "unknown argument: $arg"
            ;;
    esac
done

if [[ "$ONLY_VERIFY" == 1 ]]; then
    # Before the first pinned-path deploy the gateway still runs from target/release.
    if [[ -z "$(gateway_pid)" ]]; then gateway_pid() { pgrep -f "$TARGET_BIN run" | head -1 || true; }; fi
    verify "0000" "" || die "running gateway is not healthy"
    exit 0
fi

cd "$REPO"
# Several worktrees deploy to the same gateway. A build that does not contain the revision
# currently deployed silently reverts another session's fixes (2026-10-19: the full-access
# and Slack ingress fixes were overwritten twice). Refuse it unless forced.
deployed_rev="$(sed -n 's/^rev=\([0-9a-f]*\).*/\1/p' "$INSTALL_DIR/DEPLOYED" 2>/dev/null | head -1 || true)"
if [[ -n "$deployed_rev" && "${OMON_DEPLOY_ALLOW_REWIND:-0}" != 1 ]]; then
    if git -C "$REPO" cat-file -e "$deployed_rev^{commit}" 2>/dev/null; then
        git -C "$REPO" merge-base --is-ancestor "$deployed_rev" HEAD \
            || die "HEAD does not contain the deployed revision $deployed_rev; merge it first (or set OMON_DEPLOY_ALLOW_REWIND=1)"
    else
        die "deployed revision $deployed_rev is unknown to $REPO; fetch or merge it first (or set OMON_DEPLOY_ALLOW_REWIND=1)"
    fi
fi
if [[ "$NO_BUILD" == 0 ]]; then
    [[ -z "$(pgrep -f 'cargo (build|test|clippy).*omo-gateway' || true)" ]] \
        || die "another cargo build of omo-gateway holds the build lock; stop it first"
fi

mkdir -p "$BACKUP_DIR"
mkdir -p "$INSTALL_DIR"

stamp="$(date +%Y%m%d-%H%M%S)"
backup=""
if [[ -x "$INSTALL_BIN" ]]; then
    backup="$BACKUP_DIR/omo-gateway.$stamp"
    cp -p "$INSTALL_BIN" "$backup"
    say "backed up current binary to $backup"
fi
# Back up whatever the gateway serves now, so a rollback restores a matching binary + web pair.
web_backup=""
if [[ -f "$(served_web_root)/index.html" ]]; then
    web_backup="$BACKUP_DIR/web.$stamp"
    cp -Rp "$(served_web_root)" "$web_backup"
    say "backed up served web bundle $(served_web_root) to $web_backup"
fi

source_rev="$(git -C "$REPO" rev-parse --short HEAD 2>/dev/null || echo unknown)"
source_dirty="$(git -C "$REPO" status --porcelain 2>/dev/null | wc -l | tr -d ' ')"
say "source tree $REPO at $source_rev with $source_dirty uncommitted paths (binary and web both built from it)"

if [[ "$NO_BUILD" == 1 ]]; then
    say "skipping cargo and web builds (--no-build specified)"
    [[ -x "$TARGET_BIN" ]] || die "target binary not found or not executable at $TARGET_BIN"
    [[ -f "$WEB_SRC/dist/index.html" ]] || die "web bundle not found at $WEB_SRC/dist"
else
    say "building dashboard web bundle in $WEB_SRC"
    [[ -x "$BUN" ]] || die "bun not found (set BUN=/path/to/bun); running gateway untouched"
    (cd "$WEB_SRC" && "$BUN" install --frozen-lockfile && "$BUN" run build) \
        || die "web build failed; running gateway untouched"
    say "building release (sccache bypassed: a corrupt cache entry once broke every build; strip=none for macOS ld bug)"
    cargo build --release --bin omo-gateway \
        --config 'build.rustc-wrapper=""' \
        --config 'profile.release.strip="none"' \
        --config 'profile.release.build-override.strip="none"' \
        || die "release build failed; running gateway untouched"
fi

say "staging built binary to $INSTALL_BIN.new"
cp -p "$TARGET_BIN" "$INSTALL_BIN.new"

say "signing $INSTALL_BIN.new with $SIGN_IDENTITY"
codesign --force --options runtime --timestamp --sign "$SIGN_IDENTITY" \
    --identifier omo-gateway "$INSTALL_BIN.new"
codesign --verify --strict "$INSTALL_BIN.new" || die "signature verification failed"

# Check codesign -dv reports TeamIdentifier=5DUM8WPB4C; fail otherwise
codesign_dv="$(codesign -dv "$INSTALL_BIN.new" 2>&1 || true)"
if ! grep -q 'TeamIdentifier=5DUM8WPB4C' <<<"$codesign_dv"; then
    rm -f "$INSTALL_BIN.new"
    die "codesign check failed: TeamIdentifier is not 5DUM8WPB4C (got: $(grep 'TeamIdentifier=' <<<"$codesign_dv" || echo 'not found'))"
fi

say "staging web bundle to $INSTALL_WEB.new"
rm -rf "$INSTALL_WEB.new"
cp -Rp "$WEB_SRC/dist" "$INSTALL_WEB.new"

say "atomically installing binary to $INSTALL_BIN"
mv -f "$INSTALL_BIN.new" "$INSTALL_BIN"

say "installing web bundle to $INSTALL_WEB"
rm -rf "$INSTALL_WEB.old"
[[ -e "$INSTALL_WEB" ]] && mv "$INSTALL_WEB" "$INSTALL_WEB.old"
mv "$INSTALL_WEB.new" "$INSTALL_WEB"
rm -rf "$INSTALL_WEB.old"
INSTALL_WEB="$(cd "$INSTALL_WEB" && pwd)"

# Ensure launchd plist ProgramArguments[0] equals INSTALL_BIN
if [[ -f "$PLIST" ]]; then
    current_arg0="$(/usr/libexec/PlistBuddy -c "Print :ProgramArguments:0" "$PLIST" 2>/dev/null || true)"
    if [[ "$current_arg0" != "$INSTALL_BIN" ]]; then
        say "plist ProgramArguments[0] ($current_arg0) does not match $INSTALL_BIN; updating $PLIST"
        plist_backup="$BACKUP_DIR/$(basename "$PLIST").$(date +%Y%m%d-%H%M%S)"
        cp -p "$PLIST" "$plist_backup"
        say "backed up plist to $plist_backup"
        /usr/libexec/PlistBuddy -c "Set :ProgramArguments:0 $INSTALL_BIN" "$PLIST" \
            || die "failed to update ProgramArguments[0] in $PLIST"
        plist_modified=1
    fi
    # Pin the dashboard to the web bundle installed with this binary.
    if [[ "$(plist_value EnvironmentVariables:DASHBOARD_WEB_ROOT)" != "$INSTALL_WEB" ]]; then
        say "pinning DASHBOARD_WEB_ROOT=$INSTALL_WEB in $PLIST"
        [[ "$plist_modified" == 1 ]] || cp -p "$PLIST" "$BACKUP_DIR/$(basename "$PLIST").$stamp"
        /usr/libexec/PlistBuddy -c "Set :EnvironmentVariables:DASHBOARD_WEB_ROOT $INSTALL_WEB" "$PLIST" 2>/dev/null \
            || /usr/libexec/PlistBuddy -c "Add :EnvironmentVariables:DASHBOARD_WEB_ROOT string $INSTALL_WEB" "$PLIST" \
            || die "failed to set DASHBOARD_WEB_ROOT in $PLIST"
        plist_modified=1
    fi
    # A kickstart keeps the environment launchd loaded; reload when the plist's
    # EnvironmentVariables changed since then (for example a new feature flag).
    if [[ "$plist_modified" != 1 ]]; then
        loaded_env="$( { launchctl print "gui/${UID:-$(id -u)}/$LABEL" 2>/dev/null || true; } \
            | sed -n '/^\tenvironment = {/,/^\t}/p' | sed -n 's/^\t\t\([^ ]*\) => \(.*\)$/\1=\2/p' | sort)"
        plist_env="$(/usr/libexec/PlistBuddy -c "Print :EnvironmentVariables" "$PLIST" 2>/dev/null \
            | sed -n 's/^ *\([^ ]*\) = \(.*\)$/\1=\2/p' | sort)"
        if [[ -n "$loaded_env" ]] && ! comm -23 <(printf '%s\n' "$plist_env") <(printf '%s\n' "$loaded_env") | grep -q .; then
            :
        elif [[ -n "$loaded_env" ]]; then
            say "plist EnvironmentVariables differ from the loaded job; reloading"
            plist_modified=1
        fi
    fi
fi

old_pid="$(detect_old_pid)"
[[ -n "$old_pid" ]] && wait_for_idle_sessions
since="$(date -u +%Y-%m-%dT%H:%M:%S)"
say "restarting $LABEL (old pid ${old_pid:-none})"
restart

if verify "$since" "$old_pid"; then
    # Recorded only once the new build is verified: a rolled-back build must never become the
    # revision the rewind guard above protects.
    printf 'rev=%s dirty_paths=%s source=%s deployed_at=%s\n' "$source_rev" "$source_dirty" "$REPO" "$stamp" \
        > "$INSTALL_DIR/DEPLOYED"
    say "deploy OK"
    exit 0
fi

if [[ -n "$backup" ]]; then
    say "rolling back to $backup"
    # Atomic rollback via temporary file and rename to avoid killing running binary
    { cp -p "$backup" "$INSTALL_BIN.rollback" && mv -f "$INSTALL_BIN.rollback" "$INSTALL_BIN"; } \
        || die "rollback copy failed; $INSTALL_BIN still holds the failed build"
    if [[ -n "$web_backup" ]]; then
        say "restoring web bundle from $web_backup"
        { rm -rf "$INSTALL_WEB.rollback" && cp -Rp "$web_backup" "$INSTALL_WEB.rollback" \
            && rm -rf "$INSTALL_WEB" && mv "$INSTALL_WEB.rollback" "$INSTALL_WEB"; } \
            || die "web rollback failed; $INSTALL_WEB may not match the restored binary"
    fi
    since="$(date -u +%Y-%m-%dT%H:%M:%S)"
    old_pid="$(detect_old_pid)"
    restart
    # The previous binary may predate newer probe routes; only require the dashboard to serve.
    DASHBOARD_PROBES="/api/health"
    verify "$since" "$old_pid" && die "new build failed verification; rolled back to the previous binary" \
        || die "new build failed verification AND rollback did not come up healthy"
fi
die "new build failed verification and there was no previous binary to roll back to"

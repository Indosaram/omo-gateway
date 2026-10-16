#!/usr/bin/env bash
# One-shot production deploy: build -> sign -> restart -> verify -> rollback on failure.
#
#   scripts/deploy.sh            build, sign, restart, verify
#   scripts/deploy.sh --no-build skip build, install/sign existing target/release binary
#   scripts/deploy.sh --verify   only run the post-deploy checks against the running gateway
#
# Verification passes only when, within VERIFY_TIMEOUT seconds of the restart:
#   - a new gateway process is running,
#   - it logged "Discord client ready" after the restart,
#   - the app-server answers readyz 200,
#   - exactly one app-server listens, and its parent is the gateway.
# On failure the previous binary is restored and the gateway restarted with it.
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
LABEL="ai.omon.gateway"
PLIST="${LAUNCHD_PLIST:-$HOME/Library/LaunchAgents/$LABEL.plist}"
SIGN_IDENTITY="${SIGN_IDENTITY:-Developer ID Application: Indo Yoon (5DUM8WPB4C)}"
APPSERVER_URL="${OMON_OMO_APPSERVER_URL:-ws://127.0.0.1:19742}"
APPSERVER_HTTP="${APPSERVER_URL/ws:\/\//http://}"
LOG="$HOME/.omon/gateway.log"
VERIFY_TIMEOUT="${VERIFY_TIMEOUT:-120}"
BACKUP_DIR="$HOME/.omon/deploy-backups"

say() { printf '[deploy] %s\n' "$*"; }
die() { printf '[deploy] FAIL: %s\n' "$*" >&2; exit 1; }

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
                check_fda_warning "$since"
                return 0
            fi
        fi
        sleep 2
    done
    say "verify timed out: gateway=${pid:-none} discord=${ready:+ready} readyz=${code:-none} appservers=[$(tr '\n' ' ' <<<"$servers")] parent=${parent:-?}"
    check_fda_warning "$since"
    return 1
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
    if [[ -z "$(gateway_pid)" ]]; then gateway_pid() { detect_old_pid; }; fi
    verify "0000" "" || die "running gateway is not healthy"
    exit 0
fi

cd "$REPO"
if [[ "$NO_BUILD" == 0 ]]; then
    [[ -z "$(pgrep -f 'cargo (build|test|clippy).*omo-gateway' || true)" ]] \
        || die "another cargo build of omo-gateway holds the build lock; stop it first"
fi

mkdir -p "$BACKUP_DIR"
mkdir -p "$INSTALL_DIR"

backup=""
if [[ -x "$INSTALL_BIN" ]]; then
    backup="$BACKUP_DIR/omo-gateway.$(date +%Y%m%d-%H%M%S)"
    cp -p "$INSTALL_BIN" "$backup"
    say "backed up current binary to $backup"
fi

if [[ "$NO_BUILD" == 1 ]]; then
    say "skipping cargo build (--no-build specified)"
    [[ -x "$TARGET_BIN" ]] || die "target binary not found or not executable at $TARGET_BIN"
else
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

say "atomically installing binary to $INSTALL_BIN"
mv -f "$INSTALL_BIN.new" "$INSTALL_BIN"

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
fi

old_pid="$(detect_old_pid)"
since="$(date -u +%Y-%m-%dT%H:%M:%S)"
say "restarting $LABEL (old pid ${old_pid:-none})"
restart

if verify "$since" "$old_pid"; then
    say "deploy OK"
    exit 0
fi

if [[ -n "$backup" ]]; then
    say "rolling back to $backup"
    # Atomic rollback via temporary file and rename to avoid killing running binary
    { cp -p "$backup" "$INSTALL_BIN.rollback" && mv -f "$INSTALL_BIN.rollback" "$INSTALL_BIN"; } \
        || die "rollback copy failed; $INSTALL_BIN still holds the failed build"
    since="$(date -u +%Y-%m-%dT%H:%M:%S)"
    old_pid="$(detect_old_pid)"
    restart
    verify "$since" "$old_pid" && die "new build failed verification; rolled back to the previous binary" \
        || die "new build failed verification AND rollback did not come up healthy"
fi
die "new build failed verification and there was no previous binary to roll back to"

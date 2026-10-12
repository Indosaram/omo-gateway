#!/usr/bin/env bash
set -euo pipefail

if [[ "${OMO_GATEWAY_TEST_STUB:-}" == "1" || "$(basename "$0")" == "omo-gateway" ]]; then
    printf '%s\n' "$@" > "$OMO_GATEWAY_TEST_ARGS"
    printf '%s\n' "${DATABASE_URL:-}" > "$OMO_GATEWAY_TEST_ARGS.env"
    exit 0
fi

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
BIN_PATH="$REPO_ROOT/bin/omomeow-delegate"

TEST_TMP_DIR=$(mktemp -d -t test_omomeow_delegate_XXXXXX)
trap 'rm -rf "$TEST_TMP_DIR"' EXIT

TEST_DB="$TEST_TMP_DIR/test_gateway.db"
TEST_HOME="$TEST_TMP_DIR/home with spaces"
mkdir -p "$TEST_HOME" "$TEST_TMP_DIR/bin" "$TEST_TMP_DIR/cwd with spaces"
cat > "$TEST_TMP_DIR/bin/omo-gateway" <<'STUB'
#!/usr/bin/env bash
exec bash "$OMO_GATEWAY_TEST_SCRIPT" "$@"
STUB
chmod +x "$TEST_TMP_DIR/bin/omo-gateway"

# Keep any evaluated injection marker inside the directory checked and cleaned below.
cd "$TEST_TMP_DIR"

echo "=== Running omomeow-delegate integration tests ==="

if [[ ! -x "$BIN_PATH" ]]; then
    if [[ "$(git -C "$REPO_ROOT" ls-files -s -- bin/omomeow-delegate | awk '{print $1}')" == "100755" ]]; then
        chmod u+x "$BIN_PATH"
    else
        echo "FAIL: $BIN_PATH is not executable in git or filesystem" >&2
        exit 1
    fi
fi

HELP_OUT=$("$BIN_PATH" --help)
if ! echo "$HELP_OUT" | grep -q "Usage:"; then
    echo "FAIL: Help output missing Usage" >&2
    exit 1
fi
echo "PASS: Help output verified"

if MISSING_OUTPUT=$(HOME="$TEST_HOME" "$BIN_PATH" --title "Only Title" 2>&1); then
    echo "FAIL: Expected failure on missing required args" >&2
    exit 1
fi
if [[ "$MISSING_OUTPUT" != *"Usage:"* ]]; then
    echo "FAIL: Missing-title failure did not show usage" >&2
    exit 1
fi
echo "PASS: Missing argument validation verified"

TEST_TITLE='Refactor '\''background'\'' indexing; $(touch SHOULD_NOT_RUN)'
TEST_REPO="$REPO_ROOT"
TEST_BRIEF='Run long-running analysis; $(touch SHOULD_NOT_RUN) "literally"'
TEST_THREAD="discord_th_998877"
ARGS_FILE="$TEST_TMP_DIR/gateway-argv"

OUTPUT=$(HOME="$TEST_HOME" PATH="$TEST_TMP_DIR/bin:$PATH" \
    DATABASE_URL="parent-database-must-not-win" \
    OMO_GATEWAY_TEST_STUB=1 OMO_GATEWAY_TEST_ARGS="$ARGS_FILE" \
    OMO_GATEWAY_TEST_SCRIPT="$REPO_ROOT/tests/test_omomeow_delegate.sh" "$BIN_PATH" \
    --title "$TEST_TITLE" \
    --repo "$TEST_REPO" \
    --brief "$TEST_BRIEF" \
    --thread-id "$TEST_THREAD" \
    --cwd "$TEST_TMP_DIR/cwd with spaces" \
    --db "$TEST_DB" \
    --done 'command ["git", "-C", "/a path/$HOME", "diff", "--quiet"]')

if [[ ! -f "$ARGS_FILE" ]]; then
    echo "FAIL: gateway stub did not capture argv" >&2
    exit 1
fi

GATEWAY_ARGS=()
while IFS= read -r arg; do GATEWAY_ARGS+=("$arg"); done < "$ARGS_FILE"
if [[ "${GATEWAY_ARGS[0]}" != "lane" || "${GATEWAY_ARGS[1]}" != "start" ]]; then
    echo "FAIL: expected lane start argv" >&2
    exit 1
fi
has_arg() {
    local expected="$1" arg
    for arg in "${GATEWAY_ARGS[@]}"; do [[ "$arg" == "$expected" ]] && return 0; done
    return 1
}
for expected in "--title" "$TEST_TITLE" "--brief" "--done" '--thread' "$TEST_THREAD" "--cwd" "$TEST_TMP_DIR/cwd with spaces"; do
    if ! has_arg "$expected"; then
        echo "FAIL: gateway argv missing exact argument: $expected" >&2
        exit 1
    fi
done
if ! has_arg 'command ["git", "-C", "/a path/$HOME", "diff", "--quiet"]'; then
    echo "FAIL: JSON completion signal spacing/special characters changed" >&2
    exit 1
fi
LANE_KEY="${GATEWAY_ARGS[2]}"
if [[ ! "$LANE_KEY" =~ ^[a-z0-9-]{1,40}-[0-9a-f]{4}$ ]]; then
    echo "FAIL: invalid lane key format: $LANE_KEY" >&2
    exit 1
fi
BRIEF_ARG_INDEX=0
for i in "${!GATEWAY_ARGS[@]}"; do
    [[ "${GATEWAY_ARGS[$i]}" == "--brief" ]] && BRIEF_ARG_INDEX=$((i + 1))
done
SAVED_BRIEF="${GATEWAY_ARGS[$BRIEF_ARG_INDEX]}"
if [[ ! -f "$SAVED_BRIEF" || "$(<"$SAVED_BRIEF")" != "$TEST_BRIEF" ]]; then
    echo "FAIL: persisted brief content mismatch" >&2
    exit 1
fi
if ! grep -Fxq "$TEST_DB" "$ARGS_FILE.env"; then
    echo "FAIL: gateway stub did not receive --db as DATABASE_URL" >&2
    exit 1
fi
if [[ -e "$TEST_TMP_DIR/SHOULD_NOT_RUN" ]]; then
    echo "FAIL: shell-looking brief text was evaluated" >&2
    exit 1
fi
if [[ "$(cat "$SAVED_BRIEF")" != "$TEST_BRIEF" ]]; then
    echo "FAIL: persisted brief bytes differ from inline instructions" >&2
    exit 1
fi

if [[ -e "$TEST_DB" ]]; then
    echo "FAIL: wrapper created or wrote a database" >&2
    exit 1
fi
echo "PASS: lane start argv, brief state, and no database writes verified"

DEFAULT_ARGS="$TEST_TMP_DIR/default-argv"
DEFAULT_OUTPUT=$(HOME="$TEST_HOME" PATH="$TEST_TMP_DIR/bin:$PATH" \
    OMO_GATEWAY_TEST_STUB=1 OMO_GATEWAY_TEST_ARGS="$DEFAULT_ARGS" \
    OMO_GATEWAY_TEST_SCRIPT="$REPO_ROOT/tests/test_omomeow_delegate.sh" "$BIN_PATH" \
    --title "URI test" \
    --repo "$TEST_REPO" \
    --brief "$TEST_BRIEF" \
    --db "$TEST_DB" 2>&1)
if [[ "$DEFAULT_OUTPUT" != *"Warning:"* ]]; then
    echo "FAIL: default completion signal warning missing" >&2
    exit 1
fi
if ! grep -Fq "command [\"git\",\"-C\",\"$TEST_REPO\",\"diff\",\"--quiet\"]" "$DEFAULT_ARGS"; then
    echo "FAIL: default completion signal missing" >&2
    exit 1
fi
echo "PASS: default completion signal and warning verified"

echo "=== All integration tests PASSED successfully ==="

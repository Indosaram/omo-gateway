#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
BIN_PATH="$REPO_ROOT/bin/omomeow-delegate"

TEST_TMP_DIR=$(mktemp -d -t test_omomeow_delegate_XXXXXX)
trap 'rm -rf "$TEST_TMP_DIR"' EXIT

TEST_DB="$TEST_TMP_DIR/test_gateway.db"

echo "=== Running omomeow-delegate integration tests ==="

if [[ ! -x "$BIN_PATH" ]]; then
    echo "FAIL: $BIN_PATH is not executable" >&2
    exit 1
fi

HELP_OUT=$("$BIN_PATH" --help)
if ! echo "$HELP_OUT" | grep -q "Usage:"; then
    echo "FAIL: Help output missing Usage" >&2
    exit 1
fi
echo "PASS: Help output verified"

if "$BIN_PATH" --title "Only Title" 2>/dev/null; then
    echo "FAIL: Expected failure on missing required args" >&2
    exit 1
fi
echo "PASS: Missing argument validation verified"

TEST_TITLE="Refactor background indexing"
TEST_REPO="$REPO_ROOT"
TEST_BRIEF="Run long-running analysis across worktrees"
TEST_THREAD="discord_th_998877"

OUTPUT=$("$BIN_PATH" \
    --title "$TEST_TITLE" \
    --repo "$TEST_REPO" \
    --brief "$TEST_BRIEF" \
    --thread-id "$TEST_THREAD" \
    --db "$TEST_DB")

echo "Output from CLI: $OUTPUT"

if ! echo "$OUTPUT" | grep -q "✅ 작업 위임이 접수되었습니다."; then
    echo "FAIL: Output does not contain confirmation prefix" >&2
    exit 1
fi

WORK_ITEM_ID=$(echo "$OUTPUT" | sed -n 's/.*(ID: \([^,]*\), Pane: .*/\1/p')
DELEGATE_PANE=$(echo "$OUTPUT" | sed -n 's/.*Pane: \([^)]*\)).*/\1/p')

if [[ -z "$WORK_ITEM_ID" || -z "$DELEGATE_PANE" ]]; then
    echo "FAIL: Failed to parse ID or Pane from output" >&2
    exit 1
fi

echo "Parsed WORK_ITEM_ID: $WORK_ITEM_ID"
echo "Parsed DELEGATE_PANE: $DELEGATE_PANE"
echo "PASS: CLI receipt output verified"

if [[ ! -f "$TEST_DB" ]]; then
    echo "FAIL: Test DB $TEST_DB not created" >&2
    exit 1
fi

DB_ROW=$(sqlite3 "$TEST_DB" "SELECT id, title, thread_id, delegate_pane, status FROM work_items WHERE id = '$WORK_ITEM_ID';")

if [[ -z "$DB_ROW" ]]; then
    echo "FAIL: No row found in work_items for ID $WORK_ITEM_ID" >&2
    exit 1
fi

echo "DB row: $DB_ROW"

EXPECTED_ROW="${WORK_ITEM_ID}|${TEST_TITLE}|${TEST_THREAD}|${DELEGATE_PANE}|in_progress"
if [[ "$DB_ROW" != "$EXPECTED_ROW" ]]; then
    echo "FAIL: DB row mismatch." >&2
    echo "Expected: $EXPECTED_ROW" >&2
    echo "Actual:   $DB_ROW" >&2
    exit 1
fi

LAST_PROGRESS=$(sqlite3 "$TEST_DB" "SELECT last_progress_at FROM work_items WHERE id = '$WORK_ITEM_ID';")
if [[ -z "$LAST_PROGRESS" ]]; then
    echo "FAIL: last_progress_at is empty in DB" >&2
    exit 1
fi

echo "PASS: SQLite database insertion and row verified"

TEST_DB_URI="sqlite://${TEST_TMP_DIR}/uri_gateway.db"
OUTPUT_URI=$("$BIN_PATH" \
    --title "URI test" \
    --repo "$TEST_REPO" \
    --brief "brief" \
    --db "$TEST_DB_URI")

if ! echo "$OUTPUT_URI" | grep -q "✅ 작업 위임이 접수되었습니다."; then
    echo "FAIL: sqlite:// URI parsing failed" >&2
    exit 1
fi

URI_ID=$(echo "$OUTPUT_URI" | sed -n 's/.*(ID: \([^,]*\), Pane: .*/\1/p')
URI_ROW=$(sqlite3 "${TEST_TMP_DIR}/uri_gateway.db" "SELECT status FROM work_items WHERE id = '$URI_ID';")
if [[ "$URI_ROW" != "in_progress" ]]; then
    echo "FAIL: sqlite:// URI DB row verification failed" >&2
    exit 1
fi
echo "PASS: sqlite:// URL format compatibility verified"

echo "=== All integration tests PASSED successfully ==="

#!/bin/bash
# Single integration gate, run once after every fix is merged.
# Usage: gate.sh <run-name>   -> logs in gate/<run-name>/, summary in gate/<run-name>/SUMMARY.txt
set -u
REPO=/Users/indo/code/project/omon-gateway
OUT="$REPO/.omo/evidence/review-20261014/gate/${1:-run1}"
mkdir -p "$OUT"
cd "$REPO" || exit 2
export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-$REPO/target-w4}"
step() {
  local name=$1; shift
  local start=$(date +%s)
  "$@" >"$OUT/$name.log" 2>&1
  local rc=$?
  echo "$name exit=$rc secs=$(( $(date +%s) - start ))" | tee -a "$OUT/SUMMARY.txt"
  return $rc
}
: >"$OUT/SUMMARY.txt"
git rev-parse HEAD >>"$OUT/SUMMARY.txt"
step fmt cargo fmt --all -- --check
step clippy cargo clippy --all-targets --all-features -- -D warnings
step test cargo test --all-features --no-fail-fast
if git diff --name-only HEAD | grep -q '^web/src/'; then
  (cd web && step web-test bun test && step web-tsc bunx tsc -b && step web-build bunx vite build)
else
  echo "web: no web/src changes, skipped" | tee -a "$OUT/SUMMARY.txt"
fi
grep -q 'exit=[1-9]' "$OUT/SUMMARY.txt" && { echo GATE-FAIL | tee -a "$OUT/SUMMARY.txt"; exit 1; }
echo GATE-PASS | tee -a "$OUT/SUMMARY.txt"

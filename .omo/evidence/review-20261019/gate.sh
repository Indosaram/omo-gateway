#!/bin/bash
# Single integration gate, run once after every fix is merged.
# Usage: gate.sh <run-name>   -> logs in gate/<run-name>/, summary in gate/<run-name>/SUMMARY.txt
set -u
REPO=/Users/indo/code/project/omon-gateway
OUT="$REPO/.omo/evidence/review-20261019/gate/${1:-run1}"
mkdir -p "$OUT"
cd "$REPO" || exit 2
export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-$REPO/target}"
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
(cd web && step web-test-raw bun test; step web-tsc bunx tsc -b; step web-build bunx vite build)
# Pre-existing (identical on e57b472, see web-test-preexisting-e57b472.txt): bun cannot resolve the
# @/ and @hermes/shared aliases in 4 files. Pass when nothing beyond those 4 fails.
fails=$(grep -Eo '^ *[0-9]+ fail' "$OUT/web-test-raw.log" | grep -Eo '[0-9]+' || echo 0)
aliasfails=$(grep -c "Cannot find module .@" "$OUT/web-test-raw.log" || true)
if [[ "$fails" -le 4 && "$fails" -eq "$aliasfails" ]] && grep -q 'web-test-raw exit=' "$OUT/SUMMARY.txt"; then
  sed -i '' 's/^web-test-raw exit=[0-9]*/web-test-raw exit=PREEXISTING('"$fails"')/' "$OUT/SUMMARY.txt"
fi
grep -q 'exit=[1-9]' "$OUT/SUMMARY.txt" && { echo GATE-FAIL | tee -a "$OUT/SUMMARY.txt"; exit 1; }
echo GATE-PASS | tee -a "$OUT/SUMMARY.txt"

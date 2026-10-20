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
# No alias exemption: bun test must exit 0. web/bunfig.toml preloads
# web/test-preload.ts (adds vitest's vi.stubGlobal/vi.unstubAllGlobals to bun's
# compat layer) and the web tsconfig.json "paths" resolve @/, so unresolved
# imports are real failures. Record the counts as evidence either way.
{
  echo "web-test-fails=$(grep -Eo '^ *[0-9]+ fail' "$OUT/web-test-raw.log" | grep -Eo '[0-9]+' | tail -1 || echo 0)"
  echo "web-test-passes=$(grep -Eo '^ *[0-9]+ pass' "$OUT/web-test-raw.log" | grep -Eo '[0-9]+' | tail -1 || echo 0)"
  echo "web-test-unresolved-imports=$(grep -c 'Cannot find module' "$OUT/web-test-raw.log" || true)"
} >>"$OUT/SUMMARY.txt"
grep -q 'exit=[1-9]' "$OUT/SUMMARY.txt" && { echo GATE-FAIL | tee -a "$OUT/SUMMARY.txt"; exit 1; }
echo GATE-PASS | tee -a "$OUT/SUMMARY.txt"

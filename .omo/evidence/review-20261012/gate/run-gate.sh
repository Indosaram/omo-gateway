#!/bin/bash
cd /Users/indo/code/project/omon-gateway || exit 1
G=.omo/evidence/review-20261012/gate
mkdir -p "$G"
: > "$G/summary.txt"

step() {
  local name="$1"; shift
  "$@" > "$G/$name.log" 2>&1
  local code=$?
  echo "$name EXIT=$code" >> "$G/summary.txt"
}

step fmt cargo fmt --check
step clippy cargo clippy --all-targets --all-features -- -D warnings
step test cargo test --all-features --no-fail-fast
step web-test bash -c 'cd web && bun test src/__tests__'
step web-tsc bash -c 'cd web && npx tsc -b'
step web-build bash -c 'cd web && npx vite build'

echo GATE_DONE >> "$G/summary.txt"
printf 'GATE_DONE\n'

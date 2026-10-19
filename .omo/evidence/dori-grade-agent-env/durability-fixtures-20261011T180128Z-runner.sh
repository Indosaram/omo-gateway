#!/bin/bash
set -u
cd /Users/indo/code/project/omon-gateway-wt/dori-work || exit 2
P=.omo/evidence/dori-grade-agent-env/durability-fixtures-20261011T180128Z
export CARGO_TARGET_DIR="$PWD/target-w4"
manifest() { while IFS= read -r line; do shasum -a 256 "${line:66}" || return 2; done < "$P-pre-manifest.sha256"; }
(set -o noclobber; printf 'pid=%s\n' "$$" > "$P-runner.started") || exit 2
manifest > "$P-timeout-post-manifest.sha256" || exit 2
cmp -s "$P-pre-manifest.sha256" "$P-timeout-post-manifest.sha256" || exit 2
S=$(mktemp -d -t dori_runner_XXXXXX) || exit 2
export TMPDIR="$S"
printf 'scratch=%s\n' "$S" > "$P-runner-scratch.txt"
printf 'START runner pid=%s\n' "$$"
cargo test --offline -j2 --all-features --test test_lane_runner --no-fail-fast -- --nocapture > "$P-runner.log" 2>&1; code=$?
printf 'runner\t%s\tcargo test --offline -j2 --all-features --test test_lane_runner --no-fail-fast -- --nocapture\n' "$code" >> "$P-status.tsv"
manifest > "$P-runner-post-manifest.sha256" || exit 2
cmp -s "$P-pre-manifest.sha256" "$P-runner-post-manifest.sha256"; same=$?
find "$S" -mindepth 1 -print > "$P-runner-scratch-remaining.txt"
rmdir "$S"; cleanup=$?
printf 'runner=%s\nsource_cmp=%s\ncleanup=%s\npid=%s\n' "$code" "$same" "$cleanup" "$$" > "$P-runner-completed.marker"
printf 'COMPLETE runner=%s source_cmp=%s cleanup=%s\n' "$code" "$same" "$cleanup"

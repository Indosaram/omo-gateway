#!/bin/bash
set -u
set -o pipefail
ROOT=/Users/indo/code/project/omon-gateway-wt/dori-work
cd "$ROOT" || exit 2
E=.omo/evidence/dori-grade-agent-env
P="$E/race-port-20261011T145201Z"
export CARGO_TARGET_DIR="$ROOT/target-w4"
export CARGO_BUILD_JOBS=2
manifest() {
    while IFS= read -r line; do
        shasum -a 256 "${line:66}" || return 2
    done < "$E/guard-repair-20261011T144644Z-post-manifest.sha256"
}
preserved() {
    shasum -a 256 "$E/task-7.log" "$E"/repair-20261011T144053Z-* "$E/repair-20261011T144053Z.sh" "$E"/guard-repair-20261011T144644Z-* "$E/guard-repair-20261011T144644Z.sh"
}
run() {
    local name="$1" status
    shift
    printf 'START %s pid=%s command=%s\n' "$name" "$$" "$*"
    "$@" > "$P-$name.log" 2>&1
    status=$?
    printf '%s\t%s\t%s\n' "$name" "$status" "$*" >> "$P-status.tsv"
    printf 'RESULT %s exit=%s\n' "$name" "$status"
}
if [[ "${1:-}" == scoped ]]; then
    (set -o noclobber; printf 'pid=%s\n' "$$" > "$P-scoped.started") || exit 2
    preserved > "$P-preserved-pre.sha256" || exit 2
    manifest > "$P-pre-manifest.sha256" || exit 2
    run scoped_fmt rustfmt --edition 2021 --check src/lanes/runner.rs tests/test_lane_reopen.rs
    diff -u "$P-before-runner.rs.txt" src/lanes/runner.rs > "$P-runner.diff"
    diff -u "$P-before-reopen.rs.txt" tests/test_lane_reopen.rs > "$P-reopen.diff"
    exit 0
fi
[[ "${1:-}" == gates && -d "$CARGO_TARGET_DIR" && -f "$P-scoped.started" ]] || exit 2
(set -o noclobber; printf 'pid=%s\n' "$$" > "$P-gates.started") || exit 2
SCRATCH=$(mktemp -d -t dori_race_port_XXXXXX) || exit 2
printf 'scratch=%s\n' "$SCRATCH" > "$P-scratch.txt"
run clippy cargo clippy -j2 --all-targets --all-features -- -D warnings
run focused env TMPDIR="$SCRATCH" cargo test -j2 --test test_lane_runner --test test_lane_reopen --no-fail-fast -- --nocapture
find "$SCRATCH" -mindepth 1 -print > "$P-scratch-remaining.txt"
rmdir "$SCRATCH"
cleanup=$?
printf 'cleanup_rmdir_exit=%s\n' "$cleanup" > "$P-summary.txt"
manifest > "$P-post-manifest.sha256" || exit 2
preserved > "$P-preserved-post.sha256" || exit 2
cmp -s "$P-pre-manifest.sha256" "$P-post-manifest.sha256"; source_status=$?
cmp -s "$P-preserved-pre.sha256" "$P-preserved-post.sha256"; preserved_status=$?
printf 'source_manifest_cmp_exit=%s\npreserved_receipts_cmp_exit=%s\n' "$source_status" "$preserved_status" >> "$P-summary.txt"
printf 'COMPLETE source_cmp=%s preserved_cmp=%s cleanup=%s\n' "$source_status" "$preserved_status" "$cleanup"
printf 'completed_pid=%s\n' "$$" > "$P-completed.marker"

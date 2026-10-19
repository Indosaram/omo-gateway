#!/bin/bash
set -u
set -o pipefail
cd /Users/indo/code/project/omon-gateway-wt/dori-work || exit 2
P=.omo/evidence/dori-grade-agent-env/grouped-repairs-20261011T160343Z
export CARGO_TARGET_DIR="$PWD/target-w4"
export CARGO_BUILD_JOBS=2
(set -o noclobber; printf 'pid=%s\n' "$$" > "$P-started.marker") || exit 2
manifest() {
    while IFS= read -r line; do shasum -a 256 "${line:66}" || return 2; done < "$P-pre-manifest.sha256"
}
manifest > "$P-launch-manifest.sha256" || exit 2
cmp -s "$P-pre-manifest.sha256" "$P-launch-manifest.sha256" || exit 2
shasum -a 256 .omo/evidence/dori-grade-agent-env/task-7.log .omo/evidence/dori-grade-agent-env/repair-20261011T144053Z-* .omo/evidence/dori-grade-agent-env/guard-repair-20261011T144644Z-* .omo/evidence/dori-grade-agent-env/race-port-20261011T145201Z-* > "$P-preserved-pre.sha256"
FAILED=0
run() {
    local name="$1" code
    shift
    printf 'START %s pid=%s command=%s\n' "$name" "$$" "$*"
    "$@" > "$P-$name.log" 2>&1
    code=$?
    printf '%s\t%s\t%s\n' "$name" "$code" "$*" >> "$P-status.tsv"
    [[ "$code" == 0 ]] || FAILED=1
    printf 'RESULT %s exit=%s\n' "$name" "$code"
}
SCRATCH=$(mktemp -d -t dori_grouped_XXXXXX) || exit 2
export TMPDIR="$SCRATCH"
printf 'scratch=%s\n' "$SCRATCH" > "$P-scratch.txt"
run fmt cargo fmt --check
run clippy cargo clippy -j2 --all-targets --all-features -- -D warnings
run lib_scrub cargo test -j2 --all-features --lib creation_metadata --no-fail-fast -- --nocapture
run entry_session cargo test -j2 --all-features --bin omo-gateway session_cli_output_preserves_aggregate_exit_and_projection --no-fail-fast -- --nocapture
run entry_cli cargo test -j2 --all-features --bin omo-gateway tests::cli_ --no-fail-fast -- --nocapture
run focused cargo test -j2 --all-features --test test_lane_runner --test test_lane_reopen --test test_lane_events --test test_session_registry --no-fail-fast -- --nocapture
manifest > "$P-post-manifest.sha256" || exit 2
cmp -s "$P-pre-manifest.sha256" "$P-post-manifest.sha256"; same=$?
[[ "$same" == 0 ]] || FAILED=1
shasum -a 256 .omo/evidence/dori-grade-agent-env/task-7.log .omo/evidence/dori-grade-agent-env/repair-20261011T144053Z-* .omo/evidence/dori-grade-agent-env/guard-repair-20261011T144644Z-* .omo/evidence/dori-grade-agent-env/race-port-20261011T145201Z-* > "$P-preserved-post.sha256"
cmp -s "$P-preserved-pre.sha256" "$P-preserved-post.sha256"; kept=$?
find "$SCRATCH" -mindepth 1 -print > "$P-scratch-remaining.txt"
rmdir "$SCRATCH"; cleanup=$?
printf 'checks_failed=%s\nsource_cmp_exit=%s\npreserved_cmp_exit=%s\ncleanup_exit=%s\n' "$FAILED" "$same" "$kept" "$cleanup" > "$P-summary.txt"
printf 'completed_pid=%s\n' "$$" > "$P-completed.marker"
printf 'COMPLETE checks_failed=%s source_cmp=%s preserved_cmp=%s cleanup=%s\n' "$FAILED" "$same" "$kept" "$cleanup"

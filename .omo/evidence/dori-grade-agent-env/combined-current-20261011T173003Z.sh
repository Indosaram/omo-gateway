#!/bin/bash
set -u
set -o pipefail
cd /Users/indo/code/project/omon-gateway-wt/dori-work || exit 2
P=.omo/evidence/dori-grade-agent-env/combined-current-20261011T173003Z
export CARGO_TARGET_DIR="$PWD/target-w4"
export CARGO_BUILD_JOBS=2
manifest() { while IFS= read -r line; do shasum -a 256 "${line:66}" || return 2; done < "$P-pre-manifest.sha256"; }
run() {
    local name="$1" code
    shift
    printf 'START %s pid=%s command=%s\n' "$name" "$$" "$*"
    "$@" > "$P-$name.log" 2>&1; code=$?
    printf '%s\t%s\t%s\n' "$name" "$code" "$*" >> "$P-status.tsv"
    printf 'RESULT %s exit=%s\n' "$name" "$code"
    return "$code"
}
if [[ "${1:-}" == gates ]]; then
    (set -o noclobber; printf 'pid=%s\n' "$$" > "$P-started.marker") || exit 2
    manifest > "$P-launch-manifest.sha256" || exit 2
    cmp -s "$P-pre-manifest.sha256" "$P-launch-manifest.sha256" || exit 2
    S=$(mktemp -d -t dori_combined_XXXXXX) || exit 2
    export TMPDIR="$S"
    printf 'scratch=%s\n' "$S" > "$P-scratch.txt"
    run fmt cargo fmt --check; f=$?
    run clippy cargo clippy --offline -j2 --all-targets --all-features -- -D warnings; c=$?
    run headless cargo test --offline -j2 --all-features --bin omo-gateway headless_tests --no-fail-fast -- --nocapture; h=$?
    run focused cargo test --offline -j2 --all-features --test test_lane_dispatch --test test_inbox_sources --test test_lane_thread_transport --test test_lane_post_transport --test test_lane_events --no-fail-fast -- --nocapture; t=$?
    manifest > "$P-post-manifest.sha256" || exit 2
    cmp -s "$P-pre-manifest.sha256" "$P-post-manifest.sha256"; same=$?
    find "$S" -mindepth 1 -print > "$P-scratch-remaining.txt"
    rmdir "$S"; cleanup=$?
    printf 'fmt=%s\nclippy=%s\nheadless=%s\nfocused=%s\nsource_cmp=%s\ncleanup=%s\n' "$f" "$c" "$h" "$t" "$same" "$cleanup" > "$P-summary.txt"
    if [[ "$f$c$h$t$same" == 00000 ]]; then printf 'passed\n' > "$P-passed.marker"; fi
    printf 'completed_pid=%s\n' "$$" > "$P-completed.marker"
    printf 'COMPLETE fmt=%s clippy=%s headless=%s focused=%s source_cmp=%s cleanup=%s\n' "$f" "$c" "$h" "$t" "$same" "$cleanup"
else
    [[ "${1:-}" == build && -f "$P-passed.marker" ]] || exit 2
    (set -o noclobber; printf 'pid=%s\n' "$$" > "$P-build.started") || exit 2
    manifest > "$P-build-pre-manifest.sha256" || exit 2
    cmp -s "$P-post-manifest.sha256" "$P-build-pre-manifest.sha256" || exit 2
    run build cargo build --offline -j2 --bin omo-gateway; code=$?
    manifest > "$P-build-post-manifest.sha256" || exit 2
    cmp -s "$P-build-pre-manifest.sha256" "$P-build-post-manifest.sha256"; same=$?
    if [[ "$code$same" == 00 ]]; then shasum -a 256 target-w4/debug/omo-gateway > "$P-binary.sha256"; fi
    printf 'build=%s\nsource_cmp=%s\n' "$code" "$same" > "$P-build-completed.marker"
    printf 'COMPLETE build=%s source_cmp=%s\n' "$code" "$same"
fi

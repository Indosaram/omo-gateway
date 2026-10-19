#!/bin/bash
set -u
set -o pipefail
cd /Users/indo/code/project/omon-gateway-wt/dori-work || exit 2
P=.omo/evidence/dori-grade-agent-env/fixture-repair-20261011T161103Z
G=.omo/evidence/dori-grade-agent-env/grouped-repairs-20261011T160343Z
export CARGO_TARGET_DIR="$PWD/target-w4"
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
    shasum -a 256 "$G"-* > "$P-preserved-pre.sha256"
    manifest > "$P-launch-manifest.sha256" || exit 2
    cmp -s "$P-pre-manifest.sha256" "$P-launch-manifest.sha256" || exit 2
    S=$(mktemp -d -t dori_fixture_XXXXXX) || exit 2
    export TMPDIR="$S"
    printf 'scratch=%s\n' "$S" > "$P-scratch.txt"
    run fmt rustfmt --edition 2021 --check tests/test_lane_runner.rs; f=$?
    run clippy cargo clippy -j2 --all-targets --all-features -- -D warnings; c=$?
    run runner cargo test -j2 --all-features --test test_lane_runner --no-fail-fast -- --nocapture; t=$?
    manifest > "$P-post-manifest.sha256" || exit 2
    cmp -s "$P-pre-manifest.sha256" "$P-post-manifest.sha256"; same=$?
    shasum -a 256 "$G"-* > "$P-preserved-post.sha256"
    cmp -s "$P-preserved-pre.sha256" "$P-preserved-post.sha256"; kept=$?
    find "$S" -mindepth 1 -print > "$P-scratch-remaining.txt"
    rmdir "$S"; cleanup=$?
    printf 'fmt=%s\nclippy=%s\nrunner=%s\nsource_cmp=%s\npreserved_cmp=%s\ncleanup=%s\n' "$f" "$c" "$t" "$same" "$kept" "$cleanup" > "$P-summary.txt"
    if [[ "$f$c$t$same$kept$cleanup" == 000000 ]]; then printf 'passed\n' > "$P-passed.marker"; fi
    printf 'completed_pid=%s\n' "$$" > "$P-completed.marker"
    printf 'COMPLETE fmt=%s clippy=%s runner=%s source_cmp=%s cleanup=%s\n' "$f" "$c" "$t" "$same" "$cleanup"
else
    [[ "${1:-}" == build && -f "$P-passed.marker" ]] || exit 2
    (set -o noclobber; printf 'pid=%s\n' "$$" > "$P-build.started") || exit 2
    manifest > "$P-build-pre-manifest.sha256" || exit 2
    cmp -s "$P-post-manifest.sha256" "$P-build-pre-manifest.sha256" || exit 2
    run build cargo build -j2 --bin omo-gateway; code=$?
    manifest > "$P-build-post-manifest.sha256" || exit 2
    cmp -s "$P-build-pre-manifest.sha256" "$P-build-post-manifest.sha256"; same=$?
    if [[ "$code$same" == 00 ]]; then shasum -a 256 target-w4/debug/omo-gateway > "$P-binary.sha256"; fi
    printf 'build=%s\nsource_cmp=%s\n' "$code" "$same" > "$P-build-completed.marker"
    printf 'COMPLETE build=%s source_cmp=%s\n' "$code" "$same"
fi

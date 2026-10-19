#!/bin/bash
set -u
set -o pipefail
cd /Users/indo/code/project/omon-gateway-wt/dori-work || exit 2
P=.omo/evidence/dori-grade-agent-env/wrapper-gate-20261011T173919Z
G=.omo/evidence/dori-grade-agent-env/combined-current-20261011T173003Z
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
(set -o noclobber; printf 'pid=%s\n' "$$" > "$P-started.marker") || exit 2
shasum -a 256 "$G"-* > "$P-preserved-pre.sha256"
manifest > "$P-launch-manifest.sha256" || exit 2
cmp -s "$P-pre-manifest.sha256" "$P-launch-manifest.sha256" || exit 2
run fmt cargo fmt --check; f=$?
run clippy cargo clippy --offline -j2 --all-targets --all-features -- -D warnings; c=$?
manifest > "$P-gate-post-manifest.sha256" || exit 2
cmp -s "$P-pre-manifest.sha256" "$P-gate-post-manifest.sha256"; same=$?
printf 'fmt=%s\nclippy=%s\nsource_cmp=%s\n' "$f" "$c" "$same" > "$P-gate-summary.txt"
printf 'GATE_COMPLETE fmt=%s clippy=%s source_cmp=%s\n' "$f" "$c" "$same"
if [[ "$f$c$same" == 000 ]]; then
    printf 'pid=%s\n' "$$" > "$P-build.started"
    run build cargo build --offline -j2 --bin omo-gateway; b=$?
    manifest > "$P-build-post-manifest.sha256" || exit 2
    cmp -s "$P-pre-manifest.sha256" "$P-build-post-manifest.sha256"; bs=$?
    if [[ "$b$bs" == 00 ]]; then shasum -a 256 target-w4/debug/omo-gateway > "$P-binary.sha256"; fi
else
    b=not_run; bs=not_run
fi
shasum -a 256 "$G"-* > "$P-preserved-post.sha256"
cmp -s "$P-preserved-pre.sha256" "$P-preserved-post.sha256"; kept=$?
printf 'fmt=%s\nclippy=%s\nsource_cmp=%s\nbuild=%s\nbuild_source_cmp=%s\npreserved_cmp=%s\ncompleted_pid=%s\n' "$f" "$c" "$same" "$b" "$bs" "$kept" "$$" > "$P-completed.marker"
printf 'COMPLETE fmt=%s clippy=%s source_cmp=%s build=%s build_source_cmp=%s preserved_cmp=%s\n' "$f" "$c" "$same" "$b" "$bs" "$kept"

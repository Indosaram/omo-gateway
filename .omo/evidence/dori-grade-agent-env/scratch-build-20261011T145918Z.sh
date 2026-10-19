#!/bin/bash
set -u
set -o pipefail
ROOT=/Users/indo/code/project/omon-gateway-wt/dori-work
cd "$ROOT" || exit 2
E=.omo/evidence/dori-grade-agent-env
P="$E/scratch-build-20261011T145918Z"
G="$E/race-port-20261011T145201Z"
export CARGO_TARGET_DIR="$ROOT/target-w4"
export CARGO_BUILD_JOBS=2
[[ -f "$G-completed.marker" && -d "$CARGO_TARGET_DIR" ]] || exit 2
(set -o noclobber; printf 'pid=%s\n' "$$" > "$P-started.marker") || exit 2
manifest() {
    while IFS= read -r line; do
        shasum -a 256 "${line:66}" || return 2
    done < "$G-post-manifest.sha256"
}
manifest > "$P-pre-manifest.sha256" || exit 2
cmp -s "$G-post-manifest.sha256" "$P-pre-manifest.sha256" || { printf 'SOURCE_CONFLICT\n'; exit 2; }
shasum -a 256 "$G-post-manifest.sha256" "$G-status.tsv" "$G-focused.log" "$G-clippy.log" > "$P-gate-artifacts.sha256"
printf 'START build pid=%s command=cargo build -j2 --bin omo-gateway\n' "$$"
cargo build -j2 --bin omo-gateway > "$P-build.log" 2>&1
status=$?
printf 'build\t%s\tcargo build -j2 --bin omo-gateway\n' "$status" > "$P-status.tsv"
printf 'RESULT build exit=%s\n' "$status"
manifest > "$P-post-manifest.sha256" || exit 2
cmp -s "$P-pre-manifest.sha256" "$P-post-manifest.sha256"; source_status=$?
printf 'source_manifest_cmp_exit=%s\nprepared_root=/private/tmp/omon-lane-scratch-qa-pibpnv\nruntime_released=false\n' "$source_status" > "$P-summary.txt"
if [[ "$status" == 0 && "$source_status" == 0 ]]; then
    shasum -a 256 "$CARGO_TARGET_DIR/debug/omo-gateway" > "$P-binary.sha256"
fi
printf 'completed_pid=%s\nbuild_exit=%s\nsource_cmp_exit=%s\n' "$$" "$status" "$source_status" > "$P-completed.marker"
printf 'COMPLETE build=%s source_cmp=%s runtime_released=false\n' "$status" "$source_status"

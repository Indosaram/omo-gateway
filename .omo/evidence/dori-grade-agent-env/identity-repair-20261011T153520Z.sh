#!/bin/bash
set -u
set -o pipefail
cd /Users/indo/code/project/omon-gateway-wt/dori-work || exit 2
P=.omo/evidence/dori-grade-agent-env/identity-repair-20261011T153520Z
export CARGO_TARGET_DIR="$PWD/target-w4"
(set -o noclobber; printf 'pid=%s\n' "$$" > "$P-started.marker") || exit 2
manifest() {
    while IFS= read -r line; do shasum -a 256 "${line:66}" || return 2; done < "$P-manifest.sha256"
}
manifest > "$P-pre-manifest.sha256" || exit 2
cmp -s "$P-manifest.sha256" "$P-pre-manifest.sha256" || exit 2
shasum -a 256 target-w4/debug/omo-gateway > "$P-before-binary.sha256"
printf 'START build pid=%s\n' "$$"
cargo build -j2 --bin omo-gateway > "$P-build.log" 2>&1
code=$?
printf 'build\t%s\tcargo build -j2 --bin omo-gateway\n' "$code" > "$P-status.tsv"
printf 'RESULT build exit=%s\n' "$code"
manifest > "$P-post-manifest.sha256" || exit 2
cmp -s "$P-pre-manifest.sha256" "$P-post-manifest.sha256"; same=$?
shasum -a 256 target-w4/debug/omo-gateway > "$P-binary.sha256"
printf 'build_exit=%s\nsource_cmp_exit=%s\n' "$code" "$same" > "$P-completed.marker"
printf 'COMPLETE build=%s source_cmp=%s\n' "$code" "$same"

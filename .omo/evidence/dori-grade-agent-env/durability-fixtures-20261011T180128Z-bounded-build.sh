#!/bin/bash
set -u
cd /Users/indo/code/project/omon-gateway-wt/dori-work || exit 2
P=.omo/evidence/dori-grade-agent-env/durability-fixtures-20261011T180128Z
export CARGO_TARGET_DIR="$PWD/target-w4"
manifest() { while IFS= read -r line; do shasum -a 256 "${line:66}" || return 2; done < "$P-pre-manifest.sha256"; }
(set -o noclobber; printf 'pid=%s policy=baseline-bounded-not-all-green\n' "$$" > "$P-build.started") || exit 2
manifest > "$P-build-pre-manifest.sha256" || exit 2
cmp -s "$P-pre-manifest.sha256" "$P-build-pre-manifest.sha256" || exit 2
printf 'START build pid=%s\n' "$$"
cargo build --offline -j2 --bin omo-gateway > "$P-build.log" 2>&1; code=$?
printf 'build\t%s\tcargo build --offline -j2 --bin omo-gateway\n' "$code" >> "$P-status.tsv"
manifest > "$P-build-post-manifest.sha256" || exit 2
cmp -s "$P-build-pre-manifest.sha256" "$P-build-post-manifest.sha256"; same=$?
if [[ "$code$same" == 00 ]]; then shasum -a 256 target-w4/debug/omo-gateway > "$P-binary.sha256"; fi
printf 'build=%s\nsource_cmp=%s\npolicy=baseline-bounded-not-all-green\n' "$code" "$same" > "$P-build-completed.marker"
printf 'COMPLETE build=%s source_cmp=%s\n' "$code" "$same"

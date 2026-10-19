#!/bin/bash
set -u
cd /Users/indo/code/project/omon-gateway-wt/dori-work || exit 2
P=.omo/evidence/dori-grade-agent-env/duplicate-final-20261011T184722Z
export CARGO_TARGET_DIR="$PWD/target-w4"
manifest() { while IFS= read -r line; do shasum -a 256 "${line:66}" || return 2; done < "$P-pre-manifest.sha256"; }
[[ -f "$P-passed.marker" && -f "$P-completed.marker" ]] || exit 2
(set -o noclobber; printf 'pid=%s\n' "$$" > "$P-recovery.started") || exit 2
manifest > "$P-recovery-pre-manifest.sha256" || exit 2
cmp -s "$P-post-manifest.sha256" "$P-recovery-pre-manifest.sha256" || exit 2
S=$(mktemp -d -t dori_build_recovery_XXXXXX) || exit 2
export TMPDIR="$S"
printf 'START recovery pid=%s\n' "$$"
cargo build --offline -j2 --bin omo-gateway > "$P-build-recovery.log" 2>&1; code=$?
printf 'build_recovery\t%s\tcargo build --offline -j2 --bin omo-gateway\n' "$code" >> "$P-status.tsv"
manifest > "$P-recovery-post-manifest.sha256" || exit 2
cmp -s "$P-recovery-pre-manifest.sha256" "$P-recovery-post-manifest.sha256"; same=$?
if [[ "$code$same" == 00 ]]; then shasum -a 256 target-w4/debug/omo-gateway > "$P-recovery-binary.sha256"; fi
rmdir "$S"; cleanup=$?
printf 'build=%s\nsource_cmp=%s\ncleanup=%s\n' "$code" "$same" "$cleanup" > "$P-recovery-completed.marker"
printf 'COMPLETE recovery=%s source_cmp=%s cleanup=%s\n' "$code" "$same" "$cleanup"

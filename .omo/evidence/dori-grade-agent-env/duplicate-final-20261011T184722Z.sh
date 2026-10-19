#!/bin/bash
set -u
cd /Users/indo/code/project/omon-gateway-wt/dori-work || exit 2
P=.omo/evidence/dori-grade-agent-env/duplicate-final-20261011T184722Z
export CARGO_TARGET_DIR="$PWD/target-w4"
manifest() { while IFS= read -r line; do shasum -a 256 "${line:66}" || return 2; done < "$P-pre-manifest.sha256"; }
run() { local name="$1" code; shift; printf 'START %s pid=%s\n' "$name" "$$"; "$@" > "$P-$name.log" 2>&1; code=$?; printf '%s\t%s\t%s\n' "$name" "$code" "$*" >> "$P-status.tsv"; printf 'RESULT %s exit=%s\n' "$name" "$code"; return "$code"; }
(set -o noclobber; printf 'pid=%s\n' "$$" > "$P-started.marker") || exit 2
manifest > "$P-launch-manifest.sha256" || exit 2
cmp -s "$P-pre-manifest.sha256" "$P-launch-manifest.sha256" || exit 2
S=$(mktemp -d -t dori_duplicate_XXXXXX) || exit 2
export TMPDIR="$S"
printf 'scratch=%s\n' "$S" > "$P-scratch.txt"
run fmt cargo fmt --check; f=$?
run clippy cargo clippy --offline -j2 --all-targets --all-features -- -D warnings; c=$?
run completed cargo test --offline -j2 --all-features --test test_omo_backend completed_ --no-fail-fast -- --nocapture; t=$?
run delivery cargo test --offline -j2 --all-features --test test_omo_backend failed_final_delivery_is_not_acknowledged --no-fail-fast -- --nocapture; d=$?
run headless cargo test --offline -j2 --all-features --bin omo-gateway headless_tests --no-fail-fast -- --nocapture; h=$?
run lanes cargo test --offline -j2 --all-features --test test_lane_events --test test_lane_runner --no-fail-fast -- --nocapture; l=$?
manifest > "$P-post-manifest.sha256" || exit 2
cmp -s "$P-pre-manifest.sha256" "$P-post-manifest.sha256"; same=$?
find "$S" -mindepth 1 -print > "$P-scratch-remaining.txt"
rmdir "$S"; cleanup=$?
printf 'fmt=%s\nclippy=%s\ncompleted=%s\ndelivery=%s\nheadless=%s\nlanes=%s\nsource_cmp=%s\ncleanup=%s\n' "$f" "$c" "$t" "$d" "$h" "$l" "$same" "$cleanup" > "$P-summary.txt"
if [[ "$f$c$t$d$h$l$same" == 0000000 ]]; then
  printf 'all scoped gates passed\n' > "$P-passed.marker"
  manifest > "$P-build-pre-manifest.sha256" || exit 2
  cmp -s "$P-post-manifest.sha256" "$P-build-pre-manifest.sha256" || exit 2
  run build cargo build --offline -j2 --bin omo-gateway; b=$?
  manifest > "$P-build-post-manifest.sha256" || exit 2
  cmp -s "$P-build-pre-manifest.sha256" "$P-build-post-manifest.sha256"; bs=$?
  if [[ "$b$bs" == 00 ]]; then shasum -a 256 target-w4/debug/omo-gateway > "$P-binary.sha256"; fi
  printf 'build=%s\nsource_cmp=%s\n' "$b" "$bs" > "$P-build-completed.marker"
else b=not_run; bs=not_run; fi
printf 'pid=%s\n' "$$" > "$P-completed.marker"
printf 'COMPLETE fmt=%s clippy=%s completed=%s delivery=%s headless=%s lanes=%s source_cmp=%s cleanup=%s build=%s build_cmp=%s\n' "$f" "$c" "$t" "$d" "$h" "$l" "$same" "$cleanup" "$b" "$bs"

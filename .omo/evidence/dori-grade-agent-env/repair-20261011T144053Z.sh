#!/bin/bash
set -u
set -o pipefail
ROOT=/Users/indo/code/project/omon-gateway-wt/dori-work
cd "$ROOT" || exit 2
E=.omo/evidence/dori-grade-agent-env
P="$E/repair-20261011T144053Z"
export CARGO_TARGET_DIR="$ROOT/target-w4"
export CARGO_BUILD_JOBS=2
manifest() {
    while IFS= read -r line; do
        path="${line:66}"
        shasum -a 256 "$path" || return 2
    done < "$E/batch-20261011T135508Z-post-manifest.sha256"
}
run() {
    local name="$1" log="$2" status
    shift 2
    printf 'START %s pid=%s command=%s\n' "$name" "$$" "$*"
    "$@" > "$log" 2>&1
    status=$?
    printf '%s\t%s\t%s\n' "$name" "$status" "$*" >> "$P-status.tsv"
    printf 'RESULT %s exit=%s log=%s\n' "$name" "$status" "$log"
}
if [[ "${1:-}" == scoped ]]; then
    [[ ! -e "$P-scoped.started" ]] || { printf 'REFUSE duplicate scoped run\n'; exit 2; }
    (set -o noclobber; printf 'pid=%s\n' "$$" > "$P-scoped.started") || exit 2
    shasum -a 256 "$E"/batch-20261011T135508Z-* > "$P-original-evidence-pre.sha256"
    shasum -a 256 src/agent/backend.rs tests/test_inbox_sources.rs tests/test_omomeow_delegate.sh > "$P-modified.sha256"
    manifest > "$P-pre-manifest.sha256" || exit 2
    run scoped_fmt "$P-scoped-fmt.log" rustfmt --edition 2021 --check src/agent/backend.rs tests/test_inbox_sources.rs
    exit 0
fi
[[ "${1:-}" == gates && -f "$P-scoped.started" && -d "$CARGO_TARGET_DIR" ]] || exit 2
[[ ! -e "$P-gates.started" ]] || { printf 'REFUSE duplicate gates; inspect recorded PID and status\n'; exit 2; }
(set -o noclobber; printf 'pid=%s\n' "$$" > "$P-gates.started") || exit 2
run fmt "$P-fmt.log" cargo fmt --check
run clippy "$P-clippy.log" cargo clippy -j2 --all-targets --all-features -- -D warnings
run state "$E/task-7.log" cargo test --lib lanes::state -j2 -- --nocapture
run inbox "$P-inbox.log" cargo test -j2 --test test_inbox_sources
manifest > "$P-post-manifest.sha256" || exit 2
shasum -a 256 "$E"/batch-20261011T135508Z-* > "$P-original-evidence-post.sha256"
cmp -s "$P-pre-manifest.sha256" "$P-post-manifest.sha256"; source_status=$?
cmp -s "$P-original-evidence-pre.sha256" "$P-original-evidence-post.sha256"; original_status=$?
printf 'source_manifest_cmp_exit=%s\noriginal_evidence_cmp_exit=%s\n' "$source_status" "$original_status" > "$P-summary.txt"
printf 'COMPLETE source_cmp=%s original_cmp=%s\n' "$source_status" "$original_status"
printf 'completed_pid=%s\n' "$$" > "$P-completed.marker"

#!/bin/bash
set -u
set -o pipefail
ROOT="/Users/indo/code/project/omon-gateway-wt/dori-work"
EVIDENCE="$ROOT/.omo/evidence/dori-grade-agent-env"
TARGET="$ROOT/target-w4"
RUN_ID="121136Z-recovery-$(date -u '+%Y%m%dT%H%M%SZ')"
PREFIX="$EVIDENCE/batch-$RUN_ID"
ORIGINAL="$EVIDENCE/batch-20261011T121136Z"
cd "$ROOT" || exit 2
fail() { printf 'ERROR: %s\n' "$*" >&2; exit 2; }
# Refuse active owners; expose only safe target and cwd evidence.
for pid in $(pgrep -x cargo; pgrep -x rustc); do
    args="$(ps eww -p "$pid" -o command= 2>/dev/null || true)"
    cwd="$(lsof -a -p "$pid" -d cwd -Fn 2>/dev/null | awk '/^n/ {print substr($0,2)}')"
    if [[ "$args" == *"CARGO_TARGET_DIR=$TARGET "* || "$args" == *"--out-dir $TARGET/"* || "$args" == *"-C incremental=$TARGET/"* || "$cwd" == "$ROOT" ]]; then
        fail "active or ambiguous build owner pid=$pid cwd=$cwd"
    fi
done
owners="$(lsof -t "$TARGET/debug/.cargo-lock" 2>/dev/null || true)"
[[ -z "$owners" ]] || fail "target lock owner PID(s): $owners"
export CARGO_TARGET_DIR="$TARGET"
export CARGO_BUILD_JOBS=2
manifest() {
    local output="$1"
    {
        find src tests migrations -type f -print0
        if [[ -d .cargo ]]; then find .cargo -type f -print0; fi
        find . -maxdepth 1 -type f \( -name 'Cargo.toml' -o -name 'Cargo.lock' -o -name 'build.rs' -o -name 'rust-toolchain' -o -name 'rust-toolchain.toml' \) -print0
        find . -maxdepth 2 -type f \( -name '*.toml' -o -name '*.yaml' -o -name '*.yml' \) -not -path './target-w4/*' -not -path './.git/*' -print0
        if [[ -d bot_profiles ]]; then find bot_profiles -type f -print0; fi
        if [[ -d profiles ]]; then find profiles -type f -print0; fi
        if [[ -d blocks ]]; then find blocks -type f -print0; fi
        if [[ -d .omo ]]; then
            find .omo -path './.omo/evidence' -prune -o -type f \( -iname '*profile*' -o -iname '*persona*' -o -iname '*bot*.json' -o -iname '*bot*.yaml' -o -iname '*bot*.yml' \) -print0
        fi
        if [[ -d .omo/evidence/dori-grade-agent-env ]]; then
            find .omo/evidence/dori-grade-agent-env -maxdepth 1 -type f -name '*.ndjson' -print0
        fi
        [[ ! -f docs/AGENTS_DELEGATE.md ]] || printf '%s\0' docs/AGENTS_DELEGATE.md
        find . -maxdepth 1 -type f \( -name '*persona*' -o -name '*profile*' \) -print0
    } | sort -zu | while IFS= read -r -d '' path; do
        shasum -a 256 "$path"
    done > "$output"
}


record_provenance() {
    local output="$1"
    {
        printf 'run_id=%s\n' "$RUN_ID"
        printf 'worktree=%s\n' "$ROOT"
        printf 'git_head='; git rev-parse HEAD
        printf 'rustc='; rustc --version --verbose
        printf 'cargo='; cargo --version
        printf 'target=%s\n' "$CARGO_TARGET_DIR"
        printf 'jobs=%s\n' "$CARGO_BUILD_JOBS"
        printf 'cargo_metadata_targets='; cargo metadata --no-deps --format-version 1 | ruby -rjson -e 'puts JSON.parse(STDIN.read).fetch("packages").sum { |p| p.fetch("targets").length }'
        printf 'manifest_policy=src tests migrations .cargo bot_profiles profiles blocks docs/AGENTS_DELEGATE.md root Cargo/build/toolchain and persona/profile inputs plus shallow toml/yaml config, sorted per-file SHA-256\n'
        printf 'safe_environment_names_begin\n'
        env | cut -d= -f1 | grep -E '^(CARGO_|RUST|SQLX_|DATABASE_URL$|OMON_)' | sort -u || true
        printf 'safe_environment_names_end\n'
    } > "$output"
}


manifest "$PREFIX-current-manifest.sha256" || fail "current manifest capture failed"
cmp -s "$ORIGINAL-pre-manifest.sha256" "$PREFIX-current-manifest.sha256" || fail "current inputs differ from original PRE; recovery attribution invalid"
printf 'original_pre_comparison=IDENTICAL\nrecovery_prefix=%s\n' "$PREFIX"
record_provenance "$PREFIX-provenance.txt" || fail "provenance failed"
printf 'original_full_exit=UNKNOWN\noriginal_monitor=mon_PFW3ZRS4C7NRGHAJ\nrecovery_monitor_deadline_ms=1800000\n' >> "$PREFIX-provenance.txt"
: > "$PREFIX-status.tsv"
run_gate() {
    local name="$1"; shift
    local status
    printf '===== %s: %s =====\n' "$name" "$*"
    "$@" > "$PREFIX-$name.log" 2>&1
    status=$?
    printf '%s\t%s\t%s\n' "$name" "$status" "$*" >> "$PREFIX-status.tsv"
    printf 'exit_status=%s\nraw_log=%s\n' "$status" "$PREFIX-$name.log"
}
run_gate full cargo test -j 2 --all-targets --all-features --no-fail-fast
run_gate dashboard cargo test -j 2 --test test_review_dashboard
manifest "$PREFIX-post-manifest.sha256" || fail "recovery post manifest failed"
result=DIFFERENT
cmp -s "$PREFIX-current-manifest.sha256" "$PREFIX-post-manifest.sha256" && result=IDENTICAL
printf 'FINAL manifest_result=%s\nstatus_ledger=%s\nprovenance=%s\n' "$result" "$PREFIX-status.tsv" "$PREFIX-provenance.txt"
[[ "$result" == IDENTICAL ]] || exit 1
awk -F '\t' '$2 != 0 { failed=1 } END { exit failed }' "$PREFIX-status.tsv"

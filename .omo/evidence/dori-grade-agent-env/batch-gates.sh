#!/bin/bash
set -u
set -o pipefail

ROOT="/Users/indo/code/project/omon-gateway-wt/dori-work"
EVIDENCE="$ROOT/.omo/evidence/dori-grade-agent-env"
TARGET="$ROOT/target-w4"
RUN_ID="$(date -u '+%Y%m%dT%H%M%SZ')"
PREFIX="$EVIDENCE/batch-$RUN_ID"
SUMMARY="$PREFIX-summary.txt"
STATUS_FILE="$PREFIX-status.tsv"
WORKTREE="$PREFIX-worktree.txt"

fail() {
    printf 'ERROR: %s\n' "$*" >&2
    exit 2
}

[[ -d "$ROOT" && -d "$EVIDENCE" ]] || fail "expected worktree/evidence directory is missing"
cd "$ROOT" || fail "cannot enter worktree"
[[ -d "$TARGET" ]] || fail "warmed target-w4 is missing; refusing to create a cold target"
[[ ! -e "$SUMMARY" ]] || fail "run id already exists; refusing to overwrite prior evidence"

active_target_builds=""
for pid in $(pgrep -x cargo; pgrep -x rustc); do
    process_args="$(ps eww -p "$pid" -o command= 2>/dev/null || true)"
    if [[ "$process_args" == *"CARGO_TARGET_DIR=$TARGET "* || "$process_args" == *"--out-dir $TARGET/"* || "$process_args" == *"-C incremental=$TARGET/"* ]]; then
        active_target_builds+="pid=$pid $process_args\n"
    fi
done
if [[ -n "$active_target_builds" ]]; then
    printf '%b' "$active_target_builds" >&2
    fail "Cargo/rustc process is active for target-w4; establish target-w4 ownership before running"
fi

for input in Cargo.toml Cargo.lock; do
    [[ -f "$input" ]] || fail "required build input $input is missing"
done

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

git status --short > "$WORKTREE"
record_provenance "$PREFIX-provenance.txt" || fail "could not record provenance"
manifest "$PREFIX-pre-manifest.sha256" || fail "could not create pre-run manifest"
: > "$STATUS_FILE"

run_gate() {
    local name="$1"
    shift
    local log="$PREFIX-$name.log"
    local status
    printf '\n===== %s: %s =====\n' "$name" "$*" | tee -a "$SUMMARY"
    "$@" >"$log" 2>&1
    status=$?
    printf '%s\t%s\t%s\n' "$name" "$status" "$*" >> "$STATUS_FILE"
    printf 'exit_status=%s\nraw_log=%s\n' "$status" "$log" | tee -a "$SUMMARY"
    return 0
}

run_gate fmt cargo fmt --check
run_gate clippy cargo clippy -j 2 --all-targets --all-features -- -D warnings
run_gate full cargo test -j 2 --all-targets --all-features --no-fail-fast
run_gate dashboard cargo test -j 2 --test test_review_dashboard

manifest "$PREFIX-post-manifest.sha256" || fail "could not create post-run manifest"
if cmp -s "$PREFIX-pre-manifest.sha256" "$PREFIX-post-manifest.sha256"; then
    manifest_result="IDENTICAL"
else
    manifest_result="DIFFERENT: frozen-source attribution invalid; preserve gate outputs for triage"
fi

{
    printf '\n===== FINAL SUMMARY =====\n'
    printf 'manifest_result=%s\n' "$manifest_result"
    printf 'pre_manifest=%s\npost_manifest=%s\n' "$PREFIX-pre-manifest.sha256" "$PREFIX-post-manifest.sha256"
    printf 'status_ledger=%s\nprovenance=%s\nworktree_status=%s\n' "$STATUS_FILE" "$PREFIX-provenance.txt" "$WORKTREE"
    while IFS="$(printf '\t')" read -r name status command; do
        printf '%s_exit=%s command=%s\n' "$name" "$status" "$command"
    done < "$STATUS_FILE"
} | tee -a "$SUMMARY"

printf 'Gate collection complete; aggregate failures are evidence, not a stop condition. Summary: %s\n' "$SUMMARY"
failed=0
if awk -F '\t' '$2 != 0 { failed = 1 } END { exit !failed }' "$STATUS_FILE"; then
    failed=1
fi
[[ "$manifest_result" == IDENTICAL ]] || failed=1
exit "$failed"

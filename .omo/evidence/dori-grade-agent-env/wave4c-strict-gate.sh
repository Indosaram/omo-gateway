#!/bin/bash
set -u
set -o pipefail
cd /Users/indo/code/project/omon-gateway-wt/dori-work || exit 1
evidence="$PWD/.omo/evidence/dori-grade-agent-env"
attempt="${1:-1}"
export RUSTC_WRAPPER=
export CARGO_TARGET_DIR="$PWD/target-w4"
export TMPDIR
TMPDIR=$(mktemp -d /tmp/wave4c-strict.XXXXXX) || exit 1
manifest() {
    find src tests migrations bot_profiles -type f -print | LC_ALL=C sort | while IFS= read -r path; do
        shasum -a 256 "$path" || return 1
    done
    shasum -a 256 Cargo.toml Cargo.lock
}
manifest > "$evidence/wave4c-source-pre-$attempt.sha256" || exit 1
cargo fmt --all -- --check 2>&1 | tee "$evidence/wave4c-fmt-$attempt.log"
fmt=$?
printf 'FMT_EXIT=%s\n' "$fmt"
cargo clippy --all-targets --all-features -j4 --offline -- -D warnings 2>&1 | tee "$evidence/wave4c-clippy-$attempt.log"
clippy=$?
printf 'CLIPPY_EXIT=%s\n' "$clippy"
build=not_run
cmp_exit=not_run
if [ "$fmt" = 0 ] && [ "$clippy" = 0 ]; then
    cargo build --bin omo-gateway -j4 --offline 2>&1 | tee "$evidence/wave4c-build-$attempt.log"
    build=$?
    manifest > "$evidence/wave4c-source-post-$attempt.sha256" || exit 1
    cmp "$evidence/wave4c-source-pre-$attempt.sha256" "$evidence/wave4c-source-post-$attempt.sha256"
    cmp_exit=$?
    if [ "$build" = 0 ]; then
        shasum -a 256 "$CARGO_TARGET_DIR/debug/omo-gateway" | tee "$evidence/wave4c-binary-$attempt.sha256"
    fi
fi
printf 'BUILD_EXIT=%s\nSOURCE_CMP=%s\nOWNED_TMPDIR=%s\n' "$build" "$cmp_exit" "$TMPDIR"
printf 'fmt=%s\nclippy=%s\nbuild=%s\nsource_cmp=%s\ntmpdir=%s\n' "$fmt" "$clippy" "$build" "$cmp_exit" "$TMPDIR" > "$evidence/wave4c-strict-$attempt.exits"
[ "$fmt" = 0 ] && [ "$clippy" = 0 ] && [ "$build" = 0 ] && [ "$cmp_exit" = 0 ]

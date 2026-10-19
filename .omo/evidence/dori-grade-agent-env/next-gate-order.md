# Next gate order

Wait for shared repairs and st_01a12ada test_inbox_sources repair settlement before any Cargo invocation.

First run one compile-only gate: CARGO_TARGET_DIR=/Users/indo/code/project/omon-gateway-wt/dori-work/target-w4 CARGO_BUILD_JOBS=2 cargo test -j 2 --all-targets --all-features --no-run. Use tool.monitor with timeout_ms: 1800000 and preserve the raw log and actual exit. Refuse an active target-w4 owner; do not duplicate a live build.

On failure, collect all diagnostics and hand back; do not run the full suite. On success, immediately run existing batch-gates.sh once through tool.monitor with timeout_ms: 1800000, collecting fmt, clippy, full --no-fail-fast, and dashboard serially with pre/post manifests. No source edits during gates.

Known backend.rs:61 baseline remains untouched. No Cargo inside individual repair tracks.

# Sequential exact debug build

Prepared scratch root: /private/tmp/omon-lane-scratch-qa-pibpnv. Runtime remains unreleased; no service start or scratch mutation performed.

Prerequisite: race-port-20261011T145201Z completed with focused exit 0 (reopen 5 passed; runner 11 passed, 1 ignored), strict Clippy 0, scoped format 0, source pre/post cmp 0, and preserved receipt cmp 0. Grouped Cargo PID 60540 exited before this build. Other observed Cargo owners were unrelated ferryx and mahoquot workspaces, not dori-work/target-w4.

Build monitor mon_7VANNZ643S54YF7G, session bash_25, batch PID 71067. Exact command cargo build -j2 --bin omo-gateway. Existing CARGO_TARGET_DIR=/Users/indo/code/project/omon-gateway-wt/dori-work/target-w4. Monitor deadline 1800000 ms. One noclobber started marker; no duplicate build or full-suite replay.

Before-build source manifest must equal completed race-port gate manifest or harness exits without invoking Cargo. After-build source manifest must equal before-build manifest before binary SHA-256 is recorded. Gate artifacts (source manifest, command statuses, focused log, Clippy log) are independently hashed under this timestamp. Exit statuses, logs, source manifests, artifact hashes and completed marker are sibling files.

# Exact debug binary evidence for scratch release

Sequential once-only command: `CARGO_TARGET_DIR=/Users/indo/code/project/omon-gateway-wt/dori-work/target-w4 cargo build -j2 --bin omo-gateway`, cwd dori-work. Actual exit 0, dev profile finished in 4m 19s. Batch PID 71067; monitor mon_7VANNZ643S54YF7G completed exit 0. Raw build output and status preserved under this prefix.

Binary: /Users/indo/code/project/omon-gateway-wt/dori-work/target-w4/debug/omo-gateway.
SHA-256: dbde346e2f7be84327a919e9242cba2a999a2d23ad417819e960fde1230d8a7e.

Build pre/post source manifests are exactly equal to the completed race-port-20261011T145201Z-post-manifest.sha256 (cmp 0). Gate manifest SHA-256 b617c3303f2ad0ba4286a6013d31f06c5bb2745e73946f9b7f0bb98169a73efe. Gate status SHA-256 8fd7c3639317f69b273aac9c32af49f900a4388e655344f85e67bf34e3acd5d5. Gate artifact checksum receipt includes focused and Clippy raw logs; all are tied to the same settled source set.

Preceding work gate: grouped scoped format 0, strict all-target/all-feature Clippy 0, focused no-fail-fast 0 (reopen 5 passed; runner 11 passed, live_qa_runner_process ignored). Sources and earlier task-7/repair receipts were preserved across gate; scratch fixtures were removed with rmdir/absence 0. LSP freshness timed out; compiler/lint evidence is clean. These tests cover deterministic cancellation and one specific ready-completion ordering, not all interleavings.

Prepared root /private/tmp/omon-lane-scratch-qa-pibpnv remains unreleased. No binary installation/copy, service start, external DB, Discord, git mutation or full-suite replay occurred. This is evidence for the requested scratch release decision, not a runtime QA result or confirmed verdict.

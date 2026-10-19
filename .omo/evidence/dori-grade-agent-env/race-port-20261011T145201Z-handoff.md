# Minimal work-tree race port results

No architecture conflict: dori-work already had the required per-turn JoinSet. Exact post-port string comparisons show backend/notification block and ClosingBackend/existing reopen regression match settled next source. No wave 3 settlement code imported. Source diff receipts are race-port-20261011T145201Z-runner.diff and -reopen.diff; before snapshots retained.

Only two source paths changed from prior guard repair:
- src/lanes/runner.rs SHA-256 83c41285631ebe74eb63d91f77ccf6c967464139cd1622b43f8a616783d7136c.
- tests/test_lane_reopen.rs SHA-256 b6b255f85049762a2a900c49cab708c6c2e14375dd1f65642da73afb6a25bb1a.

Once-only work commands and actual exits:
- rustfmt --edition 2021 --check src/lanes/runner.rs tests/test_lane_reopen.rs: 0.
- cargo clippy -j2 --all-targets --all-features -- -D warnings: 0, no emitted diagnostics, 1m 12s.
- cargo test -j2 --test test_lane_runner --test test_lane_reopen --no-fail-fast -- --nocapture with unique TMPDIR: 0. Reopen 5 passed, 0 failed, 0 ignored (0.48s); runner 11 passed, 0 failed, 1 ignored live_qa_runner_process (1.46s).

The previously failing old-turn regression passed with committed reopened-state/single-old-call assertions, cancellation acknowledgement, semaphore-controlled release, and original steer panic preserved. Additional completed-turn ready-notification regression passed; no sleeps or polling. It checks a specific current-thread ordering, not all completed-turn interleavings or a broad race-free claim.

Raw clippy SHA-256: 1fc240d92782df3208378dae4030e8a9f7718e88d014cf5a24875277afa40ad2.
Raw focused SHA-256: c95724d2014fe2892a99ab35435b34132ca56e60c4dc8b486bd2c002ff65baee.
Status, raw output, source and preservation manifests, scratch receipts and commands are under race-port-20261011T145201Z-*.

Source pre/post manifests identical (cmp 0). Existing task-7.log and both earlier repair receipt sets hash-preserved (cmp 0). Scratch `/var/folders/zh/7cc25lt91b1_dj577306nwdh0000gn/T/dori_race_port_XXXXXX.B41YbNccBZ` had zero entries after focused targets; rmdir 0 and independent absence check 0. Runner socket/shutdown fixture test passed. Batch PID 60540 exited; final ps returned 1 because that PID no longer exists. Monitor/harness exit 0.

LSP fresh diagnostics timed out and remain unavailable as evidence; strict compiler/lint run passed. No full/inbox/state/shell replay, external DB/service/Discord action, git mutation, sibling edit, or plan/ledger/Boulder update. Runner test's existing dirty-worktree fixture performs git operations only inside its disposable scratch repo. Evidence-backed port result only; no confirmed/global completion verdict.

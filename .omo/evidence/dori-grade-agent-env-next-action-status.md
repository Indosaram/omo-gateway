# Current verification and QA preparation

Todo 18 static diagnostic repair was released to st_01a12b2a: lexical scopes or minimal value copies must release MutexGuard values before awaits at test_lane_posts.rs:279 and :397. Assertions must remain intact; no lint suppression or individual Cargo rerun is permitted.

Canonical dori-work gate runner st_01a12b22 verified the long omo_backend target was still inside its baseline deadline: 1266 seconds elapsed versus approximately 1801 seconds. Monitor mon_M2XDWKMZPEBEZ05X remains active; no restart or additional batch is authorized. Full-suite, dashboard and manifest completion remain pending.

Scratch QA PREPARE completed with exit 0 through st_01a12b46. Owned root: `/private/tmp/omon-lane-scratch-qa-ZyXFMr`. Prepared receipt: `/private/tmp/omon-lane-scratch-qa-ZyXFMr/prepared.json`.

- Binary SHA256: dac35879f07ad1f93d0fe37b7cb14cc3cd92a4268691fdbf1e812fa2e2fd34ec
- Manifest SHA256: 5525bd6e6b1eed943005db6584d4e5b1d353acf2d6bfd17cf38c78965dcf5777
- Judge SHA256: daacff0341e0f0f1f86e4fa9335af723603a0515732ec72ed8aad2c4c6aa0fe6
- Prepared receipt SHA256: d2753dd317009365ad22c595ab33365aef62efc61a56c0498d8463a31dde4588

An incomplete release receipt was rejected with exit 1 and `Gate not released`, before runtime actions. The scratch root remains private and has no DB or execution marker. Actual backend scope probes and live QA are UNRUN. Evidence: task-8-9-12-prepared-root.md under dori-work evidence.

Todo 15 read-only implementation preflight is complete and reports contracts to resolve before implementation: worktree deletion ownership, human-owner identity, verified_done reopening and execution-cwd lifetime. Todo 16 read-only preflight is delegated to st_01a12b55. Neither track is implemented or confirmed.

Plan completion remains 8/41. No checkbox closure is justified by these preparation artifacts.

Post-guard repair validation completed: next-posts-20261011T142217Z exited 0, all eight test_lane_posts tests passed, none ignored. The companion all-target strict-clippy run next-clippy-20261011T142121Z exited 101 on two await_holding_lock diagnostics in test_lane_reopen.rs:121 and :242. Posts itself passed clippy. Later target diagnostics remain potentially masked; no whole-tree clean claim is made.

The two reopen guard scopes and the already known inbox unit-struct defaults at test_inbox_sources.rs:528/:552 were assigned together to st_01a12b2d in dori-next only. Assertions remain intact, no suppression is authorized, and no Cargo command runs inside that repair. On its source-settled receipt, one coordinated clippy run plus changed reopen/inbox focused checks is the next validation action. The dori-work source remains frozen under its canonical gate monitor.

The grouped test-only repairs settled. Strict all-target/all-feature clippy run next-clippy-20261011T142422Z now exited 0, verified directly by the parent from status.tsv. The changed reopen/inbox targets are being exercised together in next-focused-20261011T142517Z under monitor mon_WVY9PNY1GEXBA9SS. Their pass counts and live ignored names remain pending. This strict clippy pass belongs to dori-next; it does not repair or certify the separately frozen dori-work tree, and it is not a full-suite/manual-QA pass.

That focused run completed with exit 101: inbox 13 passed and four live tests ignored; reopen three passed and one failed. The failed close_reopen_discards_old_turn_and_delivers_new_owner_text_first observed reopening guidance steered into the old turn. Worker st_01a12b2d owns root-cause diagnosis and deterministic generation/cancellation regression repair; prior passing executions do not establish reliability.

Independent todo 11 shell-wrapper QA was dispatched to st_01a12b63, confined to settled wrapper/test files and isolated stub invocation. No live lane, DB, service or Discord action is authorized in that QA.

Late model-catalog timeout notifications for st_01a12989, st_01a12b28 and st_01a12b40 are not implementation failures or verified changes. The todo 13 and todo 21 units were already reassigned and have subsequent evidence; do not duplicate those implementations because an obsolete spawn notification arrives late. The exact original scope of st_01a12989 is not established by this notification alone.

Current todo7 independent review st_01a12b64 is NOT CONFIRMED: named task-7.log absent, dori-work fmt/clippy not clean. Canonical batch135508Z is now confirmed finished/quiescent by st_01a12b22: 934 passed, 15 baseline-only failures, 16 ignored; dashboard 59 passed; pre/post manifests identical. Worker st_01a12b67 owns minimal format/inbox constructor repairs and named state QA with a coordinated gate run; the old full suite is not replayed.

Todo11 shell injection-marker cwd gap was repaired/tested in dori-next (exit0). Exact latest output and missing-title exit1 were appended with provenance to named task-11.log; the repaired test hash is c86f6af488903f045590489af0f331967f707f9c61568c535fc1bbd65aad86ea. Dori-work still requires the minimal port, assigned to st_01a12b67 before source freeze or deferred until its run completes. Do not certify its old test using the other tree's new-test receipt.

Todo10 evidence reconciliation st_01a12b68 found existing task-10.log and prior independent review: historical create/rename/archive/GET archived and invalid-channel HTTP404/Discord10003 outputs exist. These are historical executor bot API evidence, not owner-performed QA or a new execution. Source hashes differ between work and next; unarchive/successful post/edit have no runtime receipt in this unit. Exact evidence/limitations are recorded in task-10-evidence-map.md. No direct Discord QA was performed during reconciliation; do not rerun historical live tests or relabel them as owner proof.

Repair144053Z completed in dori-work: fmt exit0, named task-7.log 11 passed, inbox13 passed/4live ignored, integrated shell exit0. Parent directly read task-7.log. Strict clippy exposed the two existing reopen guard lints; worker67 received guard-only test repair authorization and now owns one clippy/focused reopen run under recorded PID39537 and its active monitor, with no full-suite replay. Reviewer64 acknowledged the evidence update without tool inspection and retained NOT CONFIRMED; that acknowledgement is not a fresh gate verdict. A final review must inspect actual new exit receipts when delivered. Plan completion remains8/41.

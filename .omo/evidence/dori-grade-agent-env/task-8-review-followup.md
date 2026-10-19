# Todo 8 rejected-gate follow-up

The actual review `.omo/evidence/task8gate-review.md` and c7 incident report were read. No live cleanup is authorized or performed. C7 proves migrations 31-33 and the exact qa-1 row in the repository DB, with zero persisted worker activity. Owner deletion approval and migration disposition remain pending.

The reported main hash discrepancy was investigated before editing. The current complete source matched the original owned diff at the lifecycle region; no foreign changes were reverted. Earlier hashes computed from a truncated read are not authoritative; final hashes will use complete source bytes.

Product change: ordinary `omo-gateway run` now accepts explicit `OMON_DISCORD_ENABLED=false`, requires an explicit DATABASE_URL, opens/logs that database, and runs the same `spawn_lane_service` initialization and cancellation/join lifecycle used by the Discord-enabled path. No separate worker daemon or test-only runner is added. Both paths observe lane-service termination; failures return a nonzero process outcome instead of leaving a healthy-looking gateway. The default remains Discord enabled.

All new QA children are spawned by an owned launcher using `Bun.spawn` with an explicit merged process env whose DATABASE_URL is checked and printed before child invocation. The main gateway's PRAGMA database_list startup log confirmed `/private/tmp/omon-task8-review-Fy6m13/qa.db` before any CLI call. No shell-prefix environment assignments were used.

Actual debug gateway PID 63508 reached `gateway lane service ready`. The actual CLI created gateway-qa on this scratch DB. A second actual gateway against the same DB (PID 64566) logged the same DB identity, returned the lock conflict `lane runner already owns this database`, and exited 1. This demonstrates startup failure propagation; it is not a claim that every runtime failure has been injected.

Final-source focused tests passed: 3 CLI + 11 runner tests, 1 explicit live harness ignored. The QA below does not use that harness. Changed-file Rust 2021 fmt passed. Strict clippy and real turn evidence are collected separately.

Coverage limits remain explicit: cancellation/semaphore tests use a fake backend with default no-op cancel and do not prove remote tool termination. Literal shell-substitution tests prove transport non-interpretation, not model resistance to injected instructions. Dirty-worktree and done-looking-output tests do not prove later settlement behavior. Strict concurrent launch-slot reservation is a guard integration follow-up. Auto/large/override model coverage and rejected-steer/error coverage remain limited as stated by the reviewer.

## Completed actual gateway QA

The ordinary debug gateway completed one production OmoBackend turn on the explicitly selected small model. `gateway-qa` persisted `turns_run=1`, `last_turn_outcome=completed`, `status=done_claimed`, `claim_evidence=done.txt created`, and thread `01a129d1-437e-7c61-8747-161b450cf0b7`. The actual CLI transcript includes the REPORT milestone and done line. `done.txt` was read and contained `qa complete`. The claim command's success is backed by the persisted row, not just agent prose. The gateway also logged a successful real active-turn steer delivering the claim acknowledgement.

The actual CLI forced close exited 0 with status closed. SIGTERM to owned gateway PID 63508 produced `gateway lane service stopped` and process exit 0. Owned engine PID 61137 was then terminated; both PIDs were absent and port 29749 had no listener. Scratch root `/tmp/omon-task8-review-Fy6m13`, socket and lock were removed and their absence checked. No owner service was stopped. No live DB cleanup was performed.

Final exact strict clippy exited 0 in 1m 06s. Raw scoped-test output: task-8-revision-tests.txt. Raw clippy and gateway shutdown receipts: task-8-revision-gates.txt. Complete actual gateway/CLI/transcript/failure/invalid-syntax and final complete-file hashes: task-8-real-gateway.json. Explicit-environment launcher preserved as task-8-qa-launcher.txt. No source edits followed these tests or clippy.

B1 remedy is implemented and exercised using the explicitly authorized disposable endpoint substitution, with the real gateway lifecycle retained. B2 incident disposition remains PENDING OWNER APPROVAL; later safe QA does not erase it. This handoff is ready for renewed independent review, not a claim that historical isolation compliance has been restored.

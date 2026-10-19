# Live Manual-QA round 2

Execution is stopped; all owned gateway PIDs and lane sockets are absent, owned ephemeral backends and detached RPC process trees are cleaned up, and all 14 scratch roots are retained. This round is not a successful full Manual-QA gate. No automatic PASS is claimed from agent prose.

Release `wave4b-release.json` remained released/allowed. Before and after binary SHA-256: `5b765f9394354022dde3006252f8d874c74801ee2c54bd373758b54b7eb20f97`. Before and after manifest SHA-256: `a0837584175fecc6b6ac3907e9f378703c59b510d8ce4887161f27b3ca109f4f`.

| Task / scenario | Verdict | Machine evidence and exact limits |
| --- | --- | --- |
| 15 close/archive/reopen | FAIL | lane start exit 2: configuration error: missing required environment variable DISCORD_BOT_TOKEN. Zero lanes, zero recorder requests. Settlement, close, removal, reopen not reached. |
| 15 dirty worktree refusal | FAIL | Same lane-start exit 2; owned fixture worktree retained, but rejection behavior not reached. |
| 15 authenticated Discord input | NEEDS-REVIEW | No authenticated Discord ingress was exercised; no live Discord API used. |
| 16 inbox insertion | FAIL | Three-minute decision deadline; generation 1, consumed 0, admitted_ms null; zero kibitzer usage, judge rows and events. |
| 16 ten-minute idle | NEEDS-REVIEW | Execution monitor timed out before 600000 ms measurement completed. Final state unchanged with zero usage/events; missing duration and numeric gateway exit prohibit PASS. No relaunch. |
| 18 two milestones | FAIL | lane start exit 2 for missing DISCORD_BOT_TOKEN; zero local recorder requests. Two acknowledged posts/status PATCH not reached. |
| 18 deleted status replacement | FAIL | Same lane-start failure; deletion/404/replacement not reached. |
| 20 profile/persona lane | FAIL | Export/apply twice succeeded with hash idempotency and immutable fields; missing profile exit 2. Real lane-start predicate timed out; zero lanes and usage. |
| 20 reply meaning | NEEDS-REVIEW | Question/recommendation response phase not reached; no automatic semantic PASS. |
| 24 live source collection | FAIL | Corrected read-only preflight reached 166 items, zero decisions/usage. Mail: No accounts registered under private HOME. Kakao: empty source reference or snippet. Writing turn and unchanged-input second collection not reached. Monitor timeout prevented numeric final gateway exit capture. |
| 25 list/unknown item | PASS | Seeded owner filter count 1; unknown inbox item exits 2. This is seeded isolated CLI coverage, not the uncompleted task24 live dataset. |
| 25 native digest access | NEEDS-REVIEW | Native omon:inbox-digest job count 0; no script/model substitute or external post. |
| 26 explicit CLI/lead path | NEEDS-REVIEW | Proposal 1 created, real unapproved reply persisted and zero lanes; CLI status approved persisted, proposal still pending, approval_message_id null, lane_key null. Approved lane not observed before monitor timeout; numeric final gateway exit unavailable. |
| 26 authenticated numbered Discord approval | NEEDS-REVIEW | Adapter trust boundary not exercised; no fabricated approval CAS row or external message. |
| 35 real token lane | FAIL | Real turn completed and signal file exists, but model_usage is empty. Token predicate deadline; usage CLI, settlement and close not reached. |
| 35 unrelated diff rejection | FAIL | Real turn completed; unrelated README commit retained, but model_usage empty. Claim/settlement/done_diff judge proof not reached. |

Task-level aggregation: 15 FAIL, 16 FAIL, 18 FAIL, 20 FAIL, 24 FAIL, 25 NEEDS-REVIEW, 26 NEEDS-REVIEW, 35 FAIL. Human-review boundaries above always remain NEEDS-REVIEW, even when another variant of the same task failed.

## Cleanup limits

Normally finalized variants recorded gateway exit 0, PID absent, socket removed and app-server listener rebind. Several app-servers required their helper's bounded SIGKILL. Detached RPC supervisors escaped the app-server group and outlived scenarios; only groups proven by owned scratch socket paths were killed. See `qa2-owned-orphan-cleanup.json` and `qa2-owned-orphan-cleanup-extra.json`.

The execution monitors terminated the task16 idle and corrected task24/task26 runners before final receipts. Their gateway PIDs and sockets are now absent, but numeric gateway exit codes were lost and are explicitly null, not invented. Their orphan backend groups were stopped and ports 55582/55814/55815 exclusively rebound. Three private app-server token directories remain retained after owner termination; token contents were never read. No pending run was relaunched after the user's instruction.

## Evidence and helper changes

Plan artifacts `task-15.log`, `task-16.log`, `task-18.log`, `task-20.log`, `task-24.log`, `task-25.log`, `task-26.log`, `task-35.log` contain round-two raw receipts/logs; earlier contents were preserved. All eight `task<N>-live-qa2.md` receipts exist. `qa2-final-integrity.json` records final hashes, absent PIDs/sockets and retained DBs. Raw helper receipts and SQLite databases remain at the roots listed in the per-task receipts; interrupted variants have evidence-directory state receipts instead of fabricated helper receipts.

Only evidence helpers were changed: accept released binary_path/manifest_path receipt keys, run independent variants concurrently, correct inbox-proposal to released inbox-propose, and correct the stale PirateTalk hostname to the recorded/product default. Initial preparation-failure attempts were retained. Helper node syntax checks passed; bun predicate tests: 4 pass / 0 fail, exit 0. LSP initialization was unavailable because no TypeScript installation exists. No product source, owner real DB, main checkout, bot profile, env file or production gateway was written; no model override, shared daemon command or live Discord call was used.

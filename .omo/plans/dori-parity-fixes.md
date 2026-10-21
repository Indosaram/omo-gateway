# dori-parity-fixes - Work Plan

## TL;DR (For humans)

**Who this is for and what changes for them:** You (talking to the lead bot in Slack) and the lead bot itself. After this, "done" lanes never hide work that exists only on this Mac, "wait" is reported as done only once the lane really stopped, and a paused lane's finished work is still checked and shown as Paused on the dashboard.

**What you'll get:** Nine fixes to the lane system: stricter completion checks, a confirmed pause with a warning if a paused lane keeps working, a Paused state on the dashboard, a lane-count warning, friendlier completion-check syntax, owner-home routing instead of the retired Discord DM, and an immediate report when a lane cannot even start (missing key, quota).

**Why this approach:** Every change sits in the existing lane modules and follows patterns already there; the pause stays a separate marker instead of a new lane status so nothing that reads lane status breaks. A repo with no remote is refused, as you chose.

**What it will NOT do:** No emoji changes (you chose to keep them). No new engine features, no deploy, push or merge; those stay your call.

**Effort:** Medium
**Risk:** Medium - the pause confirmation touches the lane runner's turn loop and adds two small database changes.
**Decisions to sanity-check:** a repo with no remote blocks completion (lanes in local-only repos will need a remote or a force-close); the pause command waits up to 30 seconds for confirmation and returns an "unverified" error otherwise, and refuses adopted outside sessions; a claim made while paused is checked without resuming, and a verified one is closed; a missing key or unknown model stops the lane at once, while a quota or rate limit is only reported and retried; the lane-count warning fires when the limit is reached.

Your next move: start execution with /ulw-execute dori-parity-fixes when ready (plan reviewed by plan-consultant and plan-reviewer). Full execution detail follows below.

---

> TL;DR (machine): Medium effort, Medium risk; 8 implementation todos + 1 integration/QA todo (todo 9) + F1-F4; closes GAP-1..GAP-9 in src/lanes, src/storage/lanes.rs, src/tracker, migrations 0055-0056.

## Scope
### Affected user and ideal state
**Affected user:** (1) the owner, who talks to the lead bot in the Slack DM, starts lanes through it, and says "wait" / "go"; (2) the lead agent (bot persona), which reads lane events and the `omo-gateway lane ... --json` output; (3) lane agents, which run `lane claim-done`; (4) whoever reads the tracker dashboard (AGT team issues mirror lanes). Today they get "verified" lanes whose commits live only on this Mac, pauses that are reported as done before anything stopped, paused lanes whose claims sit forever, and a lead fallback pointing at the retired Discord DM. After this plan, each of those behaves as the dori-mode contract (sisyphuslabs/omo-dori-mode-experimental@0057f49) says.

| Row | Statement | Reason |
| --- | --- | --- |
| IS-1 | A lane reaches verified_done only when every commit reachable from each checked worktree HEAD exists on some remote; a repo with no remote and commits is refused as `unpushed <path> (no remote)`. Close-time sweep keeps such worktrees. | Lane footer promises "refuses commits on no remote" (src/lanes/runner.rs:2126); owner chose refuse (draft Q1). Reference done-flow.ts:60-62. |
| GAP-1 | settle.rs:618-624 and runner.rs:588-594 skip the unpushed check when `git remote` prints nothing. | A local-only lane is verified and closed with its work existing only in a branch nobody pushed. |
| IS-2 | A done claim made while the lane is paused is settled like any claim (5-min window, signals, worktree, diff gate); the pause row stays (goal stays paused), so a not_done result does not start new turns until resume. | Reference scripts.md:82: claim/object move the lane out of paused in the registry while the goal stays paused. |
| GAP-2 | settle.rs:52-56 skips every paused done_claimed lane. | The lead never hears verified/not_done for work finished just before "wait". |
| IS-3 | `omo-gateway lane pause <key>` exits 0 with `"confirmed": true` only after the lane runner recorded that no turn is running for that lane; if the runner does not confirm within 30 s it exits 3 with `"error": "PAUSE_UNVERIFIED"` and the pause row stays (so the next boundary still stops it). A second pause on a paused, confirmed lane is a no-op exit 0. Pausing an external (adopted) session is refused with exit 2 `NOT_PAUSABLE` because the gateway cannot stop it (decision Q5). | Reference pause.ts:36-46 (verify before reporting paused; exit 3 when unconfirmed). |
| GAP-3 | cli.rs:186-195 exits 0 right after inserting the row (storage/lanes.rs:234-260); nothing confirms the turn stopped. | The owner is told "stopped" while a turn may still be running tools. |
| IS-4 | If a paused lane produces agent output (captured activity) after its pause was confirmed, the lead gets exactly one `working_while_paused` lane event per pause; the lane stays paused. | Reference pause.ts:70-85 / freshness.ts:30-38 (WORKING-WHILE-PAUSED once per stretch). |
| GAP-4 | activity.rs:118-160 records output from a paused lane silently. | A pause that did not hold is invisible to the owner. |
| IS-5 | The tracker shows a paused lane's issue in a "Paused" state (type started) and moves it back to the lane status state on resume. | Reference registry.ts:6 makes paused a visible status; dashboard is the owner's lane view (memory: tracker Linear parity). |
| GAP-5 | tracker/lanes.rs:72-84 and seed.rs:44-75 know only lane.status; a paused lane shows "Working". | The dashboard lies about paused lanes. |
| IS-6 | Host guard ALERT includes `active lanes (N) >= maximum (M)` when active (non-queued) lanes reach lanes_max, and CLEAR waits for that to clear too (decision Q4: reach, not exceed; reference host-guard.ts:35 uses exceed, but launches here queue at >=, guard.rs:177, so exceed would never fire). | Reference host-guard.ts:34 (pane count in alert reasons). |
| GAP-6 | guard.rs:610-676 computes only load/mem/disk. | Lane saturation never reaches the lead as a guard event; launches just queue. |
| IS-7 | `parse_signals` accepts a leading `Done =` / `done=` (any case, optional spaces), accepts `merged|closed owner/repo/pull/N` and `owner/repo/issues/N`, rejects `sha256=` that is not exactly 64 hex chars, and rejects a json path that is empty after stripping the leading dot. | Reference signals.ts:61,114,140 and the json path error. |
| GAP-7 | signals.rs:179-188 no prefix strip; :304,:343 '#' only; :675 any length; :684-688 accepts `json:.=1`. | The documented `Done = ...` line is rejected at launch; impossible-to-pass sha256 checks are accepted. |
| IS-8 | When a lane row is missing, approval and lead routing for that lane use `runtime_config::default_lead()` (Slack owner DM at boot), never the compiled Discord DM constant. | Slack-only since 2026-10-18 (memory); main.rs:875 already sets the runtime lead. |
| GAP-8 | runner.rs:933 and agent/omo_backend.rs:1915 call `kibitzer::DEFAULT_LEAD` directly. | An approval for a lane whose row vanished goes to Discord, which no longer runs. |
| IS-9 | When a lane's very first turn fails with a known startup cause (case-insensitive substring), the lane gets a `startup_error` lane event (critical) at once. Permanent causes (`no api key`/`api key`, `unknown provider`, `model_not_available`, `all fallback models failed`, `cannot find module`, `session crashed`) also fail the lane immediately; transient causes (`rate limit`, `quota`, `usage limit`) only report and keep the existing 3-error retry (decision Q3). | Reference launch.ts:20,64-71 (STARTUP_ERROR on a known failure). |
| GAP-9 | runner.rs:1981-1984 retries every error until 3, then emits `dead`. | The owner learns about a missing key or quota minutes later under a misleading "dead" label. |

### Must have
- All nine IS rows delivered, each proven by its QA scenario.
- Additive migrations only (new column on `tracker_states`, new row type values); no rebuild of `lanes` or `tracker_states`.
- Existing behavior kept: pause still cancels the running turn and starts no new turn (tests/test_lane_control.rs a-e stay green unchanged; f changes exit code and JSON for the pause step because IS-3 makes an unconfirmed pause exit 3); reopen still clears a pause (runner.rs:137-143); resume still exits 2 NOT_PAUSED.
- Every new lane_events kind (`working_while_paused`, `startup_error`) handled by src/lanes/dispatch.rs: `startup_error` in `critical`, `working_while_paused` routed like other events (not critical).

- Before todo 1 starts, the executor records `git rev-parse HEAD` into .omo/evidence/dori-parity-fixes/base.txt; F2 diffs against that SHA.
- New lead guidance for every new outcome (startup_error, working_while_paused, PAUSE_UNVERIFIED, NOT_PAUSABLE, `unpushed <path> (no remote)`) in bot_profiles/blocks/lanes.md "Lane events" (prose only; no test pins its wording).

### Must NOT have (guardrails, anti-slop, scope boundaries)
- No `LaneStatus::Paused` enum variant and no change to `lanes.status` values (decision kept from .omo/plans/autonomy-two-axis.md:302).
- No emoji changes in Slack status text (owner decision Q2: keep).
- No COMPUTE_READY/BUSY events, no idle-lane reuse code, no brief-footer file writes, no dead-pane/sync-drift/2-tick report logic, no `$(` refusal (re-verification: substrate differences or no consumer).
- No change to senpi/omo engine installs, no `.env` edits, no Discord feature work, no herdr.
- No new polling loop in the lane service; the service acks pauses inside its existing loop pass after the wake datagram. The CLI is a separate process with no notifier, so it may re-read its own pause row on a bounded 250 ms interval until the deadline (default 30 s); nothing else polls.
- No deploy, push or merge in this plan's todos; the owner deploys via main + scripts/deploy.sh when they say so.
- No intermediate cargo builds inside implementation todos (owner rule): workers write code and tests only; the single build/test/clippy run is todo 9.

## Verification strategy
> Zero human intervention - all verification is agent-executed.
- Test decision: tests-after, in the existing test files named per todo (cargo test, tokio). Per owner rule, implementation todos only write code and tests; todo 9 runs the one build/test/clippy/fmt pass and fixes failures.
- Evidence: .omo/evidence/dori-parity-fixes/task-<N>.<ext> (outside ulw-loop, plain .omo/evidence/).
- Real-surface QA (todo 9) runs the built binary `target/debug/omo-gateway lane ...` against a scratch SQLite DB (`sqlite://$RUN_TMP/qa.db`) and scratch git repos with a bare remote, plus the in-process service tests; it never touches the production DB `omon_gateway.db` except read-only.

## Execution strategy
### Parallel execution waves
- Wave 1 (parallel, disjoint files): todo 1 (settle.rs worktree_reasons + runner.rs keep_reason + test fixtures), todo 4 (guard.rs + guard tests), todo 5 (signals.rs). Todo 6 (runner.rs:933 + omo_backend.rs:1915) runs after todo 1 or before it, never at the same time, because both edit runner.rs.
- Wave 2: todo 2 (settle.rs + tests/test_lane_done.rs) after todo 1; todo 3 after todos 1, 5 and 6 (all share files with it) (pause confirmation: migration 0055, storage/lanes.rs, cli.rs, runner.rs pause paths, activity.rs, dispatch.rs tests, bot_profiles/blocks/lanes.md).
- Wave 3: todo 7 (startup_error: runner.rs + dispatch.rs) after todo 3; todo 8 (tracker Paused: migration 0056, tracker files) after todo 3; 7 and 8 touch disjoint files and may run in parallel.
- Wave 4: todo 9 single build/test/lint + real-surface QA after all.
- Delegation topology: one worker per todo; workers write code and tests only (owner rule: no intermediate builds); the lead runs todo 9.

### Dependency matrix
| Todo | Depends on | Blocks | Can parallelize with |
| --- | --- | --- | --- |
| 1 | - | 2, 9 | 4, 5 |
| 2 | 1 | 9 | 4, 5, 6 |
| 3 | 1, 5, 6 | 7, 8, 9 | 4 |
| 4 | - | 9 | 1, 5, 6 |
| 5 | - | 3, 9 | 1, 4, 6 |
| 6 | - | 3, 9 | 4, 5 (not 1: both edit runner.rs) |
| 7 | 3 | 9 | 8 |
| 8 | 3 | 9 | 7 |
| 9 | 1-8 | F1-F4 | - |

## Todos
> Implementation + Test = ONE todo. Never separate.
<!-- APPEND TASK BATCHES BELOW THIS LINE WITH edit/apply_patch - never rewrite the headers above. -->
- [ ] 1. Refuse commits that exist on no remote, in settle and in the close sweep
  What to do:
  - src/lanes/settle.rs `worktree_reasons` (lines 608-626): drop the `git remote` guard. For each path: if `git -C <path> rev-parse --verify -q HEAD` exits 1 (unborn HEAD, no commits) there is nothing to push; otherwise run `git -C <path> rev-list --count HEAD --not --remotes` (reference done-flow.ts:60; with no remote it counts every commit). Count > 0 -> reason `unpushed <path> (no remote)` when `git -C <path> remote` prints nothing, else `unpushed <path>` (existing text). Any other git failure stays an Err, never a pass.
  - src/lanes/runner.rs `keep_reason` (lines 583-596): same rule; a worktree whose HEAD has commits on no remote returns `Some("unpushed")`, so the close sweep keeps it. Unborn HEAD -> no unpushed reason.
  - Tests that must change (they assert verified/removed for a repo with commits and no remote): tests/test_lane_done.rs `Fixture::new` (lines 89-161: add a bare remote `remote.git`, `git remote add origin`, `git push -q origin HEAD:refs/heads/fixture`; tests that need an unpushed commit already add one); tests/test_lane_close.rs (repo at :87); tests/test_lane_worktree_auto.rs (repo at :58; push `main` to a bare remote so the orphan-clean test at :200-246 still removes); tests/test_usage_done_gate.rs (repo at :138); src/lanes/settle.rs test `prunable_worktree_registration_does_not_block_verification` (:794, uses commit_fixture without remote). tests/test_fix_fix_settle.rs already has a bare remote (:90): verify only. Never weaken an assertion; if another test turns red in todo 9 for this reason, give it a bare remote the same way.
  - New tests: tests/test_lane_done.rs `unpushed_commit_without_any_remote_returns_not_done` (fixture variant without remote: settle -> not_done, objection contains `(no remote)`) and `repo_without_commits_and_remote_is_not_refused` (unborn HEAD, clean -> no unpushed reason); src/lanes/runner.rs tests `keep_reason_keeps_a_no_remote_worktree_with_commits` -> `Some("unpushed")`.
  - Footer text needs no change (runner.rs:2126 already promises the refusal).
  Must NOT do: no change to the uncommitted check or to which worktrees are selected (include_worktree); no network remotes in tests.
  Closes: GAP-1
  Parallelization: Wave 1 | Blocked by: - | Blocks: 2, 9
  References: src/lanes/settle.rs:554-627,794-858, src/lanes/runner.rs:583-596,2126, tests/test_lane_done.rs:61-161,361-404 (bare-remote pattern), tests/test_fix_fix_settle.rs:90, tests/test_lane_close.rs:87, tests/test_lane_worktree_auto.rs:58,200-246, tests/test_usage_done_gate.rs:138, reference /tmp/dori/skills/dori-mode/scripts/src/done-flow.ts:55-63
  Acceptance criteria (agent-executable): in todo 9, `cargo test --test test_lane_done unpushed_commit_without_any_remote repo_without_commits` (two runs, one per filter) and `cargo test --lib keep_reason_keeps` pass; the full `cargo test` is green.
  QA scenarios (settlement runs only inside LaneRunner::serve with a 5-minute window, so the surface is the in-process settle harness with MockClock, not the binary):
  - happy (refuse): `cargo test --test test_lane_done unpushed_commit_without_any_remote -- --nocapture` -> 1 passed; asserts status not_done and objection contains `(no remote)`.
  - failure-guard (pushed or empty repo passes): `cargo test --test test_lane_done repo_without_commits -- --nocapture` and the existing `verified` tests in that file pass with the pushed fixture.
  - Evidence: .omo/evidence/dori-parity-fixes/task-1.txt (command, exit code, test output)
  Commit: Y | fix(lanes): refuse lane commits that exist on no remote
  Recommended task executor category: unspecified-low - two small functions plus named fixture updates.
- [ ] 2. Settle done claims of paused lanes
  What to do:
  - src/lanes/settle.rs `settle_due` lines 52-56: delete the paused-lane `continue`. Settlement does not touch the pause row. A verified paused lane is then closed by the service like any verified lane (runner.rs:1478-1482; close leaves the pause row, runner.rs:137-139), which is intended: the work is finished. A not_done paused lane gets its "[LEAD] not done" inbox row but starts no turn until resume (runner.rs:1501, 1613).
  - Test in tests/test_lane_done.rs (has MockClock + Checks fakes, :61-69,157-160): `paused_claim_is_settled_and_pause_survives`: Fixture with passing check, `pause_lane`, claim, advance 5 min, settle -> verified_done and `is_lane_paused` still true; variant `paused_claim_with_failing_check_is_not_done_and_stays_paused` -> not_done, a lane_events not_done row exists, pause still true.
  Must NOT do: no resume side effect; no change to change_claim.
  Closes: GAP-2
  Parallelization: Wave 2 | Blocked by: 1 (same files) | Blocks: 9
  References: src/lanes/settle.rs:47-186, src/storage/lanes.rs:215-286, src/lanes/cli.rs:672-760, src/lanes/runner.rs:137-139,1478-1482,1501,1613, tests/test_lane_done.rs:61-260, reference /tmp/dori/skills/dori-mode/references/scripts.md:82
  Acceptance criteria: `cargo test --test test_lane_done paused_claim` -> 2 passed (todo 9).
  QA scenarios:
  - happy: `cargo test --test test_lane_done paused_claim_is_settled -- --nocapture` -> verified_done, paused true.
  - failure: `cargo test --test test_lane_done paused_claim_with_failing_check -- --nocapture` -> not_done, paused true.
  - Evidence: .omo/evidence/dori-parity-fixes/task-2.txt
  Commit: Y | fix(lanes): settle a done claim even while the lane is paused
  Recommended task executor category: quick - one deleted guard plus two tests.
- [ ] 3. Confirm a pause before reporting it, and report output after a confirmed pause
  What to do:
  - Migration migrations/0055_lane_pause_ack.sql (new): `ALTER TABLE lane_pauses ADD COLUMN acked_at DATETIME; ALTER TABLE lane_pauses ADD COLUMN seen_working_at DATETIME;`.
  - src/storage/lanes.rs `pause_lane` (234-260): refuse `kind = 'external'` lanes with a typed error mapped by the CLI to exit 2 `NOT_PAUSABLE` (decision Q5); on insert/upsert of a new pause set `acked_at = NULL, seen_working_at = NULL`. Add `ack_pause(executor, key) -> Result<bool>` (`UPDATE lane_pauses SET acked_at = ? WHERE lane_key = ? AND resumed_at IS NULL AND acked_at IS NULL`) and `pause_acked(pool, key) -> Result<bool>`.
  - src/lanes/runner.rs ack points (each only after no turn of that lane is running):
    (a) `run_lane` start when already paused (1613-1615);
    (b) the turn-boundary return when paused (1667-1673);
    (c) the `shutdown.is_cancelled() || paused_now` return (1979) when `paused_now`, whatever the outcome (Ok or Err);
    (d) every service loop pass (around 1488-1503): `SELECT lane_key FROM lane_pauses WHERE resumed_at IS NULL AND acked_at IS NULL`, and ack each key with no entry in `active`. This covers queued, blocked and done_claimed lanes that the working/not_done loop never visits.
  - src/lanes/cli.rs `LaneCommand::Pause` (186-195): after `pause_lane` + `wake`: if `pause_acked` -> exit 0; else re-read the row every 250 ms (tokio interval) until a deadline of 30 s (override `OMON_LANE_PAUSE_CONFIRM_SECS`, parsed as u64, for tests). Success JSON `{"key","paused":true,"confirmed":true,"changed"}` exit 0; timeout JSON `{"ok":false,"key","error":"PAUSE_UNVERIFIED","paused":true}` exit 3, row kept. External -> `{"ok":false,"key","error":"NOT_PAUSABLE"}` exit 2.
  - src/lanes/activity.rs `capture` (118-160), used by both `captured`/`captured_generation` and `captured_item` (external path, src/sessions/external.rs:318): inside the same tx, if an open pause row has `acked_at IS NOT NULL AND seen_working_at IS NULL`, set `seen_working_at` and insert one `lane_events` row kind `working_while_paused` (body `lane <key> produced output after its pause was confirmed`, lead from the lane row). No other state change.
  - src/lanes/dispatch.rs: `working_while_paused` stays non-critical; add it to the dispatch kind test list (around :256).
  - bot_profiles/blocks/lanes.md "Lane events" section: one line each for PAUSE_UNVERIFIED (lane may still run; check `lane log`, retry), NOT_PAUSABLE (external session; tell the owner), `working_while_paused` (a paused lane produced output; tell the owner), `startup_error` (lane could not start: key, provider, quota; tell the owner the cause), and the `unpushed <path> (no remote)` objection (ask the owner to add a remote or force-close). The block text is passed to `apply_block` at apply time (src/profiles/lanes_block.rs:130-138), so editing the file is the change; re-applying it to the live profile belongs to the owner's deploy.
  - Tests (tests/test_lane_control.rs; CLI pattern `cli()` at :287-299 gains `.env("OMON_LANE_PAUSE_CONFIRM_SECS","1")`): update `f_resume_on_an_unpaused_lane_exits_2_with_not_paused` (:302) so its pause step expects exit 3 PAUSE_UNVERIFIED (no service) and resume still works; new `h_pause_without_runner_exits_3_unverified` (exit 3, error PAUSE_UNVERIFIED, row open); `i_pause_with_running_service_confirms` (file DB, spawn `LaneRunner::serve` like tests/test_lane_runner.rs:417-470 with a blocking backend, run the CLI pause with the 30 s default -> exit 0, confirmed true; then a second pause -> exit 0 changed false); `k_pause_of_done_claimed_lane_confirms_via_service_pass` (ack point d); `l_pause_of_external_lane_is_refused` (exit 2 NOT_PAUSABLE); `j_output_after_confirmed_pause_emits_one_event` (for the external `captured_item` path insert the acked pause row with SQL, since `pause_lane` refuses external lanes; then `LaneActivity::captured` twice and `captured_item` once -> exactly one working_while_paused event).
  Must NOT do: no engine-side goal command (lanes run without the engine follower, src/agent/omo_backend.rs:2666, and the gateway only uses `turn/interrupt`, omo_backend.rs:2919-2950); no lanes.status change; no sleeps in tests (await the CLI child process exit, which is bounded by its own deadline).
  Closes: GAP-3, GAP-4
  Parallelization: Wave 2 | Blocked by: 1 and 6 (runner.rs), 5 (cli.rs) | Blocks: 7, 8, 9
  References: src/lanes/cli.rs:66-80,161-204, src/storage/lanes.rs:213-286, src/lanes/runner.rs:1016-1030,1465-1516,1605-1680,1836-1846,1918-1985, src/lanes/activity.rs:94-170, src/sessions/external.rs:318, src/lanes/dispatch.rs:214-258, migrations/0048_lane_pauses.sql, tests/test_lane_control.rs:59-91,287-340, tests/test_lane_runner.rs:417-470, bot_profiles/blocks/lanes.md:17-21, src/profiles/lanes_block.rs:130-140, reference /tmp/dori/skills/dori-mode/scripts/src/pause.ts
  Acceptance criteria: `cargo test --test test_lane_control` all pass including f, h, i, j, k, l (todo 9).
  QA scenarios:
  - happy (confirmed): `cargo test --test test_lane_control i_pause_with_running_service_confirms -- --nocapture` (real service + built CLI binary on one file DB) -> exit 0, `"confirmed": true`.
  - failure (unverified, real binary): QA-3 in todo 9: `OMON_LANE_PAUSE_CONFIRM_SECS=2 DATABASE_URL=sqlite://$RUN_TMP/qa.db target/debug/omo-gateway lane pause qa-p` with no service -> exit 3, stdout `.error == "PAUSE_UNVERIFIED"`; `sqlite3 $RUN_TMP/qa.db "select resumed_at is null from lane_pauses where lane_key='qa-p'"` prints 1. Then `sqlite3 $RUN_TMP/qa.db "update lane_pauses set acked_at=datetime('now') where lane_key='qa-p'"` and a second pause -> exit 0, `"changed": false, "confirmed": true`.
  - Evidence: .omo/evidence/dori-parity-fixes/task-3.txt
  Commit: Y | feat(lanes): confirm a pause before reporting it and flag output from a paused lane
  Recommended task executor category: deep-low - cross-file, each change follows an existing pattern; additive migration.
- [ ] 4. Alert when active lanes reach the maximum
  What to do:
  - src/lanes/guard.rs `GuardWatcher::check_at` (610-676): `lanes_alert = sample.active_lanes >= self.thresholds.lanes_max` (decision Q4), reason `active lanes ({n}) >= maximum ({m})`; CLEAR additionally needs `active_lanes < lanes_max`. Extend `AlertMark` (500-507) with `lanes: bool`, `mark_alert` with the flag, and `got_worse` so a newly alerting lane count is new information.
  - Tests: src/lanes/guard.rs tests `lane_count_alerts_and_clears`; tests/test_lane_guard_alerts.rs `lane_saturation_alert_once_then_clear` (thresholds lanes_max 1; `HostSample::new(10.0, 50, 100, 1)` -> one guard event whose body contains `active lanes (1) >= maximum (1)`; a repeat sample adds none; `HostSample::new(10.0, 50, 100, 0)` -> a CLEAR event). Existing samples there use active_lanes 0 (:37,64,78,95,130,161) and stay unchanged.
  Must NOT do: no change to can_launch; no new event kind.
  Closes: GAP-6
  Parallelization: Wave 1 | Blocked by: - | Blocks: 9
  References: src/lanes/guard.rs:10-75,95-110,170-185,460-702,1100-1240, tests/test_lane_guard_alerts.rs:1-170, reference /tmp/dori/skills/dori-mode/scripts/src/host-guard.ts:29-35
  Acceptance criteria: `cargo test --lib lane_count_alerts_and_clears` and `cargo test --test test_lane_guard_alerts` pass (todo 9).
  QA scenarios (the guard is a pure state machine fed by `enqueue_transition`; the integration test drives the same path the service uses):
  - happy: `cargo test --test test_lane_guard_alerts lane_saturation_alert_once_then_clear -- --nocapture` -> one ALERT with the lane reason, then CLEAR.
  - failure: the existing tests in that file (active_lanes 0) still produce no lane reason.
  - Evidence: .omo/evidence/dori-parity-fixes/task-4.txt
  Commit: Y | fix(lanes): include the lane count in host guard alerts
  Recommended task executor category: quick - one state machine extension with tests.
- [ ] 5. Accept the documented Done line forms and reject unusable checks
  What to do (src/lanes/signals.rs unless noted):
  - Add `pub fn strip_done_prefix(raw: &str) -> &str` removing a leading `(?i)done\s*=\s*`; `parse_signals` (179-198) uses it. src/lanes/cli.rs start (validates at :372, stores `args.done` at :472) and src/sessions/external.rs adopt (validates at :102, stores via the insert at :128-131) store the stripped text in `done_signals`, so the footer (runner.rs:2126 prints `Done = {done_signals}`) never doubles the prefix.
  - `parse_merged` / `parse_closed` (298-375): accept `owner/repo#N`, `owner/repo/pull/N` and `owner/repo/issues/N`, N a positive integer; other shapes keep the existing errors.
  - `parse_file_option` (667-700): `sha256=` must be exactly 64 hex chars, else `InvalidSyntax("sha256= takes 64 hex characters")`; json path empty after stripping one leading `.` -> `InvalidSyntax("json: needs a field path like json:.version")`.
  - Fix in-file tests that use short hashes (around :1610, `[sha256=1234abcd]`) to 64 chars.
  - New tests in the module: `test_parse_done_prefix`, `test_parse_github_url_forms`, `test_parse_rejects_short_sha_and_empty_json_path`, `test_strip_done_prefix_keeps_plain_lines`.
  Must NOT do: no change to runtime checks; Display keeps the `#` form.
  Closes: GAP-7
  Parallelization: Wave 1 | Blocked by: - | Blocks: 3 (cli.rs), 9
  References: src/lanes/signals.rs:40-110,179-198,276-375,624-700,1514-1820, src/lanes/cli.rs:372,430-475, src/sessions/external.rs:91-131,497, src/lanes/runner.rs:2126, reference /tmp/dori/skills/dori-mode/scripts/src/signals.ts:55-160
  Acceptance criteria: `cargo test --lib lanes::signals` passes (todo 9). Stored prod signals keep parsing: todo 9 step 4 checks counts read-only.
  QA scenarios (real binary, QA-5 in todo 9; lead and model given explicitly so no other check fails first, cli.rs:439-452; worktree off, runner.rs:47-57):
  - happy: `OMON_LANE_AUTO_WORKTREE=0 DATABASE_URL=sqlite://$RUN_TMP/qa.db target/debug/omo-gateway lane start qa-sig --title t --brief $RUN_TMP/b.md --cwd $RUN_TMP/repo --lead slack:qa --model small --done "Done = file x.txt"` -> exit 0 or 4 (HOLD), and `sqlite3 $RUN_TMP/qa.db "select done_signals from lanes where key='qa-sig'"` prints `file x.txt`.
  - failure: same with key qa-sig2 and `--done "file x.txt sha256=abcd"` -> exit 2, stdout `.error` contains `sha256= takes 64 hex characters`.
  - Evidence: .omo/evidence/dori-parity-fixes/task-5.txt
  Commit: Y | fix(lanes): accept Done-prefixed and GitHub path signals, reject unusable sha256 and json checks
  Recommended task executor category: quick - parser edits with unit tests.
- [ ] 6. Route lead fallbacks to the runtime owner home
  What to do:
  - Add `pub fn lane_lead_or_default(lead: Option<String>) -> String` to src/runtime_config.rs (`lead.unwrap_or_else(default_lead)`).
  - src/lanes/runner.rs:933 and src/agent/omo_backend.rs:1915: use it instead of `kibitzer::DEFAULT_LEAD`.
  - Leave alone: src/runtime_config.rs:38 (its own last fallback), src/cron/health.rs:298 and src/cron/scheduler.rs:2650 (fallbacks after `default_lead()`), and `#[cfg(test)]` uses (kibitzer.rs:579+, main.rs:2149+, inbox/triage.rs).
  - Test in src/runtime_config.rs, child-process style (:47-77), env `OMON_LANE_DEFAULT_LEAD` removed: runtime set -> returns the runtime value; `Some(x)` -> x; nothing set -> the compiled constant.
  Must NOT do: no change to DEFAULT_LEAD or kibitzer owner-DM checks; no .env edits.
  Closes: GAP-8
  Parallelization: Wave 1 (not concurrently with todo 1: both edit runner.rs) | Blocked by: - | Blocks: 3, 9
  References: src/lanes/runner.rs:915-940, src/agent/omo_backend.rs:1905-1920, src/runtime_config.rs:1-77, src/main.rs:868-885, src/lanes/kibitzer.rs:21-22,160-171, src/cron/health.rs:298, src/cron/scheduler.rs:2650
  Acceptance criteria: `rg -n "kibitzer::DEFAULT_LEAD" src/lanes/runner.rs src/agent/omo_backend.rs` prints nothing; `cargo test --lib runtime_config` passes (todo 9).
  QA scenarios:
  - happy: the rg command above exits 1 with no output.
  - failure-guard: the child-process test case "nothing set -> compiled constant" passes.
  - Evidence: .omo/evidence/dori-parity-fixes/task-6.txt
  Commit: Y | fix(lanes): send lead fallbacks to the runtime owner home
  Recommended task executor category: quick - two call sites and a helper.
- [ ] 7. Report known startup failures on the first lane turn
  What to do:
  - src/lanes/runner.rs: remember `first_turn` when the `lane.turns_run == 0` prompt branch is taken (around 1741). After the outcome is recorded and the paused/shutdown return (1979), before `errors = ...` (1982): if `outcome.is_err()` and `first_turn`, lowercase the error text and match (decision Q3):
    - permanent (`api key`, `unknown provider`, `model_not_available`, `all fallback models failed`, `cannot find module`, `session crashed`) -> `return self.fail(key, "startup_error", &outcome_text, generation).await`;
    - transient (`rate limit`, `quota`, `usage limit`) -> insert one lane_events row kind `startup_error` (body says it will retry) and continue the existing 3-error rule.
  - src/lanes/dispatch.rs `critical` (214-227): add `"startup_error"`; add a test case.
  - tests/test_lane_runner.rs: add `error_text: Option<&'static str>` to the `Script` backend (:15-41, default keeps "scripted turn failure"). Tests: `permanent_startup_error_fails_on_first_turn` ("No API key for provider x" -> failed, one startup_error event); `transient_startup_error_reports_and_retries` ("rate limit exceeded" -> one startup_error event, after 3 errors failed with `dead`); `ordinary_first_turn_error_still_retries` ("connection reset" -> no startup_error, behaves like `three_errors_fail_with_dead_event` :163-178).
  Must NOT do: no model fallback logic; no change to the 3-error rule after the first turn.
  Closes: GAP-9
  Parallelization: Wave 3 | Blocked by: 3 (runner.rs, dispatch.rs) | Blocks: 9
  References: src/lanes/runner.rs:1735-1760,1918-1990,2104-2120, src/lanes/dispatch.rs:214-258, tests/test_lane_runner.rs:15-41,163-178, reference /tmp/dori/skills/dori-mode/scripts/src/launch.ts:20,64-71
  Acceptance criteria: `cargo test --test test_lane_runner startup_error` (2 passed) and `cargo test --test test_lane_runner ordinary_first_turn_error` pass (todo 9).
  QA scenarios (the real LaneRunner loop with a scripted backend is the surface; a real missing key cannot be staged without touching .env):
  - happy: `cargo test --test test_lane_runner permanent_startup_error -- --nocapture` -> failed + startup_error.
  - failure-guard: `cargo test --test test_lane_runner ordinary_first_turn_error -- --nocapture` -> no startup_error.
  - Evidence: .omo/evidence/dori-parity-fixes/task-7.txt
  Commit: Y | fix(lanes): report known startup failures on the first lane turn
  Recommended task executor category: quick - one branch, one critical kind, three tests.
- [ ] 8. Show paused lanes as Paused in the tracker
  What to do:
  - Migration migrations/0056_tracker_lane_paused.sql (new): `ALTER TABLE tracker_states ADD COLUMN lane_paused INTEGER NOT NULL DEFAULT 0;` then `INSERT INTO tracker_states (id, team_id, name, type, color, position, lane_status, description, lane_paused) SELECT lower(hex(randomblob(16))), id, 'Paused', 'started', '#9b8afb', 2.5, NULL, NULL, 1 FROM tracker_teams WHERE key = 'AGT';`
  - src/tracker/seed.rs `seed_agent_states` (44-75): also insert the Paused row (lane_paused 1) for a new AGT team.
  - src/tracker/lanes.rs `sync_inner` (100-296): load `paused_lane_keys` and, with a separate query, the AGT state with `lane_paused = 1` (the existing state map only loads rows with `lane_status IS NOT NULL`); a lane is shown paused only when it has an open pause row AND status IN ('queued','working','blocked','done_claimed','not_done'); closed, failed and verified lanes keep their lane_status state. Apply this in both the existing-issue path and the new-issue branch (283-296). `status_changed` compares the effective value (`paused` or the status) so the issue moves both ways; activity `to_value` records `paused`.
  - src/tracker/issues.rs state PATCH (602-611): before calling `lanes::handle_lane_state_transition` for a lane-linked issue, read `lane_paused` of the target `state_id`; if it is 1 return 409 `illegal_lane_transition` ("Paused is set by lane pause") for every lane status, so the reopen branch in tracker/lanes.rs:466-477 never runs for it. State deletion is already blocked for the system team (src/tracker/workspace.rs:898-911,971-976): no change.
  - Tests in tests/test_tracker_api.rs: `paused_lane_mirrors_to_paused_state_and_back` (working lane, pause row, sync -> Paused; resume -> Working); `closed_lane_with_open_pause_stays_done`; `moving_a_closed_lane_issue_to_paused_is_refused` (409 illegal_lane_transition, lane status still closed); extend the bootstrap test (:103) to see a Paused state in AGT.
  Must NOT do: no change to the lane_status CHECK constraint; no new routes; no delete guard.
  Closes: GAP-5
  Parallelization: Wave 3 | Blocked by: 3 | Blocks: 9
  References: migrations/0044_tracker.sql:13-38, src/tracker/seed.rs:44-75,235-252, src/tracker/lanes.rs:72-296,415-485, src/tracker/issues.rs:583-633 (lane call at 602-611), src/tracker/workspace.rs:898-911,971-990, src/storage/lanes.rs:224-230, tests/test_tracker_api.rs:100-260
  Acceptance criteria: `cargo test --test test_tracker_api` passes including the three new tests (todo 9).
  QA scenarios:
  - happy (real binary, QA-8 in todo 9): `DATABASE_URL=sqlite://$RUN_TMP/qa.db target/debug/omo-gateway dashboard --port 18999` with `OMON_DASHBOARD_RUN_SCHEDULER` unset (no runtime lock, no cron; src/entry.rs:133-140,385-393), seed a working lane row and pause row with sqlite3, then `curl -s http://127.0.0.1:18999/api/tracker/bootstrap` for the AGT team id and `curl -s 'http://127.0.0.1:18999/api/tracker/issues?team_id=<AGT>'` -> the lane issue's state name is "Paused"; set `resumed_at` and re-fetch -> "Working". If the dashboard needs auth or a sync trigger not available here, the Harness tests above (real axum router) are the surface and this is recorded.
  - failure: `cargo test --test test_tracker_api moving_a_closed_lane_issue_to_paused_is_refused -- --nocapture` -> 409, lane still closed.
  - Evidence: .omo/evidence/dori-parity-fixes/task-8.txt
  Commit: Y | feat(tracker): mirror paused lanes into a Paused state
  Recommended task executor category: unspecified-low - additive migration plus mirror mapping in known files.
- [ ] 9. One build, full test and lint pass, and real-surface QA
  What to do:
  1. `export RUN_TMP=$(mktemp -d)`; all QA DBs and repos live there.
  2. `cargo fmt --all -- --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test` once each (not while scripts/deploy.sh builds); fix each failure in the owning todo's files (no weakened assertions, no #[ignore]). Record exit codes.
  3. Run every cargo-test QA named in todos 1-8 with `-- --nocapture` and the binary QAs QA-3, QA-5, QA-8; write command, exit code and output to .omo/evidence/dori-parity-fixes/task-<n>.txt.
  4. Read-only prod checks on the live DB (/Users/indo/code/project/omon-gateway/omon_gateway.db; confirm with `plutil -extract EnvironmentVariables.DATABASE_URL raw ~/Library/LaunchAgents/ai.omon.gateway.plist` and fall back to that path if the key is absent): `sqlite3 -readonly <db> "select count(*) from lanes where done_signals like '%sha256=%' or lower(done_signals) like 'done%=%'"`; list the open pause rows with lane status; list distinct `cwd` of lanes created in the last 30 days and for each run `git -C <cwd> remote`, then record how many cwds have no remote. A nonzero count goes into the final report as "these lanes will now be refused until a remote exists".
  5. Teardown: kill the dashboard and any service PIDs (check `kill -0` fails), `rm -rf $RUN_TMP`, write the receipt to .omo/evidence/dori-parity-fixes/task-9.txt.
  Must NOT do: no deploy, push or merge; never write to the live DB.
  Closes: verification for GAP-1..GAP-9
  Parallelization: Wave 4 | Blocked by: 1-8 | Blocks: F1-F4
  References: Cargo.toml, scripts/deploy.sh:20-45, ~/Library/LaunchAgents/ai.omon.gateway.plist, all files above
  Acceptance criteria: fmt, clippy and test exit 0; every task-<n>.txt exists and shows the asserted values.
  QA scenarios: as listed; any mismatch reopens the owning todo.
  - Evidence: .omo/evidence/dori-parity-fixes/task-9.txt
  Commit: N
  Recommended task executor category: unspecified-high - integration of all changes, full suite and live CLI QA.

## Final verification wave
> Runs in parallel after ALL todos. ALL must APPROVE. Surface results and wait for the user's explicit okay before declaring complete.
- [ ] F1. Plan compliance audit
  - Every todo's named files changed; migrations 0055 and 0056 exist exactly once; `rg -n "kibitzer::DEFAULT_LEAD" src/lanes/runner.rs src/agent/omo_backend.rs` empty; `rg -n "is_lane_paused" src/lanes/settle.rs` empty.
- [ ] F2. Code quality review
  - Review `git diff $(cat .omo/evidence/dori-parity-fixes/base.txt)..HEAD` against Must NOT have (no Paused enum, no emoji edits, no polling loop beyond the bounded CLI confirm, no engine/.env edits, no weakened assertions).
- [ ] F3. Real manual QA
  - Re-run QA-3, QA-5, QA-8 and the todo 1-2 cargo QAs from todo 9 on the final build and compare with the recorded outputs.
- [ ] F4. Ideal-state fidelity
  - For every IS row, the evidence file in Success criteria exists and contains the asserted value; a missing row becomes a new todo.

## Commit strategy
- One commit per todo 1-8 (todo 9 none), on the task worktree branch created by ulw-execute, Conventional Commits matching `git log --oneline -20` (fix(lanes)/feat(lanes)/feat(tracker)).
- Stage only the files each todo names. Footer: `Plan: .omo/plans/dori-parity-fixes.md`.
- The owner's approval of this plan and an explicit /ulw-execute start count as the instruction to commit on the task branch. Push, merge to main and deploy (scripts/deploy.sh from main) wait for a separate owner instruction.

## Success criteria
| IS | Delivering todo(s) | Proving QA scenario | Evidence |
| --- | --- | --- | --- |
| IS-1 | 1 | todo 1 cargo QA: no-remote refused, empty repo and pushed pass | .omo/evidence/dori-parity-fixes/task-1.txt |
| IS-2 | 2 | todo 2 cargo QA: paused claim verified/not_done, pause kept | .omo/evidence/dori-parity-fixes/task-2.txt |
| IS-3 | 3 | test i confirmed exit 0; QA-3 binary exit 3 PAUSE_UNVERIFIED; test l NOT_PAUSABLE | .omo/evidence/dori-parity-fixes/task-3.txt |
| IS-4 | 3 | test j exactly one working_while_paused event | .omo/evidence/dori-parity-fixes/task-3.txt |
| IS-5 | 8 | QA-8 Paused and back; closed lane refused 409 | .omo/evidence/dori-parity-fixes/task-8.txt |
| IS-6 | 4 | lane_saturation_alert_once_then_clear | .omo/evidence/dori-parity-fixes/task-4.txt |
| IS-7 | 5 | QA-5 binary: Done-prefix stored stripped, short sha exit 2 | .omo/evidence/dori-parity-fixes/task-5.txt |
| IS-8 | 6 | rg empty + runtime_config child-process test | .omo/evidence/dori-parity-fixes/task-6.txt |
| IS-9 | 7 | permanent/transient/ordinary runner tests | .omo/evidence/dori-parity-fixes/task-7.txt |

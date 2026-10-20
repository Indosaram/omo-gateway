# Post-deployment verification

The deployed revision is `1c992e23`, matching main and the remote main commit
`1c992e238d6bb453f3b674bd471b379f4be87c3e`.
The deployment marker records `dirty_paths=8`; post-commit evidence edits were
present, so this is not a clean-worktree deployment claim.

Direct checks after deployment:

- App-server `http://127.0.0.1:19742/readyz`: HTTP 200.
- Dashboard `/api/ready`: status ready; backend, chat_runtime, database, disk,
  and workspace checks all true.
- `codesign --verify --strict ~/.omon/bin/omo-gateway`: exit 0.
- Signature TeamIdentifier: `5DUM8WPB4C`, runtime flag enabled,
  timestamp October 20, 2026, 20:10:28 local time.
- Gateway log at `2026-10-20T11:10:35.546370Z`: `slack socket: connected`,
  after restart at `11:10:31Z`.
- Read-only production SQLite query: migration version 54.
- Subsequent read-only query: mail inbox count 1; action-loop outcomes contain
  3 enqueued rows. Neither count proves completed autonomous work.

## Counterexample preventing completion

At `11:11:15.491931Z` the deployed backend received a busy refusal, then at
`11:11:15.494190Z` reported idle and resubmitted. At `11:11:16.182517Z`,
the actor failed again with `Agent is already processing`.
This is an observed A7 failure despite the shipping gate passing.
The backend worker is preparing a correction and deterministic regression.
The correction is not yet validated or deployed.

A1/A5/A14 still require an actual automatic external-result chain. No verified
outcome was observed in the queries above. The outcome log subscription is active.

Shipping the correction also needs a decision on the single-commit constraint:
the original commit is already pushed. No amend or forced push is authorized.
The question tool reported no attached user and supplied no answer; that operation
remains pending rather than implicitly approved.

## Follow-up deployment 610bf915 (2026-10-20 21:21 KST)

Commit `610bf915842a2344e3f67bbb39fef3748c39ae43` was pushed to main as a follow-up
commit; `1c992e23` was already on the remote, so no amend or forced push was made.
Gate `review-20261019/gate/review-20261020-postfix2`: GATE-PASS (fmt/clippy/test/web 151 pass 0 fail/tsc/build),
and the two changed source files hashed identically before and after the gate.
The earlier `postfix` run failed one paused-time loopback test while the file was
still being edited; that test now runs on real time and passes.

- A7: turn ids already seen completed, and the refused turn's own failure frames,
  no longer release the busy wait; retries follow an explicit busy response with
  backoff bounded by the turn deadline. Tests in `review_20261020_agent_tests`.
- Autonomy delivery: `action_loop` events are critical in `src/lanes/dispatch.rs`,
  so the wake judge can no longer record triaged items as failed (11 items at
  11:18Z/11:30Z were lost that way).

Deploy evidence: `DEPLOYED rev=610bf915 dirty_paths=0`, deploy.sh `deploy OK`,
readyz 200, dashboard ready, codesign strict OK with TeamIdentifier 5DUM8WPB4C
(timestamp 21:21:33), `slack socket: connected` at 12:21:40.857967Z.

Still to prove in production: no A7 recurrence, and one triaged item reaching
`verified` with a calendar readback. A log monitor is active for both.

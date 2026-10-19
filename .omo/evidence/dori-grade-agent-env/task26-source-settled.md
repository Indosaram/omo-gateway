# Task 26 source-settled evidence

Worktree: `/Users/indo/code/project/omon-gateway-wt/dori-work`.

## Scope and requirements

Read todo 26 in the canonical plan, including What to do, Must NOT do,
acceptance, QA and dependencies on todos 8, 24, 20 and 25.
The newer task instruction overrides the optional automatic lookup provision:
no action executes without an explicit owner reply, even with AUTO_LOOKUP true.
No commit is made despite the plan's Commit entry.

Owned changes:

- `src/inbox/mod.rs`: exports the proposal module only.
- `src/inbox/proposals.rs`: persistent numbered snapshots, presentation binding,
  explicit owner approval, atomic execution claim, lane and direct action bridge.
- `tests/test_inbox_proposals.rs`: nine focused tests with scratch in-memory
  SQLite and fake actions only. The fake lane starter inserts a real scratch
  lane row through the existing storage interface.
- `task26-wiring.patch`: unapplied commands/main/adapter/entry hunks.

No edits to lane runner, lane CLI, lane storage, Discord adapter, main, or
todo 25 list/set/digest files. No live database, service, model, Discord,
environment mutation, or git commands were used.

## Approval and effect contract

Proposals snapshot bot-class `proposed_action`, source link, trusted action
route, owner and channel. Numbers persist across relisting and service restart.
Only an exact positive-number `N 해` reply from the owner can execute, and it
must reference the bot presentation message in the same channel. Unknown
numbers, bot authors, other authors, ambiguous text, missing/wrong references,
changed decisions and generic status-only approval do not start work.

A conditional SQLite update claims execution before any action is invoked.
Approval status, approval-message evidence and deterministic lane key are
committed first. Duplicate/concurrent approval and failed/crashed execution
never automatically retry. A crash after claim needs manual diagnosis; pending
execution is not silently replayed on startup.

Repo work uses the existing lane Start API and its guard/model-routing behavior,
with the source link in the brief. Lookup is bounded read-only HTTP GET; drafts
use a tool-free model request and return text to the owner for copy/send.
There is no owner-account send, source reply or reaction API in this module.
`inbox_decisions.outcome` stores structured execution outcome; `lane_key` stores
the lane association. Repo outcome `completed` means lane-start handoff
completed, not that the lane's done protocol has verified the actual work.

## Wiring handoff

The patch is intentionally not applied. Its CLI adds `inbox-propose <item-id>
--channel <owner-dm> --action-json <typed-route>` for the lead to prepare a
proposal, not approve one. Example routes are `{"kind":"draft"}`,
`{"kind":"lookup"}`, and `{"kind":"repo","cwd":"/repo","done":"file /repo/result"}`.
Routes must come from trusted lead configuration, not arbitrary source text.

After applying the patch, the owner asks `제안` in the primary bot's DM to see
the numbered list, then replies to that message with `N 해`. The adapter consumes
approval replies directly rather than letting a persona/model infer consent.
The patch initializes GatewayActions and the proposal schema at startup.
Todo 25 remains responsible for its existing list/set/digest surfaces; its
presentation integration must use the same proposal numbers and call
`presented` only after delivery. A bare `inbox set --status approved` is not an
execution capability. No approval CLI accepts an asserted owner identity.

## Verification

- Scoped `rustfmt --check --edition 2021 --config skip_children=true` on
  `src/inbox/mod.rs`, `src/inbox/proposals.rs`, and the focused test: exit 0.
- Explicit LSP diagnostic requests on proposal and test files: unavailable,
  daemon requests timed out after 30000 ms. This is not a clean LSP claim.
- `patch --dry-run -p1 -i .omo/evidence/dori-grade-agent-env/task26-wiring.patch`:
  exit 0 for all four target files. No hunks applied. The patch is not compiled.
- One test command is subscribed through a 1800-second monitor:
  `cargo test --test test_inbox_proposals --target-dir target-w26 -j2 --offline`.
  Monitor: `mon_MYV91EFS2QQKE16R`, session `bash_1`. Result pending.

The acceptance and QA scenarios are exercised in scratch SQLite: owner approval
creates a lane row with linked brief, no approval creates no lane, both auto
flags create no action, direct results return only owner text, and concurrency,
restart/failure and stale-decision safeguards are covered. Live debug-gateway QA
is deliberately not run because the task expressly limits tests to scratch
SQLite and fakes and prohibits live services/Discord.

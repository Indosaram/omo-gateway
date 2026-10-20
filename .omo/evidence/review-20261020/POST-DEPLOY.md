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

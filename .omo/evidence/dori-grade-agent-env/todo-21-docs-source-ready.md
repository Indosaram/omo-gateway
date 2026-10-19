# Todo 21 documentation evidence

## Scope and outcome

Todo 21 rewrites `docs/AGENTS_DELEGATE.md` as the lane operator guide. This is a docs-only artifact for the todo21 scope. It is source-ready, not global confirmation and not a claim that planned lane lifecycle work is complete.

The main plan was read at `/Users/indo/code/project/omon-gateway/.omo/plans/dori-grade-agent-env.md`, exact todo 21. It requires runtime explanation, CLI coverage, done grammar, statuses, kill/env/guard/watch/troubleshooting/rollback guidance, and an explicit not-used mention at most once. Todo 11 evidence at `.omo/evidence/dori-grade-agent-env-todo11-gate-review.md` confirms the wrapper preserves literal inline `--brief` instructions and forwards lane start argv. The guide does not replace or redefine that wrapper contract. The wrapper's inline `--brief` argument remains literal task instructions, written to its lane brief file before invoking `lane start`. No code, tests, build files, or `docs/research/*` were edited.

## Source facts used

- `src/lanes/cli.rs` defines ten commands: `start`, `list`, `log`, `send`, `claim-done`, `object`, `close`, `reopen`, `can-launch`, and `heavy`.
- `src/agent/omo_backend.rs` and `src/lanes/runner.rs` establish OMO app-server-backed lane turns and persisted `omo_thread_id` continuity.
- `src/lanes/signals.rs` defines `merged`, `closed`, `published`, `command`, `file`, and `url` signals. Commands use JSON argv arrays.
- `src/lanes/state.rs` defines eight statuses: `queued`, `working`, `blocked`, `done_claimed`, `not_done`, `verified_done`, `closed`, and `failed`.
- `src/lanes/guard.rs` provides current capacity checks and defaults: 150 load alert, 80 load-ok, 20 percent memory, 50 GB disk, six concurrent lanes and one heavy slot.
- `src/agent/omo_config.rs` sets a 3600-second default lane-turn timeout. `src/lanes/runner.rs` sets a 24-turn default and fails after three consecutive turn errors.
- `src/main.rs` enables the lane service by default when `OMON_LANES_ENABLED` is unset.
- `src/dashboard.rs` has no lane-specific view or route. `src/lanes/runner.rs` has no five-minute settle loop. The `GuardWatcher` definition is not wired into a running service in the inspected references. No `close_failed` status exists.
- Plan targets that are not established in this snapshot are labeled pending in the guide: automatic done settlement, automatic queued retry, guard watcher alerts, dashboard lane view, thread archival/cleanup and `close_failed`.

## CLI binary check

The expected warm binary, `target/debug/omo-gateway`, is absent from the worktree, so exact help output and binary version could not be checked without a build. No Cargo build was run, per scope. Source command coverage is listed above. The guide requires `omo-gateway lane --help` and per-command help to be checked against the deployed version before relying on a changed binary. Exact runtime CLI acceptance remains pending a version-matched binary.

## Legacy mention count

The guide includes one line explicitly stating the legacy herdr pane manager is not used. The exact acceptance check is:

```sh
grep -ci herdr docs/AGENTS_DELEGATE.md
```

Expected count: `1`, the single explicit not-used statement. No other mention is intended.

## Nine applicable / N/A cleanup classes

| Class | Disposition |
| --- | --- |
| Malformed input | Applicable. Source validates required key/title/brief/signals and command-specific text. Error behavior is described without promising exact CLI help output. |
| Prompt injection | Applicable. The lane `command` signal is documented as JSON argv, not a shell string. The guide does not change the wrapper's literal inline `--brief` contract from todo11. |
| Cancel/resume | Applicable. `reopen` behavior and service rollback are described; no worker kill CLI is claimed. |
| Stale state | Applicable. Database consistency and status inspection are documented. Automatic guard and claim recovery remain explicitly pending. |
| Dirty worktree | Applicable to planned close semantics. The guide does not claim dirty-tree validation or cleanup from `closed`; check cleanup separately. |
| Hung commands | Applicable. Per-turn timeout and heavy command wait behavior are described. Signal checker timeouts and daemon recovery are not claimed beyond source evidence. |
| Flaky tests | N/A. This is prose-only; no tests were added or changed. |
| Misleading success output | Applicable. CLI success, queued exit code, claim state and unverified settle are distinguished. |
| Repeated interruptions | N/A. No retry, interruption or lifecycle code was changed. |

## Verification and cleanup

Machine-coverage verification passed for all ten source command names, the required signal kinds, statuses, environment keys, watch/rollback sections and exactly one legacy not-used line. `git diff --check` passed for both scoped artifacts. A scratch copy with the troubleshooting section removed failed the section-presence check as expected; the scratch copy was removed, and the user's guide remained intact. Exact CLI output coverage is still pending because no version-matched `target/debug/omo-gateway` binary exists, and no build was run. No live database, service, deployment, commit, or unrelated environment changes were used.

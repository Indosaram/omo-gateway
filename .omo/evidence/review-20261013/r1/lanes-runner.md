# lanes-runner review

### [P2] Reopen renames the thread before unarchiving it
- Location: /Users/indo/code/project/omon-gateway/src/lanes/runner.rs:209
- Trigger: A closed lane that has a Discord thread is reopened, either by an owner reply or by the CLI reopen. `thread_effect(close=false)` calls `transport.status(thread, Working)`, which does a rename PATCH on the thread. Only after that (line 213) does it call `archived(thread, false)`. At that point the thread is still archived from the close.
- Impact: This depends on Discord rejecting a name edit on an archived thread. The repo's own comment at posts.rs:274 says Discord rejects message edits on archived threads (50083). I did not confirm it for thread PATCH. If Discord does reject it, `result.map_err(...)?` at line 209 fails the whole reopen. The owner reply then gets an error stored and is parked (see the next finding), and the lane stays closed. Only the timeout case is tolerated there; an API error is not.
- Fix: Unarchive first, then rename. Mirror the close path, which renames and then archives. Treat a rename failure as non-fatal and log it, as the timeout case already does.

### [P2] A failed owner reply is parked forever with no signal
- Location: /Users/indo/code/project/omon-gateway/src/lanes/runner.rs:846
- Trigger: A reply fails once, for example on a 15s lifecycle timeout, a Discord error, or a git error in `restore_cwd`. The error branch (line 868) stores `error=?` on the row. The selector at line 846 only picks up `state='pending' AND error IS NULL`.
- Impact: The warn log says "owner reply remains pending", but nothing retries it. Only a new owner reply clears the error (storage/lanes.rs:249). The lane stays closed and the owner is not told.
- Fix: Retry errored replies with backoff or an attempt cap. Alternatively enqueue a lead/owner event when a reply is parked.

### [P2] Free-text `lane_history.reason` can forge the `'reopened'` generation marker
- Location: /Users/indo/code/project/omon-gateway/src/lanes/activity.rs:337
- Trigger: `block()` writes `lane_history.reason` from the agent's `[REPORT] key | blocker | <text>` text. `change_claim` (cli.rs:541) does the same with the claim evidence or objection text. The `close_intent` loop in runner.rs (below line 802) does the same with `--note`. Every generation fence in these files reads `MAX(id) FROM lane_history WHERE reason='reopened'`. An agent report or operator note that is exactly `reopened` inserts a fake marker.
- Impact: The generation jumps. The running turn is cancelled and its writes are fenced off. Pending close intents become "stale close intent". Pending owner replies fail with "stale reopen reply" or "reply generation mismatch". The trigger needs an exact string match, so it is unlikely, but it is a sentinel injectable by untrusted agent text.
- Fix: Use a dedicated column or a reserved reason value that user-supplied text cannot reach. At minimum, prefix free-text reasons, for example `format!("report: {text}")`.

### [P2] A crash during worktree removal or restoration leaves a state the code cannot recover from
- Location: /Users/indo/code/project/omon-gateway/src/lanes/runner.rs:515
- Trigger: The process dies, or the DB write at line 536 fails, after `state='restoring'` is set (line 515) and `git worktree add` has run. A failed `worktree_proof` or mismatch check after the add leaves the same state. A crash after a worktree is set to `removing` (line 423 area) in `remove_owned` is similar.
- Impact: In the restoring case, the next `restore_cwd` sees `state != 'removed'` and fails with "requires verified removal receipt". The lane can never be reopened until someone edits the DB by hand. In the removing case, reopen is blocked the same way if the removal crashed mid-way and the lane is then reopened.
- Fix: Handle `restoring` explicitly. If the path is a registered worktree whose proof matches, finish the transition to `registered`. Otherwise roll back to `removed`. Roll back when the proof check fails after the add.

### [P2] One failing cron_problem event aborts the whole dispatch tick
- Location: /Users/indo/code/project/omon-gateway/src/lanes/dispatch.rs:53
- Trigger: `crate::cron::remediate::handle_event(...).await?` (line 54) returns Err, for example from malformed JSON, a missing `problem_id`, or `on_problem` failing. `on_problem` retries cleanly on the next tick in most cases, because `record_failure` sets `needs_owner` and the second pass returns Ok(true). It does not recover if `record_failure` itself fails, for example when the `cron_problems` row is missing. I did not check whether that row can be deleted.
- Impact: `tick` returns before any group is delivered, so lane events for every lead are blocked. That includes `dead` and `done_claimed` events. The block lasts as long as the event stays undelivered. Remediation also runs inline in the tick, with judge calls of up to 30s, which delays delivery further.
- Fix: Catch the error per event. Log it and `continue`. Mark a permanently failing event as delivered with a failure reason.

### [P2] Duplicate transcript row when an item is replayed
- Location: /Users/indo/code/project/omon-gateway/src/lanes/runner.rs:634
- Trigger: The same completed item is dispatched twice with the same `EditMessage` identity (`lane-item:{turn}:{id}`). `captured_item` dedupes through the `activity_item` marker, but its returned decision is ignored. The `INSERT INTO lane_transcript ... 'agent'` at line 637 runs unconditionally. `captured_item` and that insert are also separate commits, so a failure between them loses the transcript row permanently, because the next attempt dedupes.
- Impact: A replayed `[REPORT] ... done` line is re-read from the transcript. The turn-end scan then sets `idle_without_claim` and parks the lane. This depends on the backend actually replaying items.
- Fix: Skip the transcript insert when `captured_item` reports the item as already seen, for example by returning a `seen` flag. Do the transcript insert in the same transaction as the marker.

### [P2] Predictable wake-socket and lock paths in `/tmp`
- Location: /Users/indo/code/project/omon-gateway/src/lanes/runner.rs:660
- Trigger: `socket_path` derives `/tmp/omon-lanes-<hash>.sock`, and the lock file next to it, from the canonical DB path. Another local user pre-creates either file. `/tmp` has the sticky bit, so `WakeSocket::bind` cannot `remove_file` the socket (and cannot take the lock on a file they hold).
- Impact: The lane service fails to start, a local denial of service. Any local user can also send wake datagrams, which is harmless.
- Fix: Put the socket and lock in a per-user directory, such as `$XDG_RUNTIME_DIR`, or in a 0700 directory next to the DB.

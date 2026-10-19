# lanes-runner review

### [P1] Transient settlement check error kills the whole lane service
- Location: src/lanes/settle.rs:93
- Trigger: A lane on a remote external target runs `claim-done`. Five minutes later `LaneRunner::serve` starts `settler.settle_due()` on the settle tick (runner.rs settlements.spawn). That call reaches `crate::sessions::external::remote_checks(&self.pool, &lane).await?`. Inside it, `SessionControl::output` has a 20s timeout, and an unreachable host or slow ssh fails it with `map_err(invalid)?`. A cron remediation lane has the same path at settle.rs:99, `crate::cron::remediate::settle_lane(&lane.key).await?`: if the cron job was deleted (`CronScheduler::delete`), `remediator.job()` fails with RowNotFound, and so does any `verify_once` error. In each case the `?` makes `settle_due` return Err, and serve treats that as fatal at runner.rs:1036 (`Ok(Err(error)) => { return Err(error); }`).
- Impact: One lane's failed check stops every lane task, settlement, close processing and post draining. In headless mode main.rs `lane_service_exit` exits the process. Remaining done_claimed lanes in that pass are not settled. After restart the `settlement_started` marker forces "settlement interrupted" -> not_done, so the next claim of the same lane crashes the service again whenever the cause persists (deleted cron job, offline remote host).
- Fix: Turn per-lane check errors into rejection reasons instead of propagating them: `match remote_checks(..).await { Ok(v) => .., Err(e) => reasons.push(e.to_string()) }`, and do the same for `settle_lane`. Optionally, in serve, log a settlement `Ok(Err(_))` instead of returning.

### [P2] Unpushed-commit check scans every local branch, so concurrent lanes in the same repo block each other
- Location: src/lanes/settle.rs:485
- Trigger: Two lanes work in linked worktrees of the same repository, which share refs/heads. Lane A claims done while lane B's branch has commits not yet pushed. `worktree_reasons` runs `git log --branches --not --remotes` in A's worktree, which lists B's commits too.
- Impact: Settlement marks A not_done with "unpushed <path>" even though A's own branch is pushed. A keeps being told to fix something it does not own and can burn turns up to the turn cap and fail. Any stale local branch in the control repo has the same effect on every lane.
- Fix: Limit the check to the worktree's checked-out branch, e.g. `git log HEAD --not --remotes --oneline` (or `@{u}..HEAD` when an upstream exists).

### [P2] Owner reply that fails once is never retried, so the reopen is silently dropped
- Location: src/lanes/runner.rs:846
- Trigger: The owner replies in a closed lane thread, and `record_owner_reply` stores it as pending. `apply_owner_replies` (called on every serve iteration) then fails transiently, for example when `thread_effect` unarchive hits the 15s `LIFECYCLE_TIMEOUT`, Discord returns an error, or `restore_cwd` fails on git. The error branch stores `error=?`, but the select only picks up `state='pending' AND error IS NULL`.
- Impact: The reply is never applied again, despite the "owner reply remains pending" log. The lane stays closed until the owner happens to post another reply, which is the only thing that clears `error` (storage/lanes.rs:249). The owner gets no signal that the reopen did not happen.
- Fix: Retry errored pending replies with backoff (select rows with `error IS NOT NULL` after a delay or attempt count), or send a lead/owner event when a reply is parked with an error.

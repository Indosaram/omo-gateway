# Lane lanes

### [P1] Host guard stops re-alerting when an alert lasts more than 24h
- Location: src/lanes/guard.rs:658
- Trigger: Disk free stays under `disk_free_min_gb` for more than 24h without recovering. That is common, because the default threshold is 50 GB and a dev host can sit below it for days. The watcher stays in `Alert`. If disk then falls another 10+ GB, or memory crosses its threshold for the first time, the Alert branch only re-alerts while `now - mark.at < GUARD_REALERT_WINDOW`, so nothing is emitted. `worth_alerting` would have returned true at that point, both from the window rule and from the new-metric rule.
- Impact: The lead never gets a `guard` ALERT for a disk that is filling up or for a new memory shortage, for as long as the original episode lasts. The longer the alert has run, the quieter the guard gets.
- Evidence:
  ```
  if !alert_reasons.is_empty()
      && self.last_alert.is_some_and(|mark| {
          now.saturating_duration_since(mark.at) < GUARD_REALERT_WINDOW
      })
      && self.worth_alerting(load_alert, mem_alert, disk_alert, sample, now)
  ```
  The doc comment says the margin and new-metric rules apply "while in Alert", but this window guard turns them off after 24h.
- Fix: Drop the window conjunct and compute `worth_alerting(...)` without the `>= GUARD_REALERT_WINDOW` early return. For example, pass a flag that ignores elapsed time, so the margin and new-metric rules still apply after 24h and time alone still never re-alerts.

### [P2] A pause survives close and reopen, so a reopened lane never runs
- Location: src/lanes/runner.rs:122
- Trigger: Run `lane pause k`, then `lane close k --force --note "forced: ..."`, which succeeds because `request_close` does not check pauses. Later, reopen the lane with `lane reopen k` or an owner reply in the Slack lane thread. `apply_reopen` sets `status='working'`, but the `lane_pauses` row still has `resumed_at IS NULL`. The service loop skips it (`if paused(...) { continue; }`), and the stale check excludes paused lanes, so no stale post appears either.
- Impact: The owner reopens the lane (for example by replying in its thread) and it sits silently in `working` until someone notices `paused: true` in `lane list` and runs `lane resume`.
- Evidence:
  - runner.rs:122: `UPDATE lanes SET status = 'working', turns_run = 0, ... WHERE key = ? AND status = ?`
  - No code clears `lane_pauses` on close or reopen. The only writers are `pause_lane` and `resume_lane` (storage/lanes.rs:254, :267).
- Fix: In `apply_reopen`'s transaction, and/or when a close completes, run `UPDATE lane_pauses SET resumed_at = ? WHERE lane_key = ? AND resumed_at IS NULL`.

### [P2] A failed auto-worktree registration leaves a worktree nothing will clean up
- Location: src/lanes/cli.rs:552
- Trigger: After the lane row commits, `register_owned_worktree` fails, for example because the `worktree_proof` identity check fails or because of the "control repository intersects registered removal targets" check. Start only records a warning.
- Impact: The lane runs in `<repo>-lanes/<key>` on branch `lane/<key>`, but nothing tracks that tree. Close skips removal ("unowned external cwd"), the startup sweep only looks at registered trees, and the worktree and branch stay on disk indefinitely.
- Evidence:
  ```
  if let Err(error) =
      super::runner::register_owned_worktree(pool, &lane.key, &tree.repo, &tree.path).await
  {
      tracing::warn!(key = %lane.key, %error, "auto worktree registration failed");
      warnings.push(format!("auto worktree not registered: {error}"));
  }
  ```
- Fix: Register in the same flow before reporting success. On failure, either fail the start (delete the row and call `discard_auto_worktree`), or record the unregistered path as a lane event so the owner can reconcile it.

### [P2] Auto-worktree lane work lands only on an unmerged `lane/<key>` branch
- Location: src/lanes/runner.rs:496
- Trigger: `lane start --cwd <repo root>` with `OMON_LANE_AUTO_WORKTREE` on (the default). The lane commits in `<repo>-lanes/<key>` on `lane/<key>`. Settle and close then remove the worktree (`remove_owned` keeps the branch), and nothing in `src/lanes` merges or pushes that branch. A grep for any merge step in the lanes code found none.
- Impact: The lane reports `verified_done`/closed, but the change is missing from the owner's main checkout unless the brief itself told the lane to push or open a PR. Nothing is lost (the branch survives), but the result looks like work that went missing.
- Evidence:
  ```
  let branch = format!("lane/{key}");
  let target = path.to_string_lossy().into_owned();
  git(&repo, &["worktree", "add", "-b", &branch, &target, "HEAD"]).await?;
  ```
- Fix: Put the branch name in the done/close lane event (and the Slack close reply), or add an explicit merge or PR step to the close path.
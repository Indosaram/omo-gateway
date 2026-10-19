# Lane lanes-runner

### [P1] Lane turns start with a quiet-check tick that can fire right away and post a false "stale" alert
- Location: src/lanes/runner.rs:1337
- Trigger: A lane sits for more than 20 minutes with no turn running. This happens when it is `blocked` waiting for the lead, idle on `idle_without_claim` waiting for the owner, or when the gateway was down for more than 20 minutes. Then a new inbox message arrives (for example `lane send`, which moves `blocked` to `working` and wakes the lane), or the service restarts, and `run_lane` starts a new turn.
- Impact: The first quiet check runs as soon as the turn starts. It sees `last_activity_at` from the end of the previous turn, which is more than 20 minutes old, while `stale_posted_at` is still NULL because `capture` cleared it on the last output. So `quiet_generation` immediately:
  - queues a `stale` lane event to the lead;
  - queues a `stale:` thread post, which goes out to the lane's Discord thread.
  The turn has only just started, so both are false. Answering a blocked lane late, or a normal owner reply after an idle period, is enough to trigger it.
- Evidence:
  - `let mut quiet_check = tokio::time::interval(std::time::Duration::from_secs(60));` A tokio `interval`'s first tick completes immediately.
  - In the `biased` select, `_ = quiet_check.tick() => { ... activity.quiet_generation(key, true, Some(generation)).await?; ... self.posts_ready.notify_one(); }` (runner.rs:1367) runs before the turn has produced anything.
  - The quiet query in activity.rs:301 matches on `status = 'working' AND stale_posted_at IS NULL AND last_activity_at <= ?` with `now - 20min`.
  - Nothing in the turn-start path updates `last_activity_at`. A grep finds it written only by `capture`, guard launch, and external adopt. `transition_lane` (used by `lane send`) changes only status and updated_at.
- Fix: Do either of these:
  - Start the timer one period later: `tokio::time::interval_at(Instant::now() + 60s, 60s)`.
  - Or, before spawning the turn, run `UPDATE lanes SET last_activity_at = now, stale_posted_at = NULL WHERE key = ? AND <generation fence>`, so the 20-minute quiet window counts from turn start.

### [P2] The event dispatcher runs cron remediation inline, including a full cron job rerun, which holds back every lead's lane events
- Location: src/lanes/dispatch.rs:55
- Trigger: A `cron_problem` event for a non-publishing job that the judge classifies as `transient_retry_once`. `handle_event` then calls `on_problem`, which calls `verify_once`, which calls `runtime.rerun`, which calls `scheduler.trigger_wait`. That call runs `execute_claim(claim).await`, the entire cron job, possibly an agent turn lasting minutes. All of this happens inside `EventDispatcher::tick`.
- Impact: While the rerun runs, the dispatcher's only loop is blocked. No other lead gets its lane events: `done_claimed`, `dead`, `not_done`, `owner_reply_failed`, and kibitzer wakes are all delayed for the length of the job, up to the cron timeout. Events are delayed, not lost. The code's own intent ("One bad cron problem must not block every other lead's events") is not met.
- Evidence: `match crate::cron::remediate::handle_event(&event.body, &event.lead_session_key).await` sits inside the `for event in take_undelivered_events(...)` loop of `tick`. In remediate.rs:328 the transient branch runs `self.verify_once(id).await?`, and remediate.rs:469 runs `let receipt = self.runtime.rerun(&job.id).await?;`.
- Fix: Run `handle_event` in a separate task, and receipt the event when that task finishes. Alternatively, have the dispatcher only record the problem for the remediator's own worker, so `tick` never waits on a rerun.
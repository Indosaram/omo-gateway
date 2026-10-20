# Lane cron

I reviewed every hunk of `cron.diff` (`src/cron/executor.rs`, `src/cron/scheduler.rs`, `src/cron/store.rs`). Each finding below was checked against the current files and the wiring in `main.rs`. In production the scheduler is built with `.with_output_review()` at `src/main.rs:1570`, and the LaunchAgent sets `OMON_HOME_PLATFORM=slack` and `SLACK_HOME_CHANNEL`, so the review path is live. I found no P0 issues: 4 P1 and 3 P2.

### [P1] Cron reports from different jobs merge into one lead turn, and all but the newest are dropped
- Location: src/cron/scheduler.rs:2433
- Trigger: Two jobs with destinations finish while the lead session is busy, or inside the same 5s dispatch tick or 30s rate-limit window. Both `cron_report` events are queued for the same lead with `lane_key = NULL`.
- Impact: `EventDispatcher::deliver_group` groups events by `(lane_key, kind)` and keeps only the newest body. It still marks every source id as delivered (`lead_turn`). The older job's report is never seen by the bot, so it is silently dropped. The same happens when one job has several destinations that share a lead, such as Discord destinations, which all fall back to the home lead.
- Evidence: `crate::storage::lanes::enqueue_lane_event(&self.pool, None, &lead, "cron_report", &body)`. In `src/lanes/dispatch.rs:116`: `.entry((event.lane_key.clone(), event.kind.clone()))`, and at :120 `entry.0 = event;` ("judge and render only the newest body").
- Fix: Pass a key per job, `enqueue_lane_event(&self.pool, Some(&job.id), &lead, "cron_report", &body)`. `lane_events.lane_key` has no foreign key (`migrations/0031_lanes.sql:36`). If one job has several destinations, also add the destination to the key.

### [P1] With no `OMON_LANE_DEFAULT_LEAD`, every cron delivery fails
- Location: src/cron/scheduler.rs:2640 (error raised at :2412)
- Trigger: This hits a Discord-home deployment where `OMON_LANE_DEFAULT_LEAD` is not set. That is the default: only `boot_slack` sets it, and only when Slack is home and the owner DM resolves. It also hits a Slack-home boot where `auth.test` or the owner-DM lookup fails, so the variable stays unset. In either case, any job with a destination and non-silent output fails.
- Impact: `review_session` returns `Ok(None)` and `queue_review` errors with "no bot conversation for cron review". The run is recorded as failed with `delivery_failed = true`, and nothing reaches any messenger. The failure notification takes the same review path and fails too, and is only logged. This is a regression: before this change these jobs were delivered directly. Every other lead reader falls back to `kibitzer::DEFAULT_LEAD`, for example `health::lead_from` at `src/cron/health.rs:293-297`.
- Evidence: `let home = std::env::var("OMON_LANE_DEFAULT_LEAD").ok().filter(|lead| !lead.trim().is_empty()); if !destination.platform.eq_ignore_ascii_case("slack") || destination.chat_id.starts_with('D') { return Ok(home); }`
- Fix: Use `.unwrap_or_else(|| crate::lanes::kibitzer::DEFAULT_LEAD.to_owned())` as `health::lead_from` does. Alternatively, have `reviews_before_delivery` return false when no lead resolves, so the job falls back to `deliver`.

### [P1] `ack_command` runs when the review is queued, before anything is delivered
- Location: src/cron/scheduler.rs:2298 (the review branch is at :2271)
- Trigger: A job has `ack_command` and a destination, and the review path is on (the production default). The job produces output.
- Impact: The scheduler treats a successful `queue_review` as a successful delivery and runs the ack at once. The report may never reach anyone: the bot can answer `[SILENT]`, the judge can skip it, or it can be collapsed as in the first finding. The upstream items are still acked or consumed, so source data is lost without anyone being told. This breaks the documented contract: "The scheduler runs a job's `ack_command` itself, after delivery succeeded."
- Evidence: Line 2271 has `let sent = if self.reviews_before_delivery(&payload) { self.queue_review(job, destination, content_str, true).await } else { ... };`, then line 2298 has `super::ack::run_ack_logged(&ack_command).await;`.
- Fix: Skip review for jobs that have an `ack_command`, so `reviews_before_delivery` returns false when `payload["ack_command"]` is non-empty. Otherwise, defer the ack until the bot's reply has been sent.

### [P1] Report files in `~/.omon/cron-reports` are never cleaned up
- Location: src/cron/scheduler.rs:2424 (directory set at :2413)
- Trigger: Every reviewed success or failure of every job that has a destination writes one new file.
- Impact: The directory grows without limit and holds full, unredacted output. A job every 5 minutes writes 288 files a day. Nothing in `src/` ever deletes them: `cron-reports` appears only here.
- Evidence: `let path = dir.join(format!("{}-{}.txt", job.id, (self.clock)().format("%Y%m%dT%H%M%S%.3f"))); tokio::fs::write(&path, content).await`
- Fix: Prune at write time, for example by deleting files in the directory older than N days or keeping the last K per job. Alternatively, prune them alongside `prune_terminal_cron_runs`. Write `redact_owner_tokens(content)` as `deliver` does for its mirrors.

### [P2] The out-of-window miss check never matches interval jobs
- Location: src/cron/scheduler.rs:1451
- Trigger: An `interval:` job is down longer than its catch-up window, for example `interval:1d` with the default `catch_up_hours` of 6 and 30 hours of downtime. `newest_missed_occurrence` returns `None`.
- Impact: `next_run_tz("interval:…", next_run - 1s)` equals `next_run - 1s + interval`, which never equals `next_run`. So a real missed interval run is no longer recorded at claim time. Only the startup scan (`scan_missed_occurrences`) still catches it. This weakens monitoring but does not change any message.
- Evidence: `let is_occurrence = next_run_tz(&expression, next_run - TimeDelta::seconds(1), timezone).is_ok_and(|occurrence| occurrence == next_run);`
- Fix: Treat interval and once expressions as always being occurrences: `let is_occurrence = parse_interval(&expression).ok().flatten().is_some() || next_run_tz(...).is_ok_and(...)`.

### [P2] A multi-target `deliver` string gives the native cron agent a bogus Slack session key
- Location: src/cron/executor.rs:636
- Trigger: A native payload has `"deliver": "slack:C1,discord:42"`.
- Impact: `slack_destination` is called on the whole string, so the channel becomes `C1,discord` and the thread becomes `42`. The agent session key (approvals, workspace) points at a Slack channel that does not exist. Scheduler delivery is unaffected because it splits on commas.
- Evidence: `if let Some(slack) = super::store::slack_destination(deliver, None) { ("slack", slack.chat_id, slack.thread_id) }`
- Fix: Use `deliver.split(',').map(str::trim).next().unwrap_or_default()` before parsing the prefix.

### [P2] `kill_process_tree` can signal an unrelated process group if a PID is reused
- Location: src/cron/executor.rs:1239
- Trigger: A descendant exits, and its PID is reused between the `ps` snapshot and the `getpgid` and kill calls.
- Impact: In theory, SIGKILL could hit an unrelated process or group. The window is milliseconds, so this is hypothetical.
- Evidence: `let pgid = unsafe { libc::getpgid(pid) }; if pgid > 1 && pgid != root { unsafe { libc::kill(-pgid, libc::SIGKILL); } }`
- Fix: Capture `pgid` in the same `ps` snapshot (`-o pid=,ppid=,pgid=`). Never signal the gateway's own pgid (`libc::getpgrp()`).

The rest of the diff checked out:
- `CronJob::profile` now accepts Slack session keys.
- `deliver` is now platform-aware.
- The Slack-without-dispatcher config error is new.
- The Slack-home destination logic in `store.rs` and the `select!`-based timeouts are new.

For the Slack WIP in `egress.rs`, cron sends use `SendMessage`, which that WIP does not touch.
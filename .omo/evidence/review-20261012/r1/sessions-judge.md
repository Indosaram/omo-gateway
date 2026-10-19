# sessions-judge review

### [P1] Slow or failing remote done-signal check stops the whole lane service
- Location: src/sessions/external.rs:394
- Trigger: An adopted remote lane claims done. After 5 minutes `Settler::settle_due` (src/lanes/settle.rs:93) calls `remote_checks`. That calls `remote_checks_with`, where `control.output(...).await.map_err(invalid)?` (lines 394, 415 and the worktree check near 424) returns `Err` on the fixed 20 s `ControlError::Timeout` in `SessionControl::output` (src/sessions/control.rs:200). It also returns `Err` on a spawn error. A remote `command [...]` Done signal that runs longer than 20 s (for example a test suite) hits this. `settle_due` propagates the error with `?`. In src/lanes/runner.rs:1035-1037 the `Ok(Err(error)) => return Err(error)` arm stops `LaneRunner::serve`. In src/main.rs:752 (`lane_service_exit`) that shuts the gateway down.
- Impact: One slow remote Done check takes down the entire gateway process, including unrelated lanes and Discord handling. The claim is already marked `settlement_started`, so after restart it becomes "settlement interrupted; claim again" and the owner must re-claim. The local `SystemSignalRunner` turns timeouts into failed signal results instead of errors; the remote path does not.
- Fix: In `remote_checks_with`, map `control.output` errors (timeout or command failure) to `reasons.push(...)` and continue, instead of returning `Err` with `?`. Use a longer timeout for command signals.

### [P1] A single transient read or send error marks an adopted external lane `failed`
- Location: src/sessions/external.rs:307
- Trigger: `LaneRunner::run_lane` (src/lanes/runner.rs:1106-1112) calls `ExternalDriver::drive`. In `drive`, `self.send_goal(key).await?` (line 286), `self.read_new(key).await?` (line 307) and `self.send_pending(key).await?` (line 308) all propagate any `ControlError`, which `map_err(invalid)` turns into `OmonError`. Examples: an ssh blip (exit 255 becomes `ControlError::Command`), a 20 s timeout, `omo thread read` failing while the session restarts, or a read response with no cursor. In src/lanes/runner.rs:1054-1056, `outcome` is `Err`, so the runner calls `self.fail(&key, "dead", ...)`. That sets the lane to `failed` when its status is `working`.
- Impact: One flaky poll permanently fails a lane for a session that is still running. The lane stops being driven, and the lead receives a "dead" event. Re-adopting is blocked until `release`, because `idx_lanes_external_target` is unique and `external_target` is still set. The managed-lane path allows 3 consecutive errors before failing; the external path allows none.
- Fix: In `drive`, log and retry on `read_new` and `send_pending` errors (a bounded consecutive-error counter like `errors >= 3`). Fail the lane only for `NotRunning` or after repeated failures.

### [P1] Pool connection held across ssh and `omo` calls starves all other DB users (max_connections = 1)
- Location: src/sessions/external.rs:151
- Trigger: `send_goal` (151-169) and `send_pending` (313-346) call `begin_with("BEGIN IMMEDIATE")` and keep the transaction open across `self.control.send(...).await`. Each `send` runs `session()` (a listing with a 20 s timeout) and then the `omo thread send` command (another 20 s timeout), so one call can hold the connection for up to about 40 s. `send_pending` repeats this for every unconsumed `lane_inbox` row. `Database::connect` (src/storage/db.rs:180-183) creates a pool with `max_connections(1)` and sets no `acquire_timeout`, so the sqlx default of 30 s applies. Every other task that needs the DB (the runner loop, settle, activity capture, Discord handlers) blocks until the transaction ends. Triggers are a slow or hung remote host, or two or more inbox messages at ~15 s each.
- Impact: The whole gateway's DB access stalls for the duration of the call. If the wait passes 30 s, callers get `PoolTimedOut`. For example, `list_lanes(...).await?` at src/lanes/runner.rs:976 propagates it and stops the lane service (the same exit path as the first finding).
- Fix: Do not hold the transaction across the send. Read the row and commit, send using the stable idempotency key (`send_key`, or the digest key in `send_pending`), then run a short transaction that sets `sent_at` or `consumed_at`.

### [P1] `release` leaves the lane `working`, so released lanes permanently consume lane slots
- Location: src/sessions/external.rs:118
- Trigger: `omo-gateway sessions release <target>` runs `ExternalDriver::release`. It only sets `external_sessions.released_at` and `lanes.external_target = NULL`. The lane's status stays `working` or `not_done` and no `lane_history` row is written. The runner skips it (src/lanes/runner.rs:991-993), so nothing ever closes it. `guard::query_active_lanes_sql` (src/lanes/guard.rs:105) and `retry_queued` (src/lanes/guard.rs:613) count `status IN ('working','blocked','not_done','done_claimed')`.
- Impact: Each adopt-then-release cycle leaks one slot against `OMON_LANES_MAX` (default 6). After about 6 releases, `can_launch` returns hold. Every new lane stays `queued` and `lane can-launch` exits 4, until someone manually runs `lane close`. The released lane also stays eligible for the 20-minute stale-activity posts.
- Fix: In the same transaction, move the lane to a terminal status (`closed`) and insert a `lane_history` row such as `'external session released'`. Alternatively, exclude released external lanes (`kind='external' AND external_target IS NULL`) from the active-lane counts.

### [P1] Done-diff verdict is computed on the first 4000 characters of the diff
- Location: src/judge/mod.rs:1439
- Trigger: `Settler::settle_due` calls `done_diff` (src/lanes/settle.rs:197), which builds a diff input of up to `MAX_DIFF` = 256 KiB and calls `judge.diff(Purpose::DoneDiff, ...)`. In `JevCli::diff`, `scrub_for_external(trimmed_diff)` ends with the unconditional `chars().take(4000)` cap (src/judge/mod.rs:47-48). Only that truncated text is piped to `typesafe-judge diff` (line 1457). `Fallback::fallback_diff` does the same at line 545 (`"state": scrub_for_external(trimmed)`). Any lane whose diff exceeds 4000 characters (a few files) triggers it.
- Impact: The judge sees only the first files of the change. It scores `fulfills` low because the visible diff omits parts of the deliverable. `done_diff` in settle.rs rejects at `fulfills < 0.65 || risk > 1.5`. Correctly finished lanes are sent back as `not_done` ("[LEAD] not done: done_diff ..."), and large work is more likely to loop. A prompt-sized cap is silently applied to a different payload.
- Fix: Scrub the diff without the 4000-character cap (a separate scrubber for diff bodies, or a larger explicit diff cap), and keep the 4000 cap only for state and instructions text.

### [P2] Daily-count resync compares an RFC3339 string against a space-separated timestamp
- Location: src/judge/mod.rs:762
- Trigger: `sync_persisted_counts` (called via `JevCli::init` from `lane` CLI, src/lanes/cli.rs:112) binds `kst_start_utc` as a `NaiveDateTime`. sqlx encodes it as `"%F %T%.f"`, for example `2026-10-11 15:00:00`. `record_call` (line 810) stores `created_at` with `DateTime<Utc>`, which sqlx encodes as RFC3339 (`2026-10-11T10:00:00+00:00`). The SQLite comparison `created_at >= ?` is a text comparison, and `'T'` (0x54) sorts after `' '` (0x20). So every row from the whole UTC calendar day of the boundary matches, including the ~15 hours before KST midnight.
- Impact: The `lane` CLI starts with inflated "today" counts. Yesterday's calls from 00:00-15:00 UTC are counted as today's. Spurious daily-cap hits can return `capped` conservative choices (for example `lane_size` becomes `large`) with the cap not actually reached.
- Fix: Bind the boundary as `DateTime<Utc>` (`Utc.from_utc_datetime(&kst_start_naive)`) so it is encoded in the same RFC3339 format as stored rows.

### [P2] External lane activity capture always uses reopen-generation 0
- Location: src/sessions/external.rs:225
- Trigger: `activity.captured_item(key, &text, 0, &id)` passes `Some(0)` as the generation. `LaneActivity::capture` (src/lanes/activity.rs:125-126) only updates the lane when `MAX(id) of 'reopened' history rows == generation`. A `failed` or `closed` external lane that was reopened (`runner::reopen`) has a nonzero generation. Reopen is reachable, and the first finding makes `failed` external lanes common.
- Impact: After a reopen, every captured message returns early as `pause: true`. `last_activity_at` is not refreshed, and `[REPORT]` milestone, blocker and question lines are not recorded or posted. The lane is flagged stale after 20 minutes even though the session is active. Claims still work (`change_claim_at` reads the generation itself).
- Fix: Read the current reopen generation (`reopen_generation`-style query) and pass it instead of the literal `0`.

### [P2] Breaker owner alert can never be delivered
- Location: src/judge/mod.rs:862
- Trigger: Five consecutive breaker-trigger failures (401, 429, "Unauthorized", "RateLimit" or a timeout) open the breaker. `record_failure` builds the alert and sets `last_alert_day`, so no second alert is built that day. `handle_breaker_alert` sends it with `let _ = alerter.send_owner_alert(&msg).await;` only when `self.alerter` is `Some`. Every production constructor passes `None` as the alerter (src/main.rs lines 710, 734, 1104, 1126; src/lanes/kibitzer.rs:158; src/lanes/runner.rs:746; src/lanes/settle.rs:28). The only `OwnerAlerter` implementor is the test `FakeOwnerAlerter` (line 575).
- Impact: The "jev response failed, switching to flash" owner notification is silently dropped. Judge calls degrade to fallback or conservative choices for 30 minutes with no owner-visible signal. A send failure would also be swallowed by `let _`.
- Fix: Provide a real `OwnerAlerter` and pass it at the production call sites. At minimum, log the swallowed result and warn when the breaker opens with no alerter configured.

### [P2] `sessions adopt --lead` is not validated as a session storage key
- Location: src/sessions/external.rs:66
- Trigger: `ExternalDriver::adopt` only checks `lead.trim().is_empty()`. `lanes::cli` validates the lead with `SessionKey::from_storage_key` (src/lanes/cli.rs:297 and 344), but this path does not. A mistyped `--lead` (anything not in canonical storage-key form) is stored in `lanes.lead_session_key`. `EventDispatcher::deliver_group` (src/lanes/dispatch.rs:95) then calls `SessionKey::from_storage_key(lead)?`, which fails.
- Impact: All lane events for that lane (`done_claimed`, `not_done`, `dead`) are never delivered. The group failure is only logged as "lane events remain queued for retry". Retries repeat every 5 seconds, and the events accumulate in `lane_events`.
- Fix: Call `SessionKey::from_storage_key(lead)` in `adopt` and reject the adoption on error.

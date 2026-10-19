# r2-dashboard review

### [P1] Combined `run` + dashboard does not compile: `home` is shadowed and moved before it is reused
- Location: /Users/indo/code/project/omon-gateway/src/main.rs:1048 (shadowing), :1056 (move), :1210 (use)
- Trigger: `cargo build`. Line 879 binds `home: PathBuf`. Line 1048 rebinds `home` as `Option<String>` (the owner-DM `session_key` query), and `if let Some(home) = home` at line 1056 moves it. Line 1210, in the `dashboard_settings` block of the same function, passes `&home` to `hermes_skill_dirs(&Path, &Path)`.
- Impact: type mismatch plus use after move, so the `omo-gateway` bin target fails to build. The round-1 fix-dashboard-share evidence says this wiring was never compiled.
- Fix: rename the DM binding at 1048 and 1056 (for example `home_session`), or compute `let skill_roots = hermes_skill_dirs(&hermes_root, &home);` right after line 884 and reuse it at 1210.

### [P1] `POST /api/bots` silently overwrites an existing bot profile (the duplicate-id 409 was fixed only for cron)
- Location: /Users/indo/code/project/omon-gateway/src/dashboard.rs:1951 (`create_bot`), :1979 (`ON CONFLICT(bot_id) DO UPDATE`)
- Trigger: the operator uses "Add Agent Profile" (web/src/pages/BotsPage.tsx `handleCreate`) with a `bot_id` that already exists. The form sends `model`, `system_prompt` and `enabled_toolsets` as null when blank, and never sends `custom_settings`.
- Impact: the existing row's `model`, `system_prompt`, `enabled_toolsets` and `custom_settings_json` (including `avatar_url`) are replaced by NULL and `{}`. The response is still `{"status":"created"}`. A live bot's persona is wiped without warning.
- Fix: for create, use a plain `INSERT` and map a unique-constraint violation to 409 `bot profile {id} already exists`. Keep updates on `PUT /api/bots/{id}`.

### [P1] Startup recovery never resumes DM turns when `DISCORD_ALLOWED_CHANNELS` is set
- Location: /Users/indo/code/project/omon-gateway/src/main.rs:366-374 (`StartupAuthorization::is_authorized_actor`)
- Trigger: `DISCORD_ALLOWED_CHANNELS` is non-empty (it must now contain valid ids, per `parse_allowlist`). A DM turn, such as the owner's lead DM, is in flight at SIGTERM or drain, so `resume_pending` is set. The DM key has `guild_id = None` and the DM channel id as `channel_id`. The check at line 371 rejects any channel not in the allowlist.
- Impact: live ingress exempts DMs from the allow-list (`is_channel_authorized(..., is_dm)` in src/discord/adapter.rs:1835-1843 and src/discord/commands.rs:260), but recovery does not. The pending DM turn is deferred with "marker retained" on every boot, so it is never resumed or cleared. It also does not count toward the restart-loop budget.
- Fix: skip the `allowed_channels` check when `session_key.guild_id.is_none()`. Keep the ignored-channels check.

### [P1] Standalone `dashboard` / `serve` starts its own cron scheduler on the shared DB, so Discord-delivery jobs fail when a gateway is also running
- Location: /Users/indo/code/project/omon-gateway/src/dashboard_runtime.rs:30 (`run_standalone(settings, shutdown, true)`) and :168-170 (`scheduler.start()`); /Users/indo/code/project/omon-gateway/src/entry.rs:125 (CLI path); /Users/indo/code/project/omon-gateway/src/entry.rs:241 (`RuntimeOwnershipLock`, never acquired outside tests)
- Trigger: the gateway runs as its own process (launchd or `run`), and an operator also runs `omo-gateway dashboard`, as web/README.md documents. Both schedulers poll every second against the same `cron_jobs` and `cron_runs` tables, and the lease only keeps them from running a job twice. When the dashboard process wins the claim for a job with `deliver: discord:...`, it executes the agent turn and delivers through `WebDashboardDispatcher`. That dispatcher now returns Err, so the delivery obligation is `failed`. The run is recorded as failed, an incident and failure notification are attempted (and also fail), and the output is lost. A manual "Run now" in a detached dashboard (`trigger_cron_job`, dashboard.rs:1645) behaves the same, while the API returns `triggered:true`.
- Impact: the round-1 fail-closed change turned silent delivery loss into recurring visible failures for a fraction of due Discord jobs. The cause is the second scheduler, which the dashboard has no way to deliver for.
- Fix: pass `start_scheduler=false` from `run_standalone_cli` by default (or behind an explicit flag), or acquire `RuntimeOwnershipLock` for the scheduler. Reject or warn on `trigger` for jobs whose destination is not web when `detached_from_gateway` is set.

### [P2] Recovery authorization diverges from live admission: roles, thread ids, and guild-channel authors
- Location: /Users/indo/code/project/omon-gateway/src/main.rs:366-381 and :396-427 (`pending_recovery_actor`); /Users/indo/code/project/omon-gateway/src/storage/message_search.rs:208
- Trigger:
  - **Roles:** users authorized only through `DISCORD_ALLOWED_ROLES` are never covered. `StartupAuthorization` has no roles, so their pending turns are deferred forever.
  - **Thread ids:** only the parent `channel_id` is checked against the ignored and allowed lists, so an ignored thread id inside an allowed parent still recovers.
  - **Guild channels:** ingress indexing forces `source = "ingress"` (message_search.rs:208), while `pending_recovery_actor` only trusts `source = 'discord_rest'`. A normal guild-channel turn interrupted by restart therefore yields no actor and is deferred indefinitely.
- Impact: those in-flight turns are silently not resumed. Each boot logs a warning and keeps the marker.
- Fix: carry role ids or resolve the actor's current roles via REST. Check both the thread id and the parent for the ignored and allowed lists. Trust the ingress `author_id` for exact `platform_message_id` matches, or document that guild turns are intentionally not auto-resumed.

### [P2] `DELETE /api/sessions/{id}` is not covered by the detached guard
- Location: /Users/indo/code/project/omon-gateway/src/dashboard.rs:1199-1215
- Trigger: a standalone or detached dashboard deletes a Discord session while a gateway process is live. `mux.stop` on the dashboard's empty multiplexer only writes `mark_session_suspended(true)`, then the row is deleted and its messages and obligations cascade away.
- Impact: the gateway's actor keeps running against a deleted transcript. This conflicts with the stated purpose of `reject_detached_messenger_session`.
- Fix: call `reject_detached_messenger_session` in `delete_session` for non-web keys, or make the delete explicit about the gateway not being stopped.

### [P2] Check-then-write races in `create_cron_job` and `update_bot`
- Location: /Users/indo/code/project/omon-gateway/src/dashboard.rs:1566-1572 (`scheduler.get`, then `register_with_id` which is an upsert), :2024 onward (`update_bot`, read-modify-write)
- Trigger: two concurrent POSTs with the same cron id both pass the existence check and the later one overwrites. Two concurrent bot PUTs on different fields lose one update.
- Impact: this is the same overwrite the 409 fix was meant to prevent, though the window is small.
- Fix: use an insert-only SQL path that returns 409 on a unique-constraint error. Do the bot update in a single `UPDATE ... COALESCE` statement or a transaction.

### [P2] Invalid allowlist entries still fail open or silently shrink access outside `DISCORD_ALLOWED_CHANNELS`
- Location: /Users/indo/code/project/omon-gateway/src/main.rs:105-111 (`DISCORD_ALLOWED_USERS` inline parser, no warning), :128 (`DISCORD_IGNORED_CHANNELS`), :2384-2400 (`parse_u64_list`)
- Trigger: `DISCORD_IGNORED_CHANNELS=general` (names instead of ids) parses to an empty list with only a warning, so nothing is ignored. An all-invalid `DISCORD_ALLOWED_USERS` becomes empty, and `should_prompt_unauthorized_dm` (adapter.rs:1671) then offers pairing codes to any DM.
- Impact: a deny-list typo leaves the bot answering in channels meant to be ignored. An allow-list typo opens the pairing prompt to everyone.
- Fix: run IGNORED_CHANNELS and ALLOWED_USERS through `parse_allowlist`. Reuse `parse_u64_list` for ALLOWED_USERS so the warning is logged.

### [P2] Standalone dashboard shutdown does not drain the multiplexer or mark in-flight sessions
- Location: /Users/indo/code/project/omon-gateway/src/dashboard_runtime.rs:200-208
- Trigger: Ctrl+C or `shutdown` during an in-flight web chat turn. The code shuts down the scheduler and scale-to-zero, then closes the pool, but never calls `mux.shutdown()` or `mark_in_flight_resume_pending()`.
- Impact: the in-flight turn is dropped without a resume marker, unlike the gateway's shutdown path.
- Fix: keep the `SessionMultiplexer`, call `mark_in_flight_resume_pending()` and `shutdown()` before `pool.close()`.

### [P2] Detached WebSocket gives misleading signals for messenger sessions
- Location: /Users/indo/code/project/omon-gateway/src/dashboard.rs:1313 (`ready.chat_available`), :1373-1385 (the `stop` branch)
- Trigger: a client opens `/api/sessions/{discord key}/ws` on a detached dashboard.
- Impact: the `ready` frame says `chat_available: true`, but every chat or stop is then rejected. The `stop` branch sends no frame at all when `state.multiplexer` is `None`.
- Fix: compute `chat_available` per session (`mux.is_some() && !(detached && platform != "web")`), and send an error frame in the `None` case.

### [P2] `dashboard.await?` can mask the gateway result, and the error text is wrong in the Discord-disabled branch
- Location: /Users/indo/code/project/omon-gateway/src/entry.rs:226-236
- Trigger: the gateway finishes first, then the dashboard returns an Err during shutdown. Separately, if the dashboard exits first, the message says "while the Discord gateway was running" even though Discord is disabled in this branch.
- Impact: a gateway error can be lost behind the dashboard error, and the message is misleading.
- Fix: log the dashboard error and return the gateway's `result`. Reword the message to "headless gateway".

### [P2] Inconsistent dashboard config view between attached and standalone modes
- Location: /Users/indo/code/project/omon-gateway/src/dashboard.rs:672; /Users/indo/code/project/omon-gateway/src/dashboard_runtime.rs:292-302 and :311-319
- Trigger: `GET /api/config` in attached mode (`config_view_from_gateway`) versus standalone (`dashboard_config_view`).
- Impact: `approval.policy` is `format!("{:?}", policy)` (for example `Smart`) in attached mode but a lowercase env string in standalone. Attached mode also omits `discord.configured` and `message_context_policy`.
- Fix: share one config-view builder.

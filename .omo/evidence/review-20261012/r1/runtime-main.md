# runtime-main review

### [P1] Attached dashboard builds a second, disconnected runtime instead of controlling the live gateway
- Location: src/entry.rs:201
- Trigger: Set `DASHBOARD_ENABLED=1` or `DASHBOARD_PORT` and run `omo-gateway run`. `run_gateway_with_optional_dashboard` starts `legacy::dashboard_runtime::run_standalone(settings, dashboard_shutdown, false)` next to `legacy::run_gateway_public()`. `run_standalone` (src/dashboard_runtime.rs, around lines 128-140) builds its own `SessionMultiplexer`, `SmartApprovalGuard`, `DiscordApprovalRequester` and `CronScheduler`. All of them use `WebDashboardDispatcher`. Nothing is shared with the gateway's multiplexer, approval guard or Discord egress.
- Impact:
  - `POST /api/sessions/{id}/stop` calls `mux.stop` on the dashboard's empty multiplexer. It returns `stopped:false`, does not interrupt the live Discord turn, and writes `mark_session_suspended(true)` to the shared DB for that session.
  - `POST /api/sessions/{id}/chat` starts a second actor and agent turn on the same session key, concurrent with the gateway's actor. Its output goes only to the web dispatcher.
  - Approvals raised by the Discord gateway cannot be resolved from the dashboard, because it uses a different approval guard.
  - A cron job triggered from the dashboard delivers through `WebDashboardDispatcher`, whose `dispatch` swallows the send result. The delivery obligation is marked delivered although nothing reached Discord.
- Fix: Build the `DashboardState` from the gateway's own multiplexer, approval guard, scheduler and egress when running attached, for example by passing shared handles into `run_standalone`. Otherwise do not offer the attached mode.

### [P1] Restart recovery clears `resume_pending` before routing, so a failed route loses the turn permanently
- Location: src/main.rs:448
- Trigger: `run_gateway` calls `recover_resume_pending_sessions_with_auth` on boot. It runs `clear_session_resume_pending` (line 448-449), then `multiplexer.route(event)` (line 471). `route` returns `Err` while the multiplexer `is_draining()`, or if actor admission fails. The multiplexer is built with `with_drain_receiver(drain_rx)` after `drain_watcher.scan_at` has already read a valid `.drain_request.json`. On macOS the epoch is empty, so such a marker only goes stale after one hour. So a drain followed by a supervisor restart inside that hour makes every recovery route fail.
- Impact: The failure is only logged at line 471-476. The durable marker is already cleared (and the `legacy_guild_pending_auth` row stays unused), so that session's unfinished user turn is never resumed and is silently dropped. Because `resumed_count` only counts successes, the boot log under-reports without naming the loss.
- Fix: On `Err` from `multiplexer.route`, call `mark_session_resume_pending(pool, &storage_key)` again so the next boot retries. Alternatively, clear the marker only after a successful route.

### [P2] Startup recovery authorizes only `allowed_users` and `allow_all_users`, so paired users' in-flight turns are never resumed
- Location: src/main.rs:336
- Trigger: A deployment that admits users through the pairing store, the default when `DISCORD_ALLOWED_USERS` is empty and `DISCORD_ALLOW_ALL_USERS` is false. A paired user has an in-flight DM turn at restart. `StartupAuthorization::is_authorized_actor` checks only `allowed_users.contains`. `PairingStore::init_cache` runs later in `run_gateway` (`poise_data.pairing_store.init_cache()`), after recovery, and `allowed_roles` is not consulted either. The live path in src/discord/commands.rs `is_user_authorized` and src/discord/adapter.rs does honour pairing.
- Impact: Recovery logs "deferring pending recovery" and keeps the marker on every boot. The paired user's interrupted turn is never resumed.
- Fix: Run `pairing_store.init_cache()` before recovery and add a `paired_users` set to `StartupAuthorization`, with `is_authorized_actor` also accepting those ids.

### [P2] A malformed channel allowlist silently becomes "no restriction"
- Location: src/main.rs:2277
- Trigger: `DISCORD_ALLOWED_CHANNELS` is set to values `parse::<u64>()` rejects, such as channel names, `<#123>` mentions, or one stray character. `parse_u64_list` is the helper used for it, and `Config::from_env` calls it at around line 121. `filter_map(|p| ... .ok())` drops every bad entry, leaving an empty list. `StartupAuthorization` and the Discord filters treat an empty `allowed_channels` as "all channels allowed".
- Impact: An operator who meant to restrict the bot to specific channels gets a bot that answers in all channels, with only a missing-value-style silence in the logs. A partially valid list drops entries without any warning.
- Fix: Return a `Config` error, or at least `warn!`, when a non-empty trimmed entry fails to parse. Fail startup if the variable is set but yields an empty list.

### [P2] A panicked Discord client task exits the gateway with status 0
- Location: src/main.rs:1158
- Trigger: `if let Ok(Err(err)) = res` only handles a task that returned an error. If a `client.start()` task panics, `join_next` yields `Err(JoinError)`. It matches nothing, `discord_task_failed` stays false, and the function finishes through `warn!("omo-gateway stopped"); Ok(())`.
- Impact: The comment at lines 1229-1231 says a crashed Discord task must exit non-zero so systemd or launchd restarts it. A panic gives exit 0, so a restart policy such as launchd `KeepAlive/SuccessfulExit=false` or systemd `Restart=on-failure` will not restart, leaving the gateway down. I did not find the deployment's restart policy in the repo.
- Fix: Treat `Err(join_error)` the same as `Ok(Err(_))` and set `discord_task_failed = true`.

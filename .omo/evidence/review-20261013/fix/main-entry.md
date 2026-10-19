# main-entry fix report (lanes: main-runtime-a, main-runtime-b)

Nothing was compiled or run (user directive). All edits are hand-checked for types, imports and rustfmt shape. Scope: `src/main.rs`, `src/entry.rs`, `src/drain_control.rs`, `src/readiness.rs`. Tests are named `fix_r5_main_*` (main.rs), or sit in the existing `tests` module of the file edited.

## main-runtime-b

- **[P1] RuntimeOwnershipLock never acquired in production** - FIXED.
  - `entry.rs:run_gateway_with_optional_dashboard` now takes `acquire_runtime_lock()?` as its first statement. The guard lives to function return, before any of the three startup branches.
  - The standalone `dashboard` / `serve` command takes the same lock only when `OMON_DASHBOARD_RUN_SCHEDULER` is truthy, because otherwise it never competes with a gateway.
  - Lock name is `runtime_lock_name(DATABASE_URL)`: the first 8 bytes of SHA-256 over the resolved database path, hex-encoded, so each DB gets its own lock. The default URL matches the gateway's `sqlite://omon_gateway.db`.
  - The lock dir is `OMON_RUNTIME_LOCK_DIR` or the per-user `~/.omon/run`, created with mode 0700. It is no longer the shared temp dir.
  - Tests: `duplicate_gateway_start_is_rejected` (rewritten to use `try_acquire_in` with a tempdir), `runtime_lock_name_is_scoped_to_the_database`.
- **[P2] Lock file in shared /tmp, default perms, no symlink protection** - FIXED.
  - `entry.rs:RuntimeOwnershipLock::try_acquire_in` opens with mode 0600 and `O_NOFOLLOW`, in a per-user dir.
  - Test: `runtime_lock_is_private_and_refuses_symlinks`.
- **[P2] Gateway error masked by dashboard error (Discord-disabled path)** - FIXED.
  - `entry.rs:run_gateway_with_optional_dashboard` now returns `result.and(dashboard_result)`, so the gateway error wins. A dashboard error is only logged.
  - No dedicated test: the path needs a full gateway plus dashboard.
- **[P2] `profile` subcommands create a stray DB at the default path** - FIXED.
  - `entry.rs:connect_profile_database` now requires a non-blank `DATABASE_URL` and fails with exit 1 otherwise.
  - No dedicated test.
- **[P2] Credential readiness provider-agnostic, substring model matching, empty var counted as present** - FIXED.
  - `readiness.rs:credential_provider` matches explicit prefixes: `claude`, `anthropic/`, `gpt`, `openai/`, `o1`/`o3`/`o4` as a whole token or followed by `-` or `.`, and `deepseek`.
  - `probe_credentials_with` requires the model's own key or `LLM_API_KEY`. Blank values count as absent. Unknown models accept any non-blank key.
  - Tests: `credential_provider_matches_prefixes_not_substrings`, `credentials_require_the_models_own_provider_key`.
- **[P2] Backend readiness probe goes through env proxies** - FIXED.
  - `readiness.rs:probe_backend` now builds the client with `.no_proxy()`.
  - No dedicated test (needs a network or proxy fixture).
- **[P2] Drain marker writer leaks temp file; clear has TOCTOU** - FIXED.
  - `drain_control.rs:write_drain_request` removes the temp file when `rename` fails.
  - `drain_control.rs:clear_drain_request` calls `remove_file` directly and maps `NotFound` to `Ok(false)`.
  - Tests: `failed_marker_commit_leaves_no_temp_file`, `clearing_an_absent_marker_is_not_an_error`.
- **[P2] main.rs tests mutate process-global env / weak drain assertions** - FIXED.
  - New `ENV_LOCK` mutex serializes `default_model_precedence_matches_dashboard` and `cron_and_interactive_share_one_daemon`.
  - `drain_is_reversible_and_preserves_accepted_work` now asserts that turns 1 and 2 are processed while draining, and that turn 4 is processed after the drain is reversed. It uses a channel with bounded `timeout` waits, no sleeps.
  - Caveat: other tests that read these env vars without taking `ENV_LOCK` are not covered by the lock.

## main-runtime-a

- **[P1] Clearing a boot-time drain marker shuts the gateway down** - FIXED.
  - `main.rs:wait_for_drain` loops on `changed()` until the value is true. A dropped sender stays pending. The final `select!` drain arm uses it.
  - Tests: `fix_r5_main_drain_wait_ignores_clearing_a_boot_time_drain`, `fix_r5_main_drain_wait_stays_pending_when_sender_is_gone`.
- **[P2] Recovery re-dispatch uses a new event id, no delivery id, re-rendered attachments** - FIXED.
  - `main.rs:recovery_event` reuses `unfinished.message_id` as the event id and `created_at` as `received_at`.
  - `[system] ` content is mapped back to `EventOrigin::System`.
  - Attachments are not re-attached, because the stored content is already rendered.
  - `main.rs:recovery_delivery_id` looks up the delivery-ledger `message_id`, using the same query as `actor.rs`, before the resume marker is cleared.
  - Tests: `fix_r5_main_recovery_event_keeps_id_origin_and_does_not_reattach`, `fix_r5_main_recovery_reuses_message_id_and_ledger_delivery` (asserts the reused id, the delivery id, and exactly one user row).
- **[P2] Startup authorization ignores DISCORD_ALLOWED_ROLES / only trusts discord_rest authors** - SKIPPED. Fail-closed by design. Fixing it needs a new author-trust column persisted at ingress, which is outside this scope (adapter, storage and migrations).
- **[P2] Mail and iMessage watchers die on a single failure; mail emits only on child exit** - FIXED.
  - `main.rs:supervise_watcher` restarts with doubling backoff, from 5s up to 300s. It resets after a run of at least 60s, and wakes on shutdown.
  - `run_mail_watch_once` now sends a `Mail` event per non-blank output line (`try_send`, coalesced when the channel is full).
  - A clean exit with events is `Completed` and restarts after the initial delay. Any other exit is `Failed`.
  - Tests: `fix_r5_main_watcher_restarts_with_bounded_backoff` (paused clock, asserts 5s+10s), `fix_r5_main_watcher_backoff_wakes_on_shutdown`.
- **[P2] Debounce batches dropped on drain / lane-failure / client-failure shutdowns** - PARTIALLY FIXED, rest BLOCKED-SCOPE.
  - FIXED: the lane-failure and `join_set` arms now call `global_debouncer().flush_all` and `mark_in_flight_resume_pending`, as the signal arms do.
  - BLOCKED-SCOPE: in the drain arm the multiplexer is already draining, so `route` refuses the flushed batch.
    - Needed in `src/multiplexer/router.rs`: an admission bypass for the shutdown flush, for example `SessionMultiplexer::route_during_shutdown`.
    - Or in `src/discord/adapter.rs`: `flush_all` persists the events and marks resume-pending instead of routing when `multiplexer.is_draining()`.
  - No test for the new arm calls (needs the full `run_gateway`).
- **[P2] `parse_bool_from` treats unknown tokens as false** - FIXED.
  - `main.rs:parse_bool_from` falls back to `default` for unrecognised tokens and warns.
  - Test: `fix_r5_main_unknown_boolean_tokens_keep_the_default`. The existing `parses_bool_from_env_variants` still holds.
- **[P2] DISCORD_ALLOWED_USERS parsed lossily, malformed value falls back to open pairing** - FIXED.
  - `Config::from_env` uses `parse_allowlist` for `DISCORD_ALLOWED_USERS` and `DISCORD_ALLOWED_ROLES`. A set-but-unparsable value now refuses to start.
  - Test: `fix_r5_main_user_allowlist_must_parse`.
- **[P2] Startup opens System Settings on every boot while FDA denied** - FIXED.
  - `main.rs:ensure_full_disk_access(workspace_root)` claims a `.fda_prompted` marker with `create_new`, so the pane opens once per workspace.
  - The `open` command now runs only on macOS, via a non-blocking `tokio::process::Command::spawn`.
  - Test: `fix_r5_main_full_disk_access_prompt_is_claimed_once`.
- **[P2] Hardcoded owner id and mail account** - FIXED.
  - `main.rs:owner_id_from(OMON_OWNER_ID)` falls back to `inbox::proposals::OWNER_ID`.
  - `main.rs:mail_account_from(OMON_MAIL_ACCOUNT)` falls back to the previous account.
  - Zero-config defaults are unchanged.
  - Test: `fix_r5_main_owner_and_mail_account_are_configurable`.
- **[P2] `build_cron_llm_config` pairs a job-supplied base_url/provider with the base api_key** - FIXED.
  - `main.rs:build_cron_llm_config` clears `api_key` when the provider changes or a `base_url` override is given.
  - Test: `fix_r5_main_cron_llm_override_never_carries_the_base_key_elsewhere`. The existing `test_build_cron_llm_config_overrides` is unaffected, since its base has no key.

## Related items outside this scope

- **[P1, CONFIRMED.md main-runtime-b] spawn_receive_watchdog never called in production** - BLOCKED-SCOPE.
  - `spawn_receive_watchdog` is a private fn in `src/discord/adapter.rs:2393`, called only from `DiscordAdapter::start`. `main.rs` starts clients via `join_set.spawn(async move { client.start().await })`.
  - Needed in `adapter.rs` (discord-adapter node): make it `pub fn spawn_receive_watchdog(...)` and re-export it from `src/discord/mod.rs` (and `lib.rs` if needed).
  - Then main-entry adds, in `main.rs` next to `shard_managers.push(client.shard_manager.clone());`, a call to `omon_gateway::discord::spawn_receive_watchdog(client.shard_manager.clone());`.
  - Alternatively, the adapter node adds `DiscordAdapter::spawn_watchdog(&client)` and main-entry calls that.
- **[P1] dashboard readiness reads `config.appserver_url` / `OMON_APPSERVER_URL`** - belongs to the dashboard node. `readiness::probe_backend` itself is unchanged and correct.

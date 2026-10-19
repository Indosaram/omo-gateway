# r2-inbox-migrate-voice review

### [P1] APPROVAL_MODE from a profile .env overrides the root config.yaml and is imported globally
- Location: /Users/indo/code/project/omon-gateway/src/migrate/config_import.rs:462-487
- Trigger: Root `config.yaml` has `approvals.mode: always` and the root `.env` has no `APPROVAL_MODE`. One Hermes profile `.env` has `APPROVAL_MODE=never` (a "yolo" profile). Run `omon migrate`.
- Impact: The SCALAR loop (line 480) inserts `root_value.or(profile_value)` for `APPROVAL_MODE`, so the profile value wins. The `config.yaml` value is only used when the key is still unset (line 482). The target `.env` gets `APPROVAL_MODE=never`, and `main.rs` parses that as `ApprovalPolicy::Never`. Approval gating is then disabled for every session. The round-1 fix closed this hole only for `DISCORD_*` access-control keys. `APPROVAL_MODE` and `APPROVALS_DESTRUCTIVE_SLASH_CONFIRM` still fall back to profile values.
- Fix: Treat `APPROVAL_MODE` and `APPROVALS_DESTRUCTIVE_SLASH_CONFIRM` like the access-control keys: never fall back to a profile value. Take the root `.env` value, then the root `config.yaml` value.

### [P1] Local transcription subprocesses have no timeout and no kill_on_drop
- Location: /Users/indo/code/project/omon-gateway/src/voice_transcription.rs:75 and :95 (the ffmpeg and whisper-cli `.output()` calls). The caller is /Users/indo/code/project/omon-gateway/src/discord/adapter.rs:1564.
- Trigger: A long or truncated voice attachment makes `whisper-cli` (large-v3-turbo) or `ffmpeg` run for minutes or stall.
- Impact: `route_claimed_event_with_constituents` awaits `transcribe_audio` before `multiplexer.route`. The delivery is already recorded in the ledger as incoming, but the turn is never routed while the child runs. The same await runs inside the debounce flush task and `flush_all`, so a stuck child also stalls shutdown or drain. Round 1 added timeouts only to the HTTP STT client. These subprocesses were not covered.
- Fix: Add `.kill_on_drop(true)` to both commands and wrap each in `tokio::time::timeout`, for example 60s for ffmpeg and a few minutes for whisper. Remove the temp `.wav` and `.txt` files on every error path.

### [P2] Dropping a GatewayMessage on a full channel can leave the Discord source uncollected
- Location: /Users/indo/code/project/omon-gateway/src/inbox/watchers.rs:27-37, and `filter_sources_for_event` (starts near line 65, GatewayMessage arm filters to Discord only)
- Trigger: A burst of 64 or more iMessage, mail or kakao events fills the channel (DEFAULT_EVENT_CHANNEL_CAPACITY = 64) while a slow collection (up to 60s per source) is running. A Discord message is indexed in that window.
- Impact: The comment says a full channel already holds pending work and the event is idempotent. That is wrong, because the queued events only collect their own source. The Discord inbox item stays uncollected until the next Discord message or a Startup event.
- Fix: Keep a coalescing "discord dirty" `AtomicBool` or `Notify` that the drain loop checks after each event. Or reserve capacity for GatewayMessage.

### [P2] Dropped progress results can strand pending triage items
- Location: /Users/indo/code/project/omon-gateway/src/inbox/watchers.rs:151, and `Triage::collected` in /Users/indo/code/project/omon-gateway/src/inbox/triage.rs (the `inserted > 0` guard that sets `deadline`)
- Trigger: Triage is inside a slow `flush` (LLM calls run inline in `serve`) while 16 or more collection results queue up. The 17th result is dropped by `try_send`.
- Impact: Only a result with `inserted > 0` arms the 120s triage deadline. A dropped result means no deadline is armed. Those inbox items stay without a decision until a later result arrives or the gateway restarts, because the startup scan is the only other recovery.
- Fix: Keep the `try_send` so the drain never stalls. On `Full`, also set a shared "triage pending" flag that `serve` checks, or re-arm the deadline from a DB check after each flush.

### [P2] Migrate preflight fails for non-selected profiles and rejected jobs, but dry-run reports no error
- Location: /Users/indo/code/project/omon-gateway/src/migrate/mod.rs:187-203 and `discover_stores` in /Users/indo/code/project/omon-gateway/src/migrate/cron_cutover.rs:585
- Trigger: `OMON_HERMES_PROFILES=work` while the `default` store holds jobs. A selected store holding a job that fails `validate` also triggers it.
- Impact: The sync imports only selected, valid jobs, but `cutover_cron_stores` walks every store. Those jobs have no `cron_jobs` row, so they land in `unverified_jobs` and the preflight returns an error. Real migrate never completes cutover, while `--dry-run` shows `would_empty=false` for those stores and no error. Hermes is correctly left running and the failure is fail-closed.
- Fix: Skip non-selected profiles in the cutover walk (pass the selected set). Make `project_migration` report "cutover blocked" when a selected store has rejected jobs.

### [P2] Dashboard /api/ready never gates on the backend
- Location: /Users/indo/code/project/omon-gateway/src/dashboard.rs:872-880
- Trigger: The omo app-server is down in production.
- Impact: `appserver_url` is read from `state.config["appserver_url"]` or `OMON_APPSERVER_URL`. The `config_view` builders do not set the former, and the real variable is `OMON_OMO_APPSERVER_URL`. So `appserver_url` is `None`, and `(backend_ok || appserver_url.is_none())` is true. `/api/ready` returns 200 while the backend is unreachable. The round-1 readiness fix only touched `collect_runtime_readiness`.
- Fix: Resolve the URL the same way `collect_runtime_readiness` does (`OMON_OMO_APPSERVER_URL`, default `ws://127.0.0.1:19742`), and gate readiness on the probe.

### [P2] Voice attachments are transcribed twice
- Location: /Users/indo/code/project/omon-gateway/src/discord/adapter.rs:1559-1566, together with `hydrate` in /Users/indo/code/project/omon-gateway/src/discord/attachments.rs:231-300
- Trigger: A Discord voice or audio attachment while an OpenAI key is configured.
- Impact: `hydrate` already sets `attachment.text_content` through STT. `route_claimed_event_with_constituents` then runs local whisper again and prepends a second transcript to `event.content`. The first transcript duplicates content, and the second adds the unbounded-blocking cost described in the P1 above. The skip on existing `text_content` was left undone in round 1 (it was blocked on scope).
- Fix: Skip `transcribe_audio` when `attachment.text_content` is already `Some` and non-empty.

### [P2] A merge deletes the target's existing DISCORD_BOT_TOKEN(S) when the Hermes source has none
- Location: /Users/indo/code/project/omon-gateway/src/migrate/config_import.rs:516-526 (`merged_values`) and `render_merged_env` (the `key == "DISCORD_BOT_TOKEN" || key == "DISCORD_BOT_TOKENS"` branch that skips the line)
- Trigger: Root `discord.enabled: false`, or no token anywhere in Hermes, run against a gateway `.env` that already has a token.
- Impact: The existing token lines are dropped from the new `.env`. The gateway then fails with a missing-token error at startup. The `.bak-*` file keeps the old values, so it is recoverable.
- Fix: Remove stale tokens only when the Hermes source supplies at least one token. Otherwise preserve the existing lines.

### [P2] bring_gateway_down unloads LaunchAgents before verifying pid identity
- Location: /Users/indo/code/project/omon-gateway/src/migrate/gateway_down.rs (`bring_gateway_down`, the plist loop followed by the pid loop; roughly lines 31-111)
- Trigger: A `gateway.lock` without `start_time`, or one where `process_start_time` cannot be read, plus a live `ai.hermes.gateway*.plist`.
- Impact: Bootout and the plist rename happen first. Then `verified_alive` returns `Err`. The migrate aborts with the service unloaded and renamed, the old process still running, and cutover not done.
- Fix: Run `verified_alive` for every lock (identity check only) before any bootout or rename.

### [P2] The readiness probe URL keeps any path from the appserver URL
- Location: /Users/indo/code/project/omon-gateway/src/readiness.rs:238-247 (`backend_http_url`) and :262-263
- Trigger: `OMON_OMO_APPSERVER_URL=ws://host:19742/some/path`. `validate_url` allows any path.
- Impact: The probe hits `http://host:19742/some/path/readyz`. The daemon's own `probe_readyz` strips the path, so the two probes disagree and the readiness check reports degraded.
- Fix: Reduce the URL to scheme and authority before appending `/readyz` and `/health`.

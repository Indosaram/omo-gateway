# main-runtime-a review
### [P1] Clearing or expiring a drain marker that was already present at boot silently shuts the gateway down
- Location: /Users/indo/code/project/omon-gateway/src/main.rs:1254 (drain arm of the final `tokio::select!`, which starts at 1233). The receiver is created at ~1010, after `drain_watcher.scan_at` has already set the flag.
- Trigger: A valid `.drain_request.json` exists in the workspace when the gateway boots. This is the normal "drain, then supervisor restarts" flow. On macOS the epoch is empty, so the marker stays valid for 1h. `scan_at` sets the watch value to `true` before `drain_watcher.receiver()` is cloned, so `drain_rx.changed()` does not fire for the initial value. The multiplexer refuses all turns because `is_draining()` is true. The operator then runs `clear_drain_request` (or the marker ages out). The value goes `true` to `false`, `changed()` resolves, `*drain_rx.borrow()` is false, and the arm body does nothing. `select!` has still completed, so execution falls through to `lane_shutdown.cancel()`, `scheduler.shutdown()`, `stop_lead_runtime` and `Ok(())`.
- Impact: Cancelling a drain makes the gateway exit with status 0 and stop serving. The graceful path (`flush_all`, `mark_in_flight_resume_pending`, `shard_managers.shutdown_all()`) is skipped. A supervisor that treats exit 0 as a clean stop does not restart it, so the bot goes offline instead of resuming.
- Fix: Loop over the `drain_rx.changed()` arm until `*borrow()` is true. For example, `loop { if drain_rx.changed().await.is_err() || *drain_rx.borrow() { break; } }` inside the arm's future. Alternatively use `drain_rx.wait_for(|v| *v)`.

### [P2] Restart recovery re-dispatches the stored transcript with a new event id, no delivery id, and re-rendered attachments
- Location: /Users/indo/code/project/omon-gateway/src/main.rs:498-512 (event construction), 502 (attachments)
- Trigger: A session has `resume_pending=1` and its last row is a user message with an attachment, or a `[system] ` lane event. The stored `content` is already the output of `render_user_prompt`/`inbound_transcript_content`, which inlines `[Attachment: ...]` and text content. Recovery sets `attachments` from `metadata_json` and `id: Uuid::new_v4()`, `delivery_id: None`, `origin: User`. The actor's `persist_inbound` then inserts a second user row (the `ON CONFLICT(id)` guard does not fire because the id is new). `render_user_prompt` appends the attachment blocks a second time to the prompt. The original `delivery_ledger` row (in the actor-side recovery at actor.rs:161-197 it is looked up) is never completed.
- Impact: The prompt carries duplicated attachment text. The transcript gets a duplicate user row, which affects `/undo`, `/retry` and search. System-origin turns are replayed as user-origin. The ledger row stays `in_progress`.
- Fix: Reuse `unfinished.message_id` as the event id and look up `delivery_id` from `delivery_ledger` as actor.rs does. Preserve the original origin (strip the `[system] ` prefix and set `EventOrigin::System`), and avoid re-attaching attachments already rendered into `content`.

### [P2] Startup authorization ignores `DISCORD_ALLOWED_ROLES` and only trusts discord_rest-sourced authors, so legitimate guild turns are never recovered
- Location: /Users/indo/code/project/omon-gateway/src/main.rs:361-385 (`is_authorized_actor`), 400-437 (`pending_recovery_actor`)
- Trigger: A deployment uses role-only authorization, or a guild user sends a normal message (indexed by `index_inbound` with source `ingress`, which is not `discord_rest`). The gateway restarts mid-turn. `pending_recovery_actor` finds no `discord_rest` document, or the author is not in `allowed_users` or the pairing list. The actor is deferred and the marker is retained.
- Impact: The turn is not resumed on this boot or any later boot. It is only logged as "deferring". This is fail-closed by design, but it practically disables recovery for guild lanes. The `allowed_roles` check exists in live ingress but not here.
- Fix: Persist an author-trust flag at ingress (the real `message.author.id`) and accept it in `pending_recovery_actor`. Resolve roles via REST at startup, or document that role-only users are never recovered.

### [P2] Mail and iMessage watchers die permanently on any single failure, and mail only emits an event when the child exits
- Location: /Users/indo/code/project/omon-gateway/src/main.rs:2285-2295 and 2242-2250 (mail), 2200-2205 and 2167-2172 (imsg)
- Trigger: `zele mail watch` exits non-zero, or exits without printing a line, or `imsg` exits once (network blip, token refresh, permission blip). The function `return`s with only a `warn!`, with no retry or backoff. In the mail loop, lines only set `produced_event`; `on_mail_event` is called only after the child exits (2296-2300). If `zele mail watch --interval 60` is long-running, no mail events are emitted while it runs. I could not verify the zele CLI semantics.
- Impact: Mail or iMessage triage events stop until the gateway restarts, with only a warning in the logs.
- Fix: Wrap the spawn loop with bounded exponential backoff and retry. Send `on_mail_event` as each line arrives instead of after exit, if `zele watch` is a streaming command.

### [P2] Buffered debounce batches are dropped on drain, lane-failure and Discord-client-failure shutdowns
- Location: /Users/indo/code/project/omon-gateway/src/main.rs:1233-1238 (lane arm), 1239-1244 (join_set arm), 1254-1262 (drain arm)
- Trigger: In the drain arm, `global_debouncer().flush_all` runs while `multiplexer.is_draining()` is true. `route` returns "gateway is draining", so `route_claimed_event_with_constituents` marks the delivery failed (adapter.rs:1577-1580) and `flush_all` only logs the error. The lane-failure and join_set arms never call `flush_all` or `mark_in_flight_resume_pending` at all. `discord_task_failed` then ends in `process::exit(1)`.
- Impact: A user message inside the ~600ms debounce window at shutdown is lost. Replay depends on `DISCORD_MISSED_BACKFILL`, which defaults to off.
- Fix: Flush before the drain flag takes effect, or bypass the drain gate for the shutdown flush. Call the same flush and mark sequence in all shutdown arms.

### [P2] `parse_bool_from` treats unknown tokens as false, so a typo disables default-true safety flags
- Location: /Users/indo/code/project/omon-gateway/src/main.rs:2366-2376, used at ~268 (`APPROVALS_DESTRUCTIVE_SLASH_CONFIRM`, default true), 174-179 (`DISCORD_PROCESSING_REACTIONS`) and 1181/1975 (`OMON_LANES_ENABLED`)
- Trigger: `APPROVALS_DESTRUCTIVE_SLASH_CONFIRM=enabled` (or `y`, `t`) is set.
- Impact: The value parses to false, silently disabling the destructive-slash confirmation. The same applies to `TIRITH_FAIL_OPEN` and `OMON_KIBITZER_ENABLED`.
- Fix: Reject unknown values, or fall back to the default when the token is not recognised as true or false.

### [P2] `DISCORD_ALLOWED_USERS` is parsed lossily, so a malformed value falls back to open-pairing mode
- Location: /Users/indo/code/project/omon-gateway/src/main.rs:105-112 (compare 128 and 2378-2386)
- Trigger: `DISCORD_ALLOWED_USERS=<@123>` or usernames. `filter_map(...ok())` yields an empty list. With no allowed roles, `should_prompt_unauthorized_dm` (adapter.rs:1671) returns true for `allowed_users.is_empty() && allowed_roles.is_empty()`.
- Impact: Instead of refusing to start (as `DISCORD_ALLOWED_CHANNELS` does via `parse_allowlist`), the gateway sends pairing prompts to every unauthorized DM sender. Guild messages stay denied.
- Fix: Use `parse_allowlist` for `DISCORD_ALLOWED_USERS` and `DISCORD_ALLOWED_ROLES`.

### [P2] Startup opens System Settings on every boot while Full Disk Access is denied
- Location: /Users/indo/code/project/omon-gateway/src/main.rs:656-700
- Trigger: A launchd-managed gateway without FDA restarts (including crash loops). `open x-apple.systempreferences:...` runs on every boot.
- Impact: A privacy pane is repeatedly forced open on a headless host, and a synchronous `.status()` runs inside async startup.
- Fix: Make it opt-in via an env flag, or record "already prompted" in state.

### [P2] Hardcoded owner id and mail account
- Location: /Users/indo/code/project/omon-gateway/src/main.rs:1050 (`owner = "414011632306618368"`) and 2236 (`freedomzero91@gmail.com`)
- Trigger: Any deployment other than the author's.
- Impact: The digest registers only for the hardcoded Discord owner, and the mail watcher polls a fixed account.
- Fix: Read both from env or config.

### [P2] `build_cron_llm_config` pairs a job-supplied `base_url` or provider with the base config's API key
- Location: /Users/indo/code/project/omon-gateway/src/main.rs:594-619
- Trigger: Currently dead code (only tests call it). If it is wired up, a Hermes job payload with `base_url` or a `provider` override keeps the base `api_key`.
- Impact: The OpenAI or Anthropic key would be sent to a job-controlled host, or to a different provider.
- Fix: Reset or re-resolve `api_key` when `provider` or `base_url` is overridden, and allow only an allowlist of hosts.

### [P1] Boot-time drain marker makes `wait_for_drain` fire immediately, contradicting its documented contract and the test that pins it
- Location: /Users/indo/code/project/omon-gateway/src/main.rs:1006-1022 (scan then `receiver()`), :1362 (select arm), :1420-1429 (`wait_for_drain`). Test: `fix_r5_main_drain_wait_ignores_clearing_a_boot_time_drain` at ~:3271.
- Trigger:
  1. A valid `.drain_request.json` exists at boot. It is under 1h old, and its epoch is empty or matches. On macOS `current_instantiation_epoch()` is `""`, so any fresh marker counts.
  2. `drain_watcher.scan_at(...)` runs before `drain_watcher.receiver()`, and flips the channel to `true`. That bumps the watch version.
  3. `receiver()` clones the struct's own `drain_rx`, which still carries `Version::INITIAL` (tokio `Receiver::clone` copies `self.version`, watch.rs:1004-1011).
  4. The first `drain_rx.changed()` therefore resolves immediately, because the versions differ (watch.rs:961-973).
  5. `*drain_rx.borrow()` is `true`, so `wait_for_drain` returns on its first poll.
- Impact:
  - The doc comment says a drain already active at boot is the initial value and must not end the wait. In production it does end the wait.
  - After all startup work, the gateway immediately runs the graceful-shutdown path and exits.
  - Under launchd or systemd with a marker that persists (nothing in these files removes it), the gateway restarts and exits in a loop until the marker is cleared or expires after 1h.
  - Each boot with pending sessions also calls `restart_guard.check_and_record()`. That can trip the restart-loop breaker, so later legitimate boots skip auto-resume.
  - The headless path (`headless_lead` plus `watcher.serve`) stays up while draining. The two modes therefore disagree.
  - The new test cannot catch this. It builds `watch::channel(true)`, which has no version bump, so it does not model the production receiver.
- Fix: call `drain_rx.mark_unchanged()` right after `let mut drain_rx = drain_watcher.receiver();`, or `borrow_and_update()` before the loop. Add a test that uses a real `DrainWatcher` with a marker already present at construction.

### [P1] A drain cancel racing a watcher scan reads as "unreadable" and flips the gateway to draining (fail-closed change introduced this round)
- Location: /Users/indo/code/project/omon-gateway/src/drain_control.rs, `check_drain_requested_at` (`path.exists()` followed by `fs::read_to_string` with the new `Err` => `Some("unreadable-marker")`). Consumer: main.rs:1362 / :1420.
- Trigger (narrow window, but concrete):
  1. The scan sees `path.exists() == true`.
  2. `clear_drain_request` (the cancel) removes the file.
  3. `read_to_string` returns `NotFound`.
  4. The code treats `NotFound` as an unreadable marker and reports draining.
- Impact:
  - `scan_at` sets the watch to `true`, which is a false-to-true transition. This is a spurious drain request produced by the cancel itself.
  - `wait_for_drain` resolves and the gateway shuts down, or at minimum refuses turns until the next 3s tick.
  - The inverse hole: if `exists()` returns false because the parent directory is not traversable (EACCES), an unreadable state is reported as absent. That is the exact confusion the fix claims to remove.
- Fix: skip the `exists()` pre-check. Call `fs::read_to_string` directly, map `ErrorKind::NotFound` to `None`, and fail closed on every other error.

### [P2] Guild resume-pending recovery is effectively dead because the actor lookup trusts only `source = 'discord_rest'` rows
- Location: /Users/indo/code/project/omon-gateway/src/main.rs:403-441 (`pending_recovery_actor`). Writers: /Users/indo/code/project/omon-gateway/src/storage/message_search.rs (`index_inbound_with_metadata` forces `"source":"ingress"`) and /Users/indo/code/project/omon-gateway/src/discord/adapter.rs:2135 (`event.author_id = Some(message.author.id)`).
- Trigger: a guild turn is in flight at restart. The live ingress index row has source `ingress`, with the real author id. The lookup requires `discord_rest`, which only appears after a message_context backfill upserts that message. The `legacy_guild_pending_auth` fallback only covers upgrade-time rows.
- Impact:
  - The actor resolves to `None`, recovery is deferred, and the marker is retained and logged on every boot.
  - The comment's rationale (author derived from historical session state) is stale. Ingress now records the true author.
- Fix: also accept `source = 'ingress'` rows whose `author_id` is non-empty. Alternatively, document that guild recovery needs REST provenance.

### [P2] `recovery_event` turns a human-typed "[system] ..." message into a System-origin event
- Location: /Users/indo/code/project/omon-gateway/src/main.rs:495-516. The transcript prefix is written at /Users/indo/code/project/omon-gateway/src/multiplexer/actor.rs:1441-1447.
- Trigger: an authorized user's message literally starts with `[system] `, and the turn is resumed after a restart.
- Impact:
  - The replay is routed as `EventOrigin::System`, so it goes through `EventIfIdle`. That path returns "lead session is busy" if the session is busy.
  - It also skips the user-only indexing and kibitzer arming (actor.rs:1087-1092).
  - The marker is restored on route error, so this is not data loss, but the replayed turn behaves differently from the original.
- Fix: escape or tag system transcripts unambiguously, for example with a metadata flag or a prefix that user text cannot produce, rather than inferring origin from content.

### [P2] `StartupAuthorization` ignores `DISCORD_ALLOWED_ROLES`
- Location: /Users/indo/code/project/omon-gateway/src/main.rs:364-390 (`is_authorized_actor`).
- Trigger: a deployment that authorizes guild users only by role, with `DISCORD_ALLOWED_USERS` empty.
- Impact: roles cannot be resolved at boot, so no actor is ever authorized. Every pending marker is deferred indefinitely and logged each boot.
- Fix: document that role-only users are not auto-resumed, or recover them via first-message re-auth.

### [P2] Runtime lock identity is not normalized for relative paths
- Location: /Users/indo/code/project/omon-gateway/src/entry.rs, `runtime_lock_name`.
- Trigger: one gateway uses `DATABASE_URL=sqlite://./x.db` and a second uses `sqlite://x.db`, or the DB is reached through a symlink or `..`.
- Impact: `cwd.join("./x.db")` and `cwd.join("x.db")` hash differently, so both gateways acquire different locks on the same database.
- Fix: normalize components lexically, or canonicalize the parent directory before hashing.

### [P2] Smaller items
- /Users/indo/code/project/omon-gateway/src/main.rs:1407-1411: `std::process::exit(1)` skips destructors, so `OmoDaemonSupervisor::drop` does not kill the spawned `omo app-server`. The next boot adopts it via `readyz`, which is probably acceptable, but the orphan survives.
- /Users/indo/code/project/omon-gateway/src/drain_control.rs, `write_drain_request`: a failed `fs::write` leaves a partial `.drain_request.*.tmp` file. Only rename failure cleans up.
- /Users/indo/code/project/omon-gateway/src/main.rs:518-575: `clear_session_resume_pending` runs before `route`. A crash between the two loses the retry marker. The window is small and pre-existing.
- /Users/indo/code/project/omon-gateway/src/main.rs:641-672: `build_cron_llm_config`, including this round's key-stripping fix, has no non-test caller in the files I searched, so the fix is currently unexercised in production.

Confirmed correct this round (no defects found):
- `parse_allowlist` fail-closed on set-but-unparsable lists.
- `claim_fda_prompt` using `create_new`.
- The O_NOFOLLOW / 0600 / 0700 lock handling and its tests.
- `RuntimeOwnershipLock` flock semantics.
- Credential provider prefix matching.
- Probe `no_proxy`.
- Disk classification.
- `stop_lead_runtime` joining actors and services.
- The inbox watcher supervisor backoff.
- The `lane_service_exit` handling.

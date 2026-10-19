# r3-dashboard-main review

Checked by reading code only: types, imports, shadowing, borrow/move, callers, `tests/`, and `web/src`.

**Verified clean**
- **`home_dm` rename, `src/main.rs:1052-1062`:**
  - The `let home_dm: Option<String>` binding is shadowed by `if let Some(home_dm)` (a `String`), then by `let home_dm = home_dm.parse::<SessionKey>()?` (a `SessionKey`).
  - `register` takes `&SessionKey`, so the types line up.
  - `?` converts `SessionKeyParseError` into `OmonError` through an existing `From` impl (`src/models/session.rs:22`).
  - `default_lead()` returns an owned `String` and `.bind()` accepts it. `owner` is a `&str`.
  - The query matches the sessions schema: DM keys keep `user_id` and have a NULL `guild_id`.
- **DM exemption, `src/main.rs:366-387`:**
  - Both callers (`:454` and `:488`) and `is_authorized` (`:363`) go through the one function.
  - DM detection (`guild_id.is_none()`) matches live ingress. `src/discord/adapter.rs:2005-2013` builds the key with `guild_id=None` when `is_dm`. The live check at `:1832` and `is_channel_authorized` in `src/discord/commands.rs:260` also exempt DMs from `allowed_channels`.
  - The ignore list is still applied to DMs, as in live ingress. Thread sessions store the parent id in `channel_id`, so startup and live checks stay consistent.
  - The new test uses `SessionKey::new("discord", None::<String>, ..., "42")`, which keeps `user_id` for DMs, so `is_authorized` sees actor 42. Its three assertions hold by trace (the guild key has an empty `user_id` and is rejected by the allow-list first).
  - The `StartupAuthorization` literals in the other tests use `..Default::default()` or all five fields, so nothing breaks.
- **`create_bot`, `src/dashboard.rs:1951-2000`:**
  - The `ON CONFLICT(bot_id) DO NOTHING` form is valid SQLite.
  - `rows_affected() == 0` returns 409 through `ApiError::new(StatusCode, impl Into<String>)`, and `sqlx::Error` converts through the existing `From` impl.
  - The partial moves (`payload.enabled_toolsets`, `payload.custom_settings`) are disjoint from the borrows of `bot_id`, `name`, `model` and `system_prompt`.
  - The new test compiles plausibly: the `post` closure is `Fn`, it is called twice, and `pool` is cloned before `router(state)` consumes `state`.
  - **No caller expects the old upsert.** Each other POST `/api/bots` in `src/dashboard.rs` tests uses a unique `bot_id`, or is a cross-origin POST rejected before the DB. `tests/` has no `/api/bots` use.
  - On the web side, `web/src/pages/BotsPage.tsx` `handleCreate` only POSTs, and its error path shows `Create failed: ...` in an alert. `web/src/App.tsx` `handleCreate` is the cron dialog and is unrelated.
- **`standalone_scheduler_enabled`, `src/dashboard_runtime.rs:20-45`:**
  - `super::optional_env` is a private fn in `src/main.rs`, reachable from the child module `legacy::dashboard_runtime`. It is already used the same way for `APPROVAL_TIMEOUT_SECS`.
  - The test `use super::*` reaches the private function.
  - `src/entry.rs:125` goes through `run_standalone_cli`, which reads the env var. `src/entry.rs:220` passes `false`. `tests/test_review_dashboard.rs:364` passes `false`. No caller expects the standalone dashboard to run a scheduler.
  - That test only does a manual `POST .../trigger`, which does not need `scheduler.start()`.
- **`src/entry.rs`:** nothing in it needs changing for these fixes.

### [P2] Standalone-scheduler opt-in is undocumented
- Location: /Users/indo/code/project/omon-gateway/web/README.md:21
- Trigger: An operator runs `omo-gateway dashboard` or `serve` with no gateway process and creates an enabled cron job in the UI.
- Impact: The job never fires, because the standalone dashboard no longer starts its scheduler by default. Nothing documents `OMON_DASHBOARD_RUN_SCHEDULER=1`. Manual trigger still works. This is a deliberate behavior change.
- Fix: Add one line to `web/README.md`, and optionally a `tracing::info!` at standalone startup saying the scheduler is disabled.

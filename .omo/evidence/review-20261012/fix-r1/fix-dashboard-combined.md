# fix-dashboard-combined

Scope: src/dashboard.rs, src/dashboard_runtime.rs (src/entry.rs unchanged).
Resumed from a prior attempt that had already added the `DashboardState::detached_from_gateway` flag/builder and the four `fix_dash_*` tests (in `src/dashboard.rs` `#[cfg(test)] mod tests`); no fix code existed yet. This run captured RED, then applied the fixes.

Logs:
- RED:   `.omo/evidence/review-20261012/fix-r1/fix-dashboard-combined-red.log`
- GREEN: `.omo/evidence/review-20261012/fix-r1/fix-dashboard-combined-green.log`

RED command (all 4, unfixed code): `cargo test --test test_review_dashboard fix_dash` -> exit 101, `0 passed; 4 failed`.
GREEN command (whole dashboard test target, regression included): `cargo test --test test_review_dashboard` -> exit 0, `78 passed; 0 failed`.

Combined-mode wiring: `dashboard_runtime::run_standalone` now builds its state with `.detached_from_gateway()`. It is used by both combined `run` (entry.rs:201) and standalone `dashboard`; neither shares the Discord runtime, so both are flagged. Tests and other constructors keep the default `false`, so the attached/shared-runtime path (`tests::attached_dashboard_controls_live_runtime`) behaves as before and still passes.

## 1. [P1] Web dispatcher reported false Discord delivery: FIXED
- Change: `impl OutboundDispatcher for WebDashboardDispatcher::dispatch` (src/dashboard.rs). It still records approvals and broadcasts every action to websockets, but `SendMessage`/`EditMessage`/`DeleteMessage`/`UploadFile`/`Stream` on a session whose `platform != "web"` now return `Err(OmonError::Config("web dashboard dispatcher cannot deliver to <platform> sessions"))`. Typing/React/approval actions are unchanged.
- Follow-on change: `CompositeDispatcher::dispatch` and `dispatch_direct_reply` used `?` on the dashboard leg. That would have aborted before the primary (Discord) send, so they now treat the dashboard leg as observation only: for web sessions they return its result, otherwise they return the primary's result.
- src/cron/scheduler.rs not edited. Because the dispatcher now errors, the scheduler's existing error path records the obligation as `failed`.
- Test: `legacy::dashboard::tests::fix_dash_web_dispatcher_fails_closed_for_messenger_delivery`. It checks the dispatcher directly (web Ok, discord Err, both still broadcast), then runs a real `CronScheduler::with_dispatcher(..., WebDashboardDispatcher)` executing a `deliver: discord:999` job and asserts `execute_job` returns Err and `delivery_obligations.state == ["failed"]`.
- RED excerpt:
  ```
  panicked at tests/../src/dashboard.rs:3517:9:
  web dashboard dispatcher must not report Discord delivery as successful
  ```
- GREEN: `test legacy::dashboard::tests::fix_dash_web_dispatcher_fails_closed_for_messenger_delivery ... ok`

## 2. [P1] Separate SmartApprovalGuard hid Discord approvals: FIXED (visible-failure fallback) + BLOCKED-SCOPE for true sharing
- Why sharing can't be done in scope: `SmartApprovalGuard` (src/discord/approval.rs:129) keeps pending approvals only in memory (`pending: Arc<Mutex<HashMap<Uuid, PendingApprovalEntry>>>` holding `oneshot::Sender`s). The DB pool only persists the allowlist and yolo state, so the dashboard cannot read or resolve the gateway's pending approvals through the shared pool.
- Implemented fallback (src/dashboard.rs):
  - `list_pending_approvals`: when `detached_from_gateway`, the response adds `"complete": false`, `"gateway_approvals_visible": false` and a `"warning"` explaining that messenger approvals are not listed and must be resolved in the messenger.
  - `resolve_approval`: an unknown id on a detached dashboard returns 409 CONFLICT with the same explanation instead of a plain 404.
  - The message lives in the constant `DETACHED_APPROVALS_WARNING`.
- BLOCKED-SCOPE (proper fix, needs src/main.rs): in combined mode, the gateway runtime in `run_gateway_public` (main.rs) should build the dashboard from its own `SmartApprovalGuard`, `SessionMultiplexer` and a `CompositeDispatcher::new(discord_dispatcher, events)`, like the attached test setup in entry.rs:~410 (`attached_dashboard_controls_live_runtime`). That means exposing a main.rs entry that accepts `DashboardSettings` and spawns `dashboard::spawn_server` with the shared state, then having entry.rs call it instead of `dashboard_runtime::run_standalone`. That state would not be flagged `detached_from_gateway`, so the fallback guards above would turn off automatically.
- Test: `legacy::dashboard::tests::fix_dash_detached_dashboard_reports_gateway_approvals_unavailable`.
- RED excerpt:
  ```
  FIX-DASH-2 pending={"items":[],"pending_count":0}
  panicked at tests/../src/dashboard.rs:3575:9:
  assertion `left == right` failed
    left: Null
   right: false
  ```
- GREEN: `test legacy::dashboard::tests::fix_dash_detached_dashboard_reports_gateway_approvals_unavailable ... ok`

## 3. [P1] Dashboard chat drove Discord sessions through a second multiplexer: FIXED
- Change: new helper `reject_detached_messenger_session(state, key)` in src/dashboard.rs returns 409 CONFLICT when `detached_from_gateway && key.platform != "web"`. It is called in `post_chat` (after key resolution), in `stop_session`, and in `handle_session_socket` for both the `message`/`chat` and `stop` websocket messages (which send a `{"type":"error"}` frame). Web sessions are unaffected.
- Test: `legacy::dashboard::tests::fix_dash_detached_dashboard_rejects_chat_and_stop_for_messenger_sessions`. It covers HTTP chat, HTTP stop, a web-session control (expects 202), and a real loopback websocket chat on the Discord session (expects an `error` frame).
- RED excerpt:
  ```
  FIX-DASH-3 chat=202 Accepted stop=200 OK web_chat=202 Accepted ws_reply={"session_id":"7:discord|1:g|3:123|-|0:","type":"accepted"}
  assertion `left == right` failed
    left: 202
   right: 409
  ```
- GREEN: `test legacy::dashboard::tests::fix_dash_detached_dashboard_rejects_chat_and_stop_for_messenger_sessions ... ok`

## 4. [P2] create_cron_job silently overwrote an existing id: FIXED
- Change: `create_cron_job` (src/dashboard.rs) now returns 409 CONFLICT `cron job <id> already exists` when `state.scheduler.get(&id)` returns `Some` before `register_with_id`.
- Test: `legacy::dashboard::tests::fix_dash_create_cron_job_rejects_duplicate_id`. It asserts the second POST returns 409 and the stored payload still contains "original".
- RED excerpt:
  ```
  FIX-DASH-4 second_status=201 Created payload={"content":"overwrite"}
  assertion `left == right` failed
    left: 201
   right: 409
  ```
- GREEN: `test legacy::dashboard::tests::fix_dash_create_cron_job_rejects_duplicate_id ... ok`

## Notes for lead
- The editor hook auto-ran rustfmt on src/dashboard.rs and src/dashboard_runtime.rs after my edits. In dashboard_runtime.rs that also reformatted pre-existing lines (+26/-32, import ordering and line wrapping, no semantic change). Expect `cargo fmt --check` to be clean for these files.
- Clippy was not run, per the rules.

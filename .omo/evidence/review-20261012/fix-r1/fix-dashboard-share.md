# fix-dashboard-share

RED/GREEN: deferred to single final integration gate (user directive: stop all builds).
No cargo command finished in this node. The one RED attempt (`cargo test --bin omo-gateway fix_share_combined_dashboard_resolves_gateway_approvals`) hit its 1800s deadline while still compiling (load avg ~70). The retry was killed under the stop directive. `fix-dashboard-share-red.log` holds only lock/compile lines, with no test result.

## Defect 1 [P1] Combined-mode dashboard used its own SmartApprovalGuard/multiplexer: FIXED (source only, unverified by build)

### Files and functions changed
- `src/main.rs`
  - New `attach_gateway_dashboard(...)`: builds a `CompositeDispatcher::new(discord_egress, events)` and installs it on the gateway's `DiscordApprovalRequester` and `SharedDispatcher`, so approvals and session output reach both Discord and the dashboard. It returns `DashboardState::new(...)` using the gateway's own guard (`approval_requester.guard().clone()`), multiplexer, scheduler, tools and `config_view_from_gateway`. It is NOT flagged `detached_from_gateway`.
  - `run_gateway()` now delegates to the new `run_gateway_with_dashboard(Option<DashboardSettings>)`. In the Discord path, when settings are given, it spawns `dashboard::spawn_server` after the scheduler and Discord clients are ready, and cancels and awaits the server on shutdown. With `None`, behavior is unchanged.
  - Added `discord_egress_dispatcher: Arc<dyn OutboundDispatcher>` next to the existing `shared_dispatcher.set(...)`.
- `src/entry.rs`
  - `legacy::run_gateway_with_dashboard_public(settings)` added.
  - `run_gateway_with_optional_dashboard`: when the dashboard is enabled and `OMON_DISCORD_ENABLED` is true (the default), it runs the gateway with the attached, shared dashboard. When Discord is disabled (the headless lane runtime has no Discord approvals), it keeps the previous `run_standalone` path.
- Standalone `dashboard`/`serve` mode is unchanged: it still uses `run_standalone`, which is still `.detached_from_gateway()`.
- `src/dashboard.rs` was not edited.

### Test (written, not run)
- `runner_tests::fix_share_combined_dashboard_resolves_gateway_approvals` in `src/main.rs`.
  - Wires a gateway-style `DiscordApprovalRequester` and `SharedDispatcher` over a recording primary dispatcher, then calls `attach_gateway_dashboard`.
  - Raises an approval through the gateway requester and waits for the `ApprovalRequest` to reach the primary egress (bounded event wait, no sleeps).
  - Asserts the id is listed by `GET /api/approvals/pending` and that `POST /api/approvals/{id}/resolve {"decision":"once"}` returns 200.
  - Asserts the requester receives `Ok(ApprovalDecision::Once)` and the state is not detached.
- Expected RED on the old wiring (separate guard, separate `WebDashboardDispatcher`, detached): `items` is empty, so the "gateway approval must be listed by the dashboard" assertion fails.

### Commands for the final gate
- `cargo test --bin omo-gateway fix_share_combined_dashboard_resolves_gateway_approvals`
- `cargo test --test test_review_dashboard` (this target includes `entry.rs`/`main.rs`, so the new code compiles there too)

### Notes for the lead
- `git diff` on `src/main.rs` and `src/dashboard_runtime.rs` also contains earlier, unrelated edits from other fix nodes (fix-main, and rustfmt-style reflow plus `.detached_from_gateway()` from fix-dashboard-combined). These were left as found.
- Compile risk to check at the gate: `attach_gateway_dashboard` uses `dashboard::CompositeDispatcher` and `dashboard::config_view_from_gateway`. Both exist in `src/dashboard.rs` (the latter is `pub(crate)` and takes `&super::Config`, which resolves to main.rs's `Config` inside `legacy`). It has not been compiled.

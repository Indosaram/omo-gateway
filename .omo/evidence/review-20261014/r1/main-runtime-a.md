# Lane main-runtime-a

### [P1] The Discord-free (headless) gateway never starts the local omo app-server, so every lead, cron and lane turn fails
- Location: src/main.rs:825
- Trigger: Run the gateway as the only service on a host with `OMON_DISCORD_ENABLED=false` and `OMON_LANES_ENABLED` true or unset. This is the mode described in docs/AGENTS_DELEGATE.md:11. Leave `OMON_OMO_APPSERVER_URL` unset (it defaults to `ws://127.0.0.1:19742`) and leave autospawn at its default (on), with no app-server already running.
- Impact:
  - The Discord path calls `OmoDaemonSupervisor::ensure` (main.rs:995, and :1123 for cron), and so does the dashboard runtime (dashboard_runtime.rs:147).
  - The headless branch (main.rs:803-944) builds `OmoBackendConfig` and several `OmoBackend`s but never calls `ensure`. Nothing spawns or supervises the daemon.
  - Every turn fails to connect. That includes `headless_lead` lane-event turns, `CronScheduler`/`AgentCronExecutor` runs, and `LaneRunner` turns. After 3 errors the lane runner marks lanes failed, and cron runs are recorded as failures.
  - `force_restart_owned_daemon` cannot recover this, because no daemon is registered as owned.
  - The headless tests can't catch it: they always pass an explicit external WS server.
- Evidence: `let backend_config = OmoBackendConfig::from_env()?.with_workspace_root(workspace.clone());` is followed directly by `headless_lead(...)` and `OmoBackend::new(OmoBackendConfig::cron_from_env()?...)`. There is no `OmoDaemonSupervisor::ensure` anywhere in 803-944. By contrast, the Discord path has `let _daemon_supervisor = OmoDaemonSupervisor::ensure(&omo_config).await?;` at 995.
- Fix: In the headless branch, hold `let _daemon_supervisor = OmoDaemonSupervisor::ensure(&backend_config).await?;`, plus the cron URL guard used at 1120-1124, for the whole branch. `ensure` already returns `None` when an external daemon is serving, so co-located deployments are unaffected.

### [P2] `OMON_OWNER_ID` applies only to digest registration; the proposals store keeps the built-in owner
- Location: src/main.rs:1180
- Trigger: Set `OMON_OWNER_ID` to someone other than `414011632306618368`, with inbox triage enabled.
- Impact:
  - The digest cron is registered for the configured owner (main.rs:1149-1162).
  - Proposals are created for the hardcoded `OWNER_ID`, and the owner-reply handler checks the same constant (adapter.rs:902, 909).
  - The configured owner gets digests that list proposals, but their replies are never treated as owner approvals. The built-in id still holds approval rights.
  - This is a partial fix of the recorded "hardcoded owner id" P2. The new trigger is the half-applied override.
- Evidence: `let owner_id = owner_id_from(optional_env("OMON_OWNER_ID").as_deref());` at 1149, versus `Proposals::new(pool.clone(), OWNER_ID).await?;` at 1180.
- Fix: Pass `owner` (the `owner_id` string) to `Proposals::new` at 1180. Thread the same id into `PoiseData` so the adapter's owner check uses it instead of the constant.

### [P2] `APPROVAL_TIMEOUT_SECS + 5` can overflow
- Location: src/main.rs:939 (also :946)
- Trigger: Set `APPROVAL_TIMEOUT_SECS` to a value within 5 of `u64::MAX`. `approval_timeout_secs_from` accepts any positive u64.
- Impact:
  - Debug builds panic at startup.
  - Release builds wrap, so the tool approval timeout drops to 0-4 seconds. Approvals then time out almost immediately and approval-gated commands are denied.
  - dashboard_runtime.rs:121 already uses `saturating_add(5)` for the same value.
- Evidence: `std::time::Duration::from_secs(config.approval_timeout_secs + 5),`
- Fix: Use `config.approval_timeout_secs.saturating_add(5)` at both sites, or clamp the value in `approval_timeout_secs_from`.

I re-opened every cited line and it contains the quoted code. All three findings come from reading the code only; per the task, I ran no build or tests.
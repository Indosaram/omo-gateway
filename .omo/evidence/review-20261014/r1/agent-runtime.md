# Lane agent-runtime

### [P1] Starting a standalone dashboard while the gateway runs kills the gateway's live daemon, and the gateway stops supervising permanently
- Location: src/agent/omo_daemon.rs:313
- Trigger: The gateway is running and owns the daemon on `ws://127.0.0.1:19742`. Its pid is in `~/.omon/omo-appserver-19742.pid`. The operator then starts `omon dashboard` or `omon serve` without `OMON_DASHBOARD_RUN_SCHEDULER`. That path takes no runtime lock: entry.rs:117-121 says it "never competes with a gateway". `dashboard_runtime.rs:147` calls `OmoDaemonSupervisor::ensure` with the same default URL and the same HOME. `ensure` then calls `reclaim_leftover_daemon` (omo_daemon.rs:459).
- Impact:
  - The live gateway daemon gets SIGTERM, then SIGKILL, so every turn in flight (interactive, cron, lane) is aborted.
  - The dashboard then starts its own daemon and overwrites the pidfile.
  - The gateway's watcher sees its child exit and probes readyz. The dashboard's daemon answers, so the watcher logs "deferring to the external daemon" and returns for good (omo_daemon.rs:589).
  - When the dashboard exits, its daemon is dropped with it (`kill_on_drop`). The gateway is left with no daemon and no supervisor, and every turn fails until the gateway is restarted by hand.
  - It also happens in reverse: restarting the gateway kills the dashboard's daemon.
- Evidence: `(command.contains("app-server") && command.contains(&needle)).then_some(pid)`. A pid counts as a "leftover from a previous gateway" just because it is alive and has a matching command line. Nothing checks whether the process that spawned it is still running. The watcher's `return;` at :589 then makes losing supervision permanent.
- Fix:
  - Record the owner gateway's pid with the daemon pid, and reclaim only when that owner is dead. For example, write `daemon_pid owner_pid` and check the owner with `kill -0`.
  - In the watcher, do not `return` when another daemon is serving. Keep polling readyz and respawn once the port is free again.

### [P1] One slow first response force-kills the shared app-server, which aborts every other in-flight turn
- Location: src/agent/omo_daemon.rs:685
- Trigger:
  - An interactive turn gets no agent output for 45s after the turn is acknowledged, for example a model that thinks for a long time first. omo_backend.rs:1203 returns "agent produced no output within", and `is_backend_stall` (omo_backend.rs:1857) counts that as a stall.
  - omo_backend.rs:2021 then calls `force_restart_owned_daemon`.
  - Cron (`cron_from_env`) and lanes (`lane_from_env`, lanes/runner.rs:974) use the same daemon URL by default.
- Impact:
  - The function SIGKILLs the single daemon that serves every session, so concurrent lane turns (budgets up to 1h), cron jobs and other users' turns are cut off mid-run. Some of them may already have had side effects.
  - The only throttle is one restart per 120s, which does not stop this from repeating.
  - The stalled turn itself is resubmitted after the restart.
- Evidence: `tracing::warn!(..., "force-restarting unresponsive omo app-server"); let _ = child.start_kill();`. Nothing checks whether the daemon is actually wedged before the kill. It does not probe readyz and does not look at other active turns.
- Fix: Before killing, confirm the daemon is unresponsive: `probe_readyz` fails, or a JSON-RPC ping times out. A healthy daemon behind a silent model should not be restarted.

### [P2] Daemon log rotation only happens at spawn, so a long-lived daemon's log grows without limit
- Location: src/agent/omo_daemon.rs:230
- Trigger: A daemon that stays up for days or weeks and writes a lot of prompt and tool output.
- Impact: `~/.omon/omo-appserver-<port>.log` grows without a cap. This contradicts the stated guarantee "so it cannot grow without bound" and can fill the disk.
- Evidence: `if std::fs::metadata(path).is_ok_and(|meta| meta.len() > MAX_LOG_BYTES) { let _ = std::fs::rename(...) }` only runs inside `open_private_log`, which only `daemon_command` calls, at spawn time. The running child keeps appending through its inherited fd.
- Fix: Check the size periodically in the watcher loop. When it is over the limit, rotate with copy-truncate, since the child holds an `O_APPEND` fd. Alternatively, pipe stdout and stderr through the gateway and rotate there.

All cited lines were re-read in this session: 230, 304-313, 459, 587-589 and 681-685 in omo_daemon.rs, plus the callers in dashboard_runtime.rs:147, entry.rs:117-121 and omo_backend.rs:1203, 1857 and 2021. The other in-scope files had no new defects beyond the P2s already declined in review-20261013/CONFIRMED.md: omo_config.rs, llm.rs, agent_workspace.rs (its slug slicing only ever slices ASCII), workspace_migration.rs and mod.rs.
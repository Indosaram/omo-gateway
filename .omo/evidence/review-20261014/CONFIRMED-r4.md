# CONFIRMED-r4: production crash loop after the deploy (2026-10-15)

## Incident
- From about 09:54 KST the gateway crash-looped (431+ restarts; the watchdog alerted after 139 minutes).
- `OmoDaemonSupervisor::ensure` killed the app-server and returned Err when readyz was not 200 within 30s. Every caller propagates that with `?`, so the process exited 1 and launchd/the watchdog restarted it into the same timeout.
- Host load was about 270 on 16 cores, from other projects' cargo/vitest runs. A standalone app-server on port 19799 became ready in 19s, so the daemon was healthy, just slow.
- The code was pre-existing (not introduced by e57b472). The e57b472 deploy started and verified fine at 06:59.

## Fix F18 (src/agent/omo_daemon.rs)
- ensure() never fails on slow readiness. It keeps the gateway running and hands the daemon to the watcher.
- A first child that is still unready after READY_WAIT_MAX (180s) is killed, guarded by its PID, so the watcher respawns it.
- The replacement readiness deadline grows per unready restart (30s up to the 180s cap).
- An early child exit is logged at error level.
- Test: readiness_wait_grows_per_unready_restart_and_is_capped. The existing replacement test was adjusted to ready_wait(0).

## Re-review
| Lane | Result |
|---|---|
| r4a (st_01a13dd6) | The first fix was NOT FIXED overall: 2 P1 (a hung first child is never replaced; flat 30s on replacements). Both were addressed in v2. |
| r4b (st_01a13e31) | v2: 0 P0/P1, 3 P2. Fix VERIFIED. The race (b) was closed by the first_pid guard. |

- P2s recorded only:
  - force_restart gives up at 90s under load (one turn fails, then recovers);
  - after a hung first child the ramp resets to 30s;
  - no paused-time test for the slow-start kill task.

## VERDICT
Final re-review round r4: 0 confirmed P0, 0 confirmed P1.

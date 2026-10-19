# Lane r4a-daemon-slow-start

The fix does stop the incident's crash loop, but it leaves two P1 gaps. A daemon that never becomes ready is never recovered, and the restart path still kills slow starters after 30 seconds. This was a read and grep review only; nothing was built or run.

### [P1] A first daemon that never becomes ready is never killed or replaced, so the gateway runs with a dead backend until someone restarts it by hand
- **Location:** `/Users/indo/code/project/omon-gateway/src/agent/omo_daemon.rs:540-548`. Also involved: the watcher at `:590-605` and `force_restart_owned_daemon`, triggered from `/Users/indo/code/project/omon-gateway/src/agent/omo_backend.rs:2033-2035`.
- **Trigger:** `omo app-server` starts but never answers readyz with 200. It might never bind the port, deadlock during init, or keep returning 503. This is case (c).
- **Impact:**
  - When `ensure` hands over to the watcher, its inner loop only checks `child.try_wait()` (`:590-605`). A child that is alive but stuck is never touched.
  - The only other thing that kills the child is `force_restart_owned_daemon`. It runs only when `is_backend_stall` matches, which means the errors "agent produced no output within" or "no activity from server" (`omo_backend.rs:1870-1874`). Those happen only after a turn has been submitted to a daemon that responds.
  - A daemon that is not listening makes `connect_ws` fail with "failed to connect to omo app-server" or "timeout connecting" (`omo_backend.rs:207-267`). That error gets one retry (`:1975-2000`) and then goes back to the user. It never reaches the stall path, so no restart happens.
  - Result: every Discord, lane and cron turn fails about 30 seconds after it starts, forever, with nothing recovering the daemon.
  - Before the fix, the same hang produced a crash loop. That was also a total outage, but every launchd restart was a fresh try. After the fix the outage is silent and permanent: the process looks alive to launchd and the watchdog, and startup readiness only logs a warning (`main.rs:1255-1259`).
  - This does not affect the actual incident (slow but healthy daemon), only the never-ready case. I rate it P1: it is not worse than the old total outage, but it removes the only self-healing that existed.
- **Evidence:** The watcher's own replacement path already handles this exact case. At `:650-680` it kills an unready replacement after `READY_WAIT_AFTER_SPAWN` and tries again, and the test `daemon_unready_replacement_is_killed_and_retried` checks that. The first child skips that path entirely.
- **Fix:** On the deadline path, give the first child a longer grace period instead of no limit. In the spawned task, keep polling readyz (or exit) until something like `READY_WAIT_AFTER_SPAWN * 4`. If it is still alive and not ready, `take()` and `kill()` it the same way the replacement path does, so the watcher's exit branch respawns it with backoff. Another option is to pass a readiness deadline into `spawn_watcher_with_backoff` for the first iteration.

### [P1] The restart path still kills a slow-starting daemon after 30 seconds, so the incident can come back inside the process
- **Location:** `/Users/indo/code/project/omon-gateway/src/agent/omo_daemon.rs:650-681` (replacement deadline is `READY_WAIT_AFTER_SPAWN`, 30s). `force_restart_owned_daemon` at `:742-756` waits 30s plus 60s.
- **Trigger:** The same load as the incident (about 270 load on 16 cores, roughly 19s or more to start), combined with any exit of the first daemon or any forced restart after a stall.
- **Impact:**
  - Each replacement that takes longer than 30s is killed (`:672-679`) and retried with backoff of 2, 4, 8 ... up to 60 seconds.
  - Under sustained load, this is the incident's crash loop moved into the process: the backend never comes up, while the gateway itself stays alive.
  - The fix only covers the first spawn.
  - Load at boot is exactly when this happens, and a forced restart from the stall path would land in the same loop.
- **Evidence:** The kill-after-deadline block (`:672-679`) uses the same `READY_WAIT_AFTER_SPAWN`. Nothing grows the deadline per attempt.
- **Fix:** Use a more generous replacement deadline (for example 90–120s) or grow it with `failures` (for example `READY_WAIT_AFTER_SPAWN * (failures+1)`, with a cap). That keeps the hung-daemon recovery while stopping it from killing a healthy slow start.

### [P2] The warning on the exit path is misleading, and a child that exits immediately now only shows up as warnings
- **Location:** `/Users/indo/code/project/omon-gateway/src/agent/omo_daemon.rs:533-547`
- **Trigger:** The child exits before it becomes ready (bad binary version, port conflict, bad token file). This is case (b).
- **Impact:**
  - The log says "not ready yet; continuing startup and supervising it" even though the child is already dead. The old clear Err ("did not become ready") is gone.
  - The watcher notices the exit within about 500ms and respawns with backoff forever. That part is fine.
  - The cost is diagnosability: a permanent misconfiguration now looks like an endless series of "daemon is down; restarting" warnings, not a startup failure.
- **Evidence:** `break` on `child_exited()` falls through to the same `warn!` and `spawn_watcher`.
- **Fix:** Log a separate message, at error level, with the exit status when `child_exited()` caused the break.

### [P2] No test covers the new slow-start path
- **Location:** The test module in `/Users/indo/code/project/omon-gateway/src/agent/omo_daemon.rs:760-1460`.
- **Trigger:** n/a
- **Impact:**
  - No existing test checks the old Err-on-timeout behaviour. The `ensure` tests (`daemon_ensure_owned_local_surface`, `leftover_..._reclaimed`, `force_restart_...`) all use a daemon that is ready right away. So nothing breaks, but nothing protects the new behaviour either.
  - This repo does keep integration tests for daemon behaviour, using Python fixtures and `isolated_home`.
- **Fix:** Add one isolated test, in the style of `daemon_unready_replacement_is_killed_and_retried`: a fixture that serves 503 and then 200, and `ensure` with paused time. Assert `Ok(Some(_))`, that the child is not killed, and that readyz is eventually 200.

### What happens in each case
- **(a) Slow start that becomes ready later:**
  - The fix works. The child is left running and the gateway keeps going.
  - Nothing downstream treats `Ok(Some)` as "ready". `main.rs:828/1003` and `dashboard_runtime.rs:147` only bind `_daemon_supervisor` and build `OmoBackend`; no connection is made at startup. The startup readiness probe (`main.rs:1248-1259`) only warns.
  - The first Discord turn neither hangs nor fails instantly. `connect_ws` retries every 500ms for up to 15 seconds (`omo_backend.rs:222-257`), then setup is retried once after a 500ms cooldown (`:1985-2000`). So a turn that arrives up to about 30 seconds before the daemon is ready still succeeds; earlier turns fail with "failed to connect …" and later turns work.
- **(b) Child already exited:** Covered by the watcher. See the second P1 for the 30s limit on replacements.
- **(c) Hung forever:** Never recovered. See the first P1.

### Compile check (by reading only)
`OmonError` is still used in `prepare_token_file` (`:261,272`) and in the spawn `map_err` (`:509`). `timeout` is used in `probe_readyz` and `force_restart_owned_daemon`, and `sleep` and `Instant` are still used. I found no unused imports and no type mismatch: both return paths are `Ok(Some(supervisor))`, and `bin` and `token_file` move only once on either path.

## Verdict
0 P0 and 2 P1: the incident's crash loop is FIXED, but the change is NOT FIXED overall, because a never-ready first daemon is never recovered and slow-starting replacements are still killed after 30 seconds.

<results>
<files>
- /Users/indo/code/project/omon-gateway/src/agent/omo_daemon.rs - ensure() deadline path at 525-548; the watcher (590-605) only reacts to exit; replacement kill after 30s at 650-681; force_restart at 710-757; tests at 760-1460, none asserting the old Err
- /Users/indo/code/project/omon-gateway/src/agent/omo_backend.rs - connect_ws retries for 15s (207-267); setup retried once (1960-2000); stall-only force_restart (1870-1874, 2033-2035)
- /Users/indo/code/project/omon-gateway/src/main.rs - ensure callers at 828/835/1003/1131; startup readiness only warns at 1248-1259
- /Users/indo/code/project/omon-gateway/src/dashboard_runtime.rs - ensure callers at 147/176; no immediate connection
- /Users/indo/code/project/omon-gateway/src/readiness.rs - probe_backend gives a degraded status, never fatal
</files>
<answer>The crash loop from the incident is fixed. Callers do not treat Ok(Some) as "ready", and early turns get about 30 seconds of connect retries. Two P1 gaps remain. First, a first child that never becomes ready is never killed, because the watcher only reacts to an exit and the stall restart only fires after a turn was submitted; the gateway stays up with a backend that never comes back. Second, replacement daemons are still killed after 30 seconds, so the same load turns any later restart into a loop inside the process. There are no compile issues by reading, and no tests asserted the old behaviour.</answer>
<next_steps>Add a longer readiness deadline for the first child, after which it is killed and the watcher respawns it. Make the replacement deadline grow per attempt or raise it. Log a separate error when the child exits early. Add an isolated test for a slow start that then becomes ready.</next_steps>
</results>
# Lane r4b-daemon-slow-start-v2

**(a) Both r4a P1s are resolved.**
- **P1 #1 (a first child that never gets ready is never killed):** fixed. After the 30s wait in `ensure`, the code at `omo_daemon.rs:568-602` starts a task that keeps probing for `READY_WAIT_MAX - READY_WAIT_AFTER_SPAWN` (150s) more. If the child is still alive and unready at the end, it calls `start_kill()`. The watcher then sees the exit and respawns. Total budget is about 180s.
- **P1 #2 (replacements killed after a flat 30s):** fixed. The watcher's deadline is `ready_wait(failures.saturating_sub(1))`, which gives 30, 60, 90, 120, 150, then 180s for every later attempt. Under sustained load, the attempts grow until one fits.

**(b) Race closed.**
- `first_pid` is read with `try_lock` before the watcher exists. The only other `try_lock` user, `child_exited`, has already dropped its guard, so `first_pid` is `Some(pid)` for the live first child.
- The poll loop returns early if any of these happen:
  - the slot is empty;
  - `id()` differs from `first_pid` (a swapped replacement);
  - `id()` is `None`, because tokio clears the id once anyone reaps the child, including the watcher's `try_wait` and `force_restart`'s `wait`;
  - the child has exited.
- The final kill is one critical section: `if let Some(child) = slot.lock().await.as_mut() { if child.id() == first_pid { start_kill } }`. Under edition 2021 the guard lives through the whole `if let` body. The watcher swaps children under the same mutex, so the check and the kill cannot interleave with a replacement.
- The only remaining path is PID reuse within a single 500ms window right after the first child was reaped. That is negligible.

**(c) The arithmetic is correct.**
- `failures` is incremented (`:681`) before the backoff sleep and the spawn. The first replacement therefore sees `failures=1`, which maps to `ready_wait(0)` = 30s.
- The defer-to-external path resets `failures` to 0 and continues, so the next cycle again starts at 1.
- `healthy_since` is only set again on ready, so an unready kill or a self-exit does not reset `failures`. That is correct.
- The test `daemon_unready_replacement_is_killed_and_retried` advances by `ready_wait(0) + 3s`, which matches the first replacement's 30s.

**(d) It should compile (judged by reading only):**
- `Option<&mut Child>::is_none_or` is stable since 1.82, and the project's rust-version is 1.88.
- The closure gets `&mut Child`, which is enough for `id()` and `try_wait()`.
- tokio `Instant` has `Add<Duration>` and `Sub<Duration>`, and `now + 180s - 30s` cannot underflow.
- `first_pid` is a `Copy` `Option<u32>`. `slot` and `shutdown` are `Arc` clones and `url` is cloned, so `supervisor.spawn_watcher(&self)` still works afterwards.
- `g.as_ref().and_then(Child::id)` auto-derefs the guard; the tests already use the same pattern. The guard temporary in `let exited = ...` is dropped at the end of the statement.

**(e) `force_restart_owned_daemon` timeout is harmless:** see the P2 below.

### [P2] force_restart gives up before a loaded host's replacement can become ready
- **Location:** `src/agent/omo_daemon.rs:796` (`READY_WAIT_AFTER_SPAWN + RESTART_BACKOFF_MAX` = 90s), called from `src/agent/omo_backend.rs:2034`.
- **Trigger:** A forced restart while the host is heavily loaded. The daemon was healthy before, so `failures` resets to 0. The first replacement gets 30s and is then killed by the watcher. The second starts at about 36s and gets up to 60s, which runs past the 90s deadline.
- **Impact:** The stalled turn is not retried and surfaces its original error. The watcher still brings the daemon up later. `LAST_FORCED_RESTART` throttling (120s) keeps this from becoming a loop. One turn fails; there is no crash and no lasting outage.
- **Evidence:** The deadline at `:796` vs the watcher's `ready_wait` growth at `:712`.
- **Fix (optional):** Use `READY_WAIT_MAX + RESTART_BACKOFF_MAX` as the deadline, or accept the behaviour.

### [P2] A hung first child resets the replacement ramp to 30s
- **Location:** `:661` (`healthy_since = Some(Instant::now())` when the watcher starts) together with the reset at `:675-680`.
- **Trigger:** The slow-start task kills the first child about 150s after the watcher started. `healthy_since.elapsed() >= RESTART_WINDOW` is true, so `failures` drops back to 0.
- **Impact:** The first replacement after an unready first child gets 30s, even though it has just been shown that more than 180s was not enough. The ramp still grows to 180s, so this costs a few minutes of extra cycles rather than looping forever.
- **Evidence:** The watcher treats "started" as "healthy" even on the slow-start path.
- **Fix (optional):** When `ensure` falls into the slow-start path, start the watcher with `healthy_since = None`.

### [P2] No test covers the slow-start kill task or the `first_pid` guard
- **Location:** The `ensure` slow-start branch, `:557-602`. Of the two new or updated tests, `readiness_wait_grows_per_unready_restart_and_is_capped` only checks the arithmetic, and `daemon_unready_replacement_is_killed_and_retried` only exercises the watcher.
- **Trigger:** Any future regression in this branch, for example `first_pid` ending up `None`. Then `id() != first_pid` is true on the first poll, the task returns at once, and P1 #1 silently comes back. That case is unreachable today.
- **Impact:** The P1 #1 fix and the race guard rely on review alone.
- **Fix:** Add a paused-time test: an owned child that serves 503 is killed about 180s after `ensure` returns, and a child swapped into the slot before the deadline is not killed.

**(f)** I found no other P0 or P1. `ensure` still returns `Err` when spawning fails (ENOENT) or the token file cannot be written. That is older behaviour unrelated to host load.

## Verdict
0 P0 / 0 P1 (3 P2); fix VERIFIED.
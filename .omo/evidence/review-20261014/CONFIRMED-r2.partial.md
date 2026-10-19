# CONFIRMED-r2 (lead falsification of R2 delta review)

## r2d-discord-stream (st_01a13ae9, claude-opus-5-5)
- NO FINDINGS. Fix verification: VERIFIED (F5). Lead check:
  - the key (identity, channel_id) is the same as in the mark_dead_for_bot callers;
  - throttler.update with is_final=true returns Err on any send/edit failure before the clear;
  - no await is held under the clear's lock;
  - clear_for_bot spawns its DB write only on a real removal.
- Note: stream never marks a target dead on 403/404. This was already true before this diff and stays a follow-up.

## r2a-backend-daemon (st_01a13ae6, claude-opus-5-5)
- Fix verification: F1 VERIFIED; F6 VERIFIED when the gateway starts first.
- [P1 CONFIRMED, regression from F6] When the dashboard starts first, its daemon's pidfile names a live owner, so the gateway's ensure() returned Ok(None) with no supervisor. When the dashboard exits (kill_on_drop) the gateway has no daemon and cannot restart it.
  FIX APPLIED: ensure() detects a daemon owned by another live omon process (daemon_owned_by_other_live_process) and returns a supervisor with an empty child slot plus a watcher, registered in OWNED_DAEMONS. The empty slot reads as exited, so the watcher waits while the foreign daemon serves, then spawns its own. This is the same shape as the daemon_external_takeover_is_untouched test.
- P2 recorded:
  - an open tool id that never completes disables the 180s watchdog until the total deadline (accepted trade-off: bounded, and no resubmit);
  - one missed readyz probe makes the watcher spawn a doomed child that overwrites the pidfile;
  - a reused owner pid makes a real leftover unreclaimable;
  - no frame-level / takeover-respawn tests.

## r2c-cron-lanes-sec (st_01a13ae8, claude-opus-5-5)
- Fix verification: F2 VERIFIED; F3 VERIFIED except stepped ranges ending in 7 and `7-7`; F4 VERIFIED.
- 0 P0/P1. P2s in my own new code, fixed anyway because they are cheap:
  - unix_day_of_week_to_crate now expands numeric items to the Unix day set and emits an explicit crate list. This fixes `1-7/2`, `5-7/2` and `7-7`.
  - settle.rs listing uses `--no-relative`, so a user `diff.relative=true` cannot reintroduce the bug.
  - The toplevel read trims only newlines.
  - Test assertions updated: 5-7 -> 1,6,7; 0-6/2 -> 1,3,5,7; 1-7/2 -> 1,2,4,6; 7-7 -> 1; 1/2 -> 2,4,6; 1,FRI -> 2,FRI; */2 unchanged.
- P2 recorded (pre-existing): hardline misses `/etc/..` and `/usr/../`, which resolve to root.

## r2b-runtime-mux (st_01a13ae7, claude-opus-5-5)
- Fix verification: F8 (headless supervisor) VERIFIED; F7 (cursor rewind) VERIFIED for single messages.
- [P0->regraded P1 CONFIRMED, gap in F7] A debounced batch carries only the last constituent's platform_message_id (coalesce, adapter.rs:607), so the rewind moved the cursor only to last-1 and earlier batch messages stayed unrecoverable on graceful restart. Graded P1: restart-only trigger, the same severity as the original finding.
  FIX APPLIED: rewind_backfill_cursor takes MIN(CAST(substr(constituent_id,9) AS INTEGER)) from delivery_ledger_constituents for the event's delivery_id (ids are 'discord:<snowflake>', and record_incoming_with_constituents writes them), falling back to the event's own id.
- P2 recorded:
  - a live cursor advance after route() can race a shutdown rewind (narrow window);
  - the rewind can replay other sessions' failed rows that share the cursor key.

## r2e-daemon-takeover (st_01a13afa, claude-opus-5-5)
- 0 P0/P1. Fix verification: F6b (dashboard-first takeover) VERIFIED. Empty-slot kill/Drop/is_available are safe, there is no endless mutual deferral or port fight, and no compile issue is visible.
- P2 recorded:
  - the pidfile is written before the respawned child proves it bound (a later restart can lose takeover);
  - force_restart stamps the 120s throttle while deferring;
  - a misleading 'daemon is down' WARN is logged on takeover start.

## r2f-misc-fixes (st_01a13afb, claude-opus-5-5)
- NO FINDINGS (0 P0/P1/P2). Verified by reading only, no compile:
  - F9 (proposal packing): VERIFIED
  - F10 (/thread PublicThread): VERIFIED
  - F7b (rewind_backfill_cursor batch MIN): VERIFIED
  - F11 (lane activity reset + interval_at): VERIFIED
  - F3 rewrite (unix weekday conversion): VERIFIED
  - F2 tweaks (--no-relative/toplevel): VERIFIED

## r2g-deploy-fixes (st_01a13b0b, claude-opus-5-5)
- 0 P0/P1. F12 VERIFIED: autospawn is off, ensure() returns Ok(None) on all 6 call sites, and env overrides env_file. F13 VERIFIED: atomic rename, signature kept.
- P2 fixed anyway:
  - the empty compose token hid the OMON_OMO_AUTH_TOKEN alias (now chained);
  - a rollback cp failure under set -e fell through to restart (now `{ cp && mv; } || die`).
- P2 recorded: Linux host-gateway reachability and a .env loopback-URL pitfall (docs).

## r2h-lane-retry (st_01a13b22, claude-opus-5-5)
- 0 P0/P1. F14 VERIFIED:
  - the julianday() RFC3339 comparison is correct;
  - single caller (60s tick), so the retry runs about every 10 min;
  - the test fails without the fix, passes with it, and does not interfere with other tests.
- P2 fixed anyway: the stale test comment.
- P2 recorded: a permanent failure retries and notifies every 10 min with no cap (bounded, a deliberate trade-off).

## r2i-session-delete (st_01a13b28, claude-opus-5-5)
- 0 P0/P1. F15 VERIFIED:
  - the only production session-delete path is guarded;
  - the code compiles by reading;
  - the test is valid (axum {id} accepts 'web:bound');
  - no regressions.
- P2 fixed anyway: the count-then-delete race; the DELETE now carries AND NOT EXISTS(cron_jobs) and stays atomic.
- P2 recorded: the web UI shows a generic delete-failed toast instead of the 409 reason (UX only; no web change, to avoid a web rebuild).

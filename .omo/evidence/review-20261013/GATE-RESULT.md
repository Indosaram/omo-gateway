# Cycle 2 integration gate (2026-10-13)

Run 2 (final): fmt 0, clippy --all-targets --all-features -D warnings 0, cargo test --all-features 0 (78 targets, 1536 passed, 0 failed, 20 ignored), web bun test 0, web tsc -b 0, web vite build 0.
Also: web/src/lib/remote-image.test.ts run separately with bun test: 3 pass, 0 fail.

Run 1: test step failed with 22 tests in 8 groups, all caused by this round's changes or their fixtures, and fixed:
- adopt lead validation: tests passed "owner" as the lead; now use DEFAULT_LEAD.
- backfill: route_claimed_event_awaiting_turn still advanced the cursor before the turn; removed.
- /stop: defer_ephemeral added this round broke the registered-command fixtures; reverted (fast command).
- lanes claim/settlement: round change scrubbed local evidence, contradicting existing tests; restored the HEAD contract (local evidence exact, event body scrubbed and capped).
- readiness: the backend now gates when a chat runtime exists or a backend URL is configured explicitly.
- missing fixture rows (sessions row for a cron FK, cron_problems row for needs_owner).
- daemon fixture rebind: SO_REUSEADDR.
Logs: gate/run1, gate/run2.

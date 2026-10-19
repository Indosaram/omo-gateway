# lanes review

### [P2] `apply_reopen` opens a deferred transaction and reads before it writes
- Location: /Users/indo/code/project/omon-gateway/src/lanes/runner.rs:72 (SELECT at 73, first write at 84)
- Trigger: the service's `apply_owner_replies` (runner.rs:905) calls `apply_reopen`, or the CLI calls `reopen` (runner.rs:47). Another process (the CLI or the service) commits any write between the SELECT and the `UPDATE lanes ...` at 84. SQLite returns SQLITE_BUSY_SNAPSHOT immediately and `busy_timeout` does not retry it. The comment at /Users/indo/code/project/omon-gateway/src/storage/lanes.rs:445-446 describes this same failure and fixes it with `BEGIN IMMEDIATE` in `transition_lane` (storage/lanes.rs:447). `apply_owner_replies` already uses `BEGIN IMMEDIATE`.
- Impact: the reopen fails with a database error. For an owner reply, the reply stays parked with an error set until the owner sends another message. The window is microseconds and needs a cross-process writer, so this is not P1.
- Fix: use `pool.begin_with("BEGIN IMMEDIATE")` at runner.rs:72.

Confirmed correct this round (no defects found):
- Reopen generation and receipt fencing in `apply_reopen` (runner.rs:84-100) and `record_owner_reply` (storage/lanes.rs:235-255): the generation is derived from the pending minimum or the latest `reopened` history id, and the reply is applied only if still pending at that generation.
- `request_close` upsert (storage/lanes.rs:202-208): it resets only a `failed` intent. Pending and running intents are idempotent, and the UNIQUE(lane_key, generation) constraint in migration 0038 backs this.
- `bind_lane_thread` (storage/lanes.rs:215-219): it rejects a bot mismatch, and adapter.rs:1862-1876 falls through to normal routing on that error.
- `capture_lane_start` (usage.rs:167-190): it uses `ON CONFLICT DO NOTHING`, a 5s timeout and `kill_on_drop`.
- `lane_diff_input` (settle.rs:256-300): it builds its index in a private temp index file, so the lane's real index is untouched.

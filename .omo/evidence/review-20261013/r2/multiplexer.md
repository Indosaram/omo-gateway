# multiplexer review

I checked each round-2 fix against the code. I found no P0 or P1. The fixes hold, and I found three P2 issues in the changed code and one more in nearby code.

### [P2] Interrupt rollback also deletes out-of-band mirror rows
- Location: /Users/indo/code/project/omon-gateway/src/multiplexer/actor.rs:1025
- Trigger: A cron or system delivery is mirrored into a session with `mirror_to_session`, which writes `role='assistant'` with `mirror:true` and no `delivery_obligation_id`. This happens while that session's turn is running. The turn is then stopped, reset, or interrupted by shutdown.
- Impact: The rollback now deletes only assistant rows after the turn row, but that still includes the mirror rows. The cron delivery record disappears from the transcript, and so does any `mirror_injected` bookkeeping that depends on it.
- Fix: Add `AND COALESCE(json_extract(metadata_json,'$.mirror'),0) <> 1` to the DELETE. Rows with `direct_reply` are already protected by the obligation check.

### [P2] Kibitzer arming is not atomic with the insert and fires on rows that are not new
- Location: /Users/indo/code/project/omon-gateway/src/multiplexer/actor.rs:1087
- Trigger 1: `persist_inbound` inserts the transcript row on the pool, then arms in a separate transaction. A crash between the two leaves the owner message unarmed.
- Trigger 2: Restart recovery re-dispatches the same event id, so the insert hits `ON CONFLICT DO NOTHING`. `arm_kibitzer` still runs and bumps the generation for an old message.
- Trigger 3: The instant-steer path writes the owner's lead-DM message through `record_inbound_transcript` and never arms.
- Trigger 4: `is_kibitzer_lead` compares the raw `OMON_LANE_DEFAULT_LEAD` string to `storage_key()`. A non-canonical value never matches, and nothing is logged.
- Impact: The kibitzer misses some owner messages and fires spuriously on others.
- Fix: Arm only when the insert reports `rows_affected()==1`, inside the same transaction as the insert. Call the same arming from `record_inbound_transcript`. Parse the lead with `SessionKey::from_storage_key` and compare keys instead of strings.

### [P2] Eviction can mark an idle session `resume_pending`
- Location: /Users/indo/code/project/omon-gateway/src/multiplexer/actor.rs:848
- Trigger: A `TouchActivity` or `IsIdle` command is sent through `handle.sender.try_send`, which bypasses the `accepting` gate. It lands in the mailbox after `EvictIfIdle` and is drained in the post-loop. `pending_events` is non-empty, so the session gets `mark_session_resume_pending`.
- Impact: After the next restart, a session whose last row is a user row from a silent turn is re-run, which repeats the turn.
- Fix: Call `mark_session_resume_pending` only if `pending_events` holds an `Event`, `EventWithAck` or `EventIfIdle`.

### [P2] `SessionActor::recover_resume_pending_sessions` diverges from the `main.rs` recovery
- Location: /Users/indo/code/project/omon-gateway/src/multiplexer/actor.rs:144
- Trigger: A caller uses the library version. Only a test uses it today, because `main.rs` has its own copy.
- Impact: The library version hard-codes `EventOrigin::User` and re-attaches attachments. A `[system] ` row is therefore replayed as a user turn. The user turn also arms the kibitzer, and attachment blocks are rendered twice. `main.rs::recovery_event` fixes both.
- Fix: Delete the library copy, or have it call the shared `recovery_event`.

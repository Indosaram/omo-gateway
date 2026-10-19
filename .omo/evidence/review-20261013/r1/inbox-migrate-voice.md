# inbox-migrate-voice review
### [P1] Mail follow-up replies in an already-collected thread are silently dropped as duplicates
- Location: /Users/indo/code/project/omon-gateway/src/inbox/sources/mail.rs:47 (item keyed by thread_id); /Users/indo/code/project/omon-gateway/src/inbox/mod.rs:222 (`ON CONFLICT(source, source_ref) DO NOTHING`)
- Trigger: A mail thread is collected and triaged. A second unread message then arrives in the same Gmail thread, and `zele mail list` returns the thread again with a newer date. `source_ref` is the thread id, so the insert conflicts and counts as a duplicate. The new snippet is never stored, never triaged, and never reaches the digest.
- Impact: Follow-ups such as "please sign by today" in an existing thread are lost. The thread keeps its old class, for example fyi or ignore.
- Fix: Key mail items by message id, or by thread id plus the latest message id or internal_date. Alternatively, on conflict with a changed `raw_hash`, update the snippet and delete the old decision so it is re-triaged.

### [P1] Work heartbeat step aborts on the first sink or DB error and starves every other stale item
- Location: /Users/indo/code/project/omon-gateway/src/work_heartbeat.rs:325
- Trigger: One in_progress work item has a deleted or archived thread, a non-numeric `thread_id`, or a title long enough to push the message past 2000 chars. `post_message(...)` returns Err and `?` ends `step()`. Items are ordered by `last_progress_at ASC`, and the failing item's progress is never updated, so it sorts first on every tick.
- Impact: Every later stale item gets no heartbeat or dead-worker alert on any tick until the bad row is fixed by hand. The error is only logged by `spawn`.
- Fix: Handle each item's post and update in its own `match`. Log the error and `continue` so one bad channel cannot block the rest. Truncate `item.title` before building the message.

### [P1] Lookup and draft results are sent to the owner as one Discord message with no length handling, so large results are never delivered
- Location: /Users/indo/code/project/omon-gateway/src/inbox/proposals.rs:136 (64 KiB lookup cap, returned as `owner_text`); consumer at /Users/indo/code/project/omon-gateway/src/discord/adapter.rs:877 (`send_message(...content(text))`)
- Trigger: The owner approves, or auto-lookup runs, a `Lookup` proposal. For mail and Discord items the link is a mail.google.com or discord.com page, so the fetched HTML is routinely over 2000 chars. The LLM draft route can also exceed 2000 chars.
- Impact: The proposal is already committed as `completed` and there is no auto-retry. The send fails, the handler returns Err, and the owner sees nothing. The result survives only in `inbox_decisions.outcome` JSON.
- Fix: Chunk or truncate `owner_text` before sending (the adapter already has `bound_split_messages`). Alternatively, cap the lookup body at about 1800 chars and reduce HTML to text.

### [P2] One malformed record wedges a whole inbox source permanently
- Location: /Users/indo/code/project/omon-gateway/src/inbox/sources/imessage.rs:98; /Users/indo/code/project/omon-gateway/src/inbox/sources/kakao.rs:89; /Users/indo/code/project/omon-gateway/src/inbox/sources/discord.rs (timestamp parse in `fetch_since`, about line 85); /Users/indo/code/project/omon-gateway/src/inbox/sources/mail.rs:47
- Trigger: A single message lacks a text field (for example an attachment-only iMessage or Kakao message with a null or absent text), or has an unparsable date. `parse` returns Err for the whole batch. The cursor is never advanced, so the same record is hit on every collection.
- Impact: No new items from that source until the upstream record ages out of the window. The error is only recorded, and the digest only surfaces auth-like errors.
- Fix: Skip and count the bad record (or record it as a drop reason) and advance the cursor past it. Fail the batch only for transport or JSON-shape errors.

### [P2] Kakao first run pages whole room history, and one failure loses all progress
- Location: /Users/indo/code/project/omon-gateway/src/inbox/sources/kakao.rs:160-210
- Trigger: There is no cursor and `OMON_INBOX_KAKAO_ROOMS` is unset. Every DirectChat and MultiChat room is paged back to the start of history, with no page or item cap. The 60s `collect_all` timeout, or any HTTP error in one room, aborts the source before the cursor map is saved.
- Impact: A large archive can time out every 120s tick, re-fetching the same history without ever committing a cursor.
- Fix: Bound the initial backfill (a max-pages or max-age floor when the cursor is empty). Persist a per-room cursor map incrementally, or return partial progress on error.

### [P2] Schema DDL and ALTER run at runtime on every `Proposals::new` and race on first upgrade
- Location: /Users/indo/code/project/omon-gateway/src/inbox/proposals.rs:285-307
- Trigger: On an upgraded DB without `inbox_decisions.outcome`, two callers run `PRAGMA table_info` and both see the column missing (for example `inbox list` or `inbox set` while the gateway starts, or several DM handlers). The second `ALTER TABLE ... ADD COLUMN` fails with "duplicate column name".
- Impact: A spurious startup, CLI or approval failure. `Proposals::new` also runs DDL on every owner DM and every serve loop iteration.
- Fix: Move `inbox_proposals` and the `outcome` column into a numbered migration under `migrations/`, and drop the runtime DDL.

### [P2] The owner approves a proposal without seeing the execution route (kind, cwd, done)
- Location: /Users/indo/code/project/omon-gateway/src/inbox/proposals.rs:63 (`display`), 21-30 (`run_proposal` accepts arbitrary `--action-json`)
- Trigger: `omon inbox-propose` is invoked, for example by the lead agent after reading injected inbox content, with `{"kind":"repo","cwd":"/some/path","done":...}`. The Discord approval text shows only `title -> proposed_action` and the link.
- Impact: The owner's "N 해" approves a lane start with a cwd and done-criterion they never saw. This is a weakened approval boundary, not a bypass.
- Fix: Include `kind`, `cwd` and `done` in `Proposal::display()`. Optionally restrict `run_proposal` to the trusted configured route.

### [P2] Auto-lookup runs inline in the triage loop and discards its result
- Location: /Users/indo/code/project/omon-gateway/src/inbox/triage.rs:406-424
- Trigger: `OMON_INBOX_AUTO_LOOKUP=true` and a pending Lookup proposal. Each lookup, with a 30s HTTP timeout, runs sequentially in the same `serve` loop that handles events and deadlines. The `ApprovalResult::Executed{owner_text}` is dropped. One failing proposal (`?`) skips the rest of the loop until the next iteration.
- Impact: The owner never sees the lookup output. Collection events and the triage deadline stall for the duration of the fetches, and `progress_rx` can fill and drop events.
- Fix: Spawn auto-lookups separately and deliver the text to the owner. Handle errors per proposal.

### [P2] `inbox set --status approved` reports success when nothing executed
- Location: /Users/indo/code/project/omon-gateway/src/inbox/set.rs:60-62
- Trigger: The proposal is already running, completed or failed, or its decision was dismissed or changed. `approve_item` returns `AlreadyClaimed`, and the CLI treats it as `Ok(())` with exit code 0 and empty output.
- Impact: An operator believes the approval worked.
- Fix: Map `AlreadyClaimed` to an explicit error or message, for example "already claimed or changed".

### [P2] The config importer deletes the target .env's existing Discord tokens when Hermes provides none
- Location: /Users/indo/code/project/omon-gateway/src/migrate/config_import.rs:519-531 and 552
- Trigger: `omon migrate` against a Hermes home whose root config has `discord.enabled: false` or no token, while the target `.env` already holds `DISCORD_BOT_TOKEN` or `DISCORD_BOT_TOKENS`. Both are removed from the merged values and from the rendered file, with only a timestamped backup kept.
- Impact: The gateway fails to start after migration. This looks deliberate mirroring, but it destroys credentials that Hermes does not own.
- Fix: Only overwrite tokens when Hermes supplies one. Otherwise keep the existing entries, or refuse with a clear error.

### [P2] A store rewritten between snapshot and rewrite leaves cutover permanently pending
- Location: /Users/indo/code/project/omon-gateway/src/migrate/cron_cutover.rs:535 (compare `current != store.original_bytes`); recovery at 138-148
- Trigger: After the receipt is committed and jobs are set to `cutover_pending`, a store changes on disk before its atomic replace. This needs Hermes to restart, for example via a launchd respawn, after gateway-down. The call returns Err, jobs stay `cutover_pending` (the scheduler excludes them), and the next `reconcile_pending_cutover` fails with "unknown content".
- Impact: Those cron jobs stop running until the DB is repaired by hand.
- Fix: On the mismatch, roll the receipt back to `failed` and restore the authority of the affected jobs. Alternatively, let reconcile treat an unknown-content store as "operator must choose".

### [P2] The digest owner check is vacuous
- Location: /Users/indo/code/project/omon-gateway/src/inbox/digest.rs:130 (`spec(&home, &home.user_id)`)
- Trigger: The cron tool's `delete` followed by `add` with id `omon:inbox-digest`, issued from a non-owner Discord DM session. Both `delete` and `add` go through the tool, and `register_with_id` accepts the id. The executor only checks that the payload matches the session key's own user.
- Impact: The owner's inbox digest (mail, Kakao, iMessage summaries) is delivered to that other DM.
- Fix: In `DigestExecutor::execute`, compare `home.user_id` with the configured owner (`OWNER_ID`). Optionally reserve the `omon:` id prefix in the cron tool.

### [P2] Digest and usage drop counts use UTC dates against KST windows
- Location: /Users/indo/code/project/omon-gateway/src/inbox/prefilter.rs:184 (writes UTC `day`); /Users/indo/code/project/omon-gateway/src/usage.rs:58 and :232 (reads by UTC date range or the KST previous date)
- Trigger: Any drop between 00:00 and 09:00 KST is stored under the previous UTC date. `yesterday()` compares that date with the KST previous day. `usage --since 1h` within one UTC day sums `day>=D AND day<D`, which is always 0.
- Impact: `inbox_collected` is misreported in the digest line and the usage JSON.
- Fix: Store the KST date, or store a timestamp and filter by the same window as `inbox_items`.

### [P2] Triage re-judges and drops progress under failure or lag
- Location: /Users/indo/code/project/omon-gateway/src/inbox/triage.rs:233-251 (judge calls before any write, `?` aborts the batch); /Users/indo/code/project/omon-gateway/src/inbox/watchers.rs (about line 139, `let _ = tx.try_send(results)`)
- Trigger: One `choice()` error, a fallback or capped result, or a writer failure makes the whole 20-item batch retry. Owner and bot items that failed at the writer are re-judged on each retry, spending the daily jev cap. Separately, if triage is busy for minutes and the 16-slot progress channel fills, a collection event is dropped.
- Impact: Wasted judge calls. A dropped event means sub-20 batches wait until the next insert event or a restart.
- Fix: Persist per-item judge results as they succeed, or cache them within the batch. Re-arm the deadline from the DB when events are dropped.

### [P2] Gateway-down grace is 1 second before SIGKILL
- Location: /Users/indo/code/project/omon-gateway/src/migrate/gateway_down.rs:8-9 (10 polls × 100 ms)
- Trigger: A Hermes gateway needs more than 1s to shut down gracefully, for example with a cron run in flight.
- Impact: It is SIGKILLed mid-shutdown just before the cron store is snapshotted and emptied.
- Fix: Allow a longer SIGTERM grace, about 10-30s, before SIGKILL.

### [P2] STT transcribes only the first audio frame
- Location: /Users/indo/code/project/omon-gateway/src/voice/pipeline.rs:156
- Trigger: `SpeechToText::transcribe` or `SpeechPipeline::process` is called with several frames, for example VoiceTick frames or buffered audio.
- Impact: Everything after `frames.first()` is silently ignored. Current callers pass a single whole-file frame, so this is latent.
- Fix: Concatenate PCM frames, or reject multi-frame input with an error.

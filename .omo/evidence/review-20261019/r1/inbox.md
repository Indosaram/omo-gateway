# Lane inbox

The diff has one P1 defect: a Slack message that is indexed late can be skipped by the Slack inbox and never collected. I also found four P2 issues. I read the current working tree only and did not build or run anything.

### [P1] Slack inbox cursor skips any message indexed after a newer message was collected
- Location: src/inbox/sources/slack.rs:61
- Trigger: The Slack source reads from the search index across all channels. It keeps one high-water cursor equal to the largest message timestamp it has seen, and it only reads rows with a larger timestamp. Indexing happens when the message's turn is persisted, not in timestamp order. Two production paths make this reachable:
  - **Missed socket events.** Slack Socket Mode can drop events (incident 2026-10-19 in `src/slack/runtime.rs`). The reconcile pass recovers them only after waiting 120 s plus a 60 s settle (`RECONCILE_INTERVAL` / `RECONCILE_SETTLE`). Example: owner-DM message A at T1 is dropped. A live mention B at T2 > T1 gets indexed, which fires `on_gateway_message`, and the inbox collects B and sets cursor = T2. About two minutes later the backfill routes A. `persist_inbound` indexes A (`actor.rs:1355-1362`), but `d.message_id > 'T2'` excludes A.
  - **Debounce/actor ordering.** A DM batch whose 600 ms window is extended flushes after a later message in another session.
- Impact: The skipped message is never collected, triaged or proposed, so a DM or mention silently never reaches the owner's inbox or digest. Nothing records an error.
- Evidence:
  ```
  AND d.message_id > ? \
  ...
  ORDER BY d.message_id",
  ...
  next = next.max(ts);
  ```
- Fix: Stop using the message timestamp as the cursor. Key the cursor on an insertion-ordered column, for example `d.rowid > ?` with `next = max(rowid)`, since `upsert` keeps the rowid on conflict. Alternatively, keep a lookback window (for example `d.message_id > cursor - 1h`) and rely on `ON CONFLICT(source, source_ref) DO NOTHING` to dedupe.

### [P2] Channel messages read through Slack message_context from a DM session are indexed with `guild_id = NULL` and classified as `dm`
- Location: src/tools/message_context_slack.rs:593
- Trigger: The owner, in the Slack DM, asks the agent to read a cross-channel target listed in `SLACK_ALLOWED_CHANNELS`. `document()` copies the DM session's `guild_id` (None) onto every channel message. The new Slack source then classifies these rows as DMs (`src/inbox/sources/slack.rs:46`), and they pass the prefilter's `dm` branch.
- Impact: Every non-bot message in that channel batch becomes an inbox item. Recent ones cost a jev classification each and can surface as owner tasks. This only happens when cross-channel reads are configured.
- Evidence: `guild_id: session.guild_id.clone(),` and `WHEN d.guild_id IS NULL OR d.guild_id = '' THEN 'dm'`
- Fix: In the Slack source, classify `dm` by the real channel, using `json_extract(metadata_json,'$.channel_id')` / `real_channel` starting with `D`. Or have `document()` set `guild_id` from the conversation instead of the session.

### [P2] GitHub CI-failure items are filed as fyi when the PR's updatedAt is more than 24h old
- Location: src/inbox/triage.rs:295
- Trigger: The GitHub source sets `received_at` from the PR's `updatedAt` (`src/inbox/sources/github.rs:333`). Its own comment (line 318) says `updatedAt` does not move when checks finish. A CI re-run or a scheduled workflow failing on a PR untouched for more than 24h therefore looks like stale backlog.
- Impact: A "[CI failed]" change is stored as fyi without a model call and never appears as an owner or bot task.
- Evidence: `if now - item.received_at > chrono::Duration::hours(STALE_AFTER_HOURS) { classified.push((item, Class::Fyi, None));` and `received_at(&current.updated_at),`
- Fix: Use the poll time (`Utc::now()`) as `received_at` for PR change items.

### [P2] Slack prefilter checks the owner mention in the truncated snippet while the SQL checks full content
- Location: src/inbox/prefilter.rs:180
- Trigger: A channel message longer than 1000 characters that mentions the owner after character 1000. The SQL (`instr(d.content, ?)`) selects it, but the snippet is capped at `MAX_SNIPPET_CHARS`.
- Impact: The message is dropped as `slack_not_owner_mention_or_dm`.
- Evidence: `let owner_mention = item.snippet.contains(&format!("<@{}>", self.owner_slack_id));`
- Fix: Have the source set `room_type = "mention"` when the SQL matched on a mention, and accept that room type in the prefilter.

### [P2] `inbox collect --source github` polls GitHub twice and prints the first poll's changes
- Location: src/inbox/cli.rs:116
- Trigger: Running it without `--dry-run` calls `source.poll()` for display, then `collect_all` polls again (lines 122-127) and stores that second result.
- Impact: It doubles the `gh` API calls. If state changes between the two polls, the printed `changes` can differ from what was stored.
- Evidence: `let poll = source.poll(cursor.as_deref())` … `let results = super::collect_all(pool, &[source as &dyn InboxSource], ...)`
- Fix: When not a dry run, commit the already-fetched `poll.items` / `poll.cursor`, for example by exposing `commit_source`, instead of calling `collect_all`.
# Task 22 Integration Fix Receipt: Event Collector & Prefilter Integration

**Date:** 2026-10-11  
**Worktree:** `/Users/indo/code/project/omon-gateway-wt/dori-work`  
**Status:** Concrete Implementation Complete; Runtime Gates Pending Coordinator Batch  
**Owned Files Touched:**
- `src/inbox/mod.rs` (sources module export verified)
- `src/inbox/watchers.rs` (concrete event-aware callback, bounded channels, strict cursor timestamp vs watcher rowid decoupling)
- `src/inbox/prefilter.rs` (corrected Discord prefilter: fixed `thread_ref.is_none()` wrongDM bug, added `room_type` eligibility for `dm` / `owner_reply`, strict author exclusions for owner/bot)
- `tests/test_inbox_store.rs` (pure prefilter unit tests, full rules/mentions/author excludes, event dispatch tests without sleeps, cursor consistency tests)

---

## 1. Summary of Gaps Resolved

### A. Event-Aware Watcher Callback (`src/inbox/watchers.rs`)
- **Previous state:** `src/main.rs` used a 5-minute timer polling loop (`tokio::time::sleep(300s)`), violating the event-first, push-first architecture.
- **Implemented solution:**
  - `InboxWatcherEvent`: `GatewayMessage`, `IMessage { rowid: Option<i64> }`, `Mail`, `Periodic`.
  - Bounded mpsc event channel (`DEFAULT_EVENT_CHANNEL_CAPACITY = 64`) via `watchers::event_channel(capacity)`.
  - Concrete event triggers:
    - `on_gateway_message(&sender).await` / `try_on_gateway_message(&sender)` (for Discord/inbound gateway message insertion).
    - `on_imessage_event(&sender, rowid).await` (for `imsg watch` child events).
    - `on_mail_event(&sender).await` (for `zele mail watch` hits).
    - `on_periodic(&sender).await` (for Kakao 2-minute model-free cursor check).
  - `filter_sources_for_event`: Routes events directly to the affected source (e.g. `GatewayMessage` -> `InboxSourceKind::Discord`, `IMessage` -> `InboxSourceKind::Imessage`, `Mail` -> `InboxSourceKind::Mail`, `Periodic` -> `InboxSourceKind::Kakao`), falling back to all sources if unspecialized.
  - `process_event`: Single-event collection execution without timer polling.
  - `run_event_watch` & `run_event_watch_with_shutdown`: Asynchronous event loop with bounded channel progress reporting (`progress_tx`) and graceful shutdown integration via `watch::Receiver<bool>`.

### B. Cursor Source Timestamp vs Watcher RowID Consistency (`src/inbox/watchers.rs`)
- **Risk identified:** `IMessageSource` uses an RFC3339 timestamp string (`"2026-10-11T08:14:10.830Z"`) in `inbox_cursors.cursor`. Meanwhile, `imsg watch` uses a numeric integer rowid (`--since-rowid <rowid>`). Silently mixing them would either corrupt `inbox_cursors` or break `imsg watch`.
- **Implemented solution:**
  - `IMessageCursorState`: Strongly typed struct documenting and distinguishing `source_timestamp: Option<String>` vs `watcher_rowid: Option<i64>`.
  - `read_imessage_source_cursor(&pool)`: Explicitly reads the RFC3339 timestamp from `inbox_cursors`.
  - `latest_imessage_rowid(&pool)`: Queries `SELECT MAX(CAST(source_ref AS INTEGER)) FROM inbox_items WHERE source = 'imessage' AND source_ref GLOB '[0-9]*'` to find the latest numeric rowid without touching `inbox_cursors`.
  - `imsg_watch_args(since_rowid: Option<i64>)`: Accepts strongly typed `Option<i64>`, ensuring non-numeric timestamp strings can never be passed to `--since-rowid`.
  - `parse_imsg_watch_line(line: &str) -> Option<i64>`: Parses JSON stdout lines from `imsg watch` to extract numeric rowids.

### C. Discord Prefilter Eligibility & Author Exclusions (`src/inbox/prefilter.rs`)
- **Previous state:**
  1. `item.thread_ref.is_none()` was used to detect DMs. In Discord, standard guild channel messages have `thread_ref: None`, causing regular channel chatter from others to be falsely treated as DMs (`wrongDM` bug).
  2. Owner replies without explicit `<@owner_id>` mentions were dropped as `discord_not_owner_mention_or_dm` if they were inside threads.
  3. Owner self-mentions or bot mentions were not dropped if `<@owner_id>` was present in the text.
- **Implemented solution:**
  - **Author exclusion first:** If `author == owner_discord_id` or `author == bot_discord_id`, dropped immediately with `Some("discord_author_excluded")`.
  - **Owner mention:** Text containing `<@{owner_id}>` or `<@!{owner_id}>` passes.
  - **Explicit DM metadata:** Checked via `item.room_type == Some("dm")` (or comma-separated containing `"dm"`), decoupling from `thread_ref`.
  - **Explicit Owner Reply metadata:** Checked via `item.room_type == Some("owner_reply")`, allowing verified SQL replies to pass without mentions.
  - **Drop reason:** Any non-mention, non-dm, non-owner-reply message is dropped with `Some("discord_not_owner_mention_or_dm")`.

---

## 2. Exact Adapter Seam for Parent / 23 Worker Delegation

Per instructions, no files under `src/inbox/sources/` were modified by this task. Below is the exact adapter seam specification for the 23 owner to wire:

### In `src/inbox/sources/discord.rs`:
In `DiscordSource::fetch_since`, update the SQL query and `InboxItem` construction to populate `room_type`:

```rust
// 1. Update SELECT query to determine dm and owner_reply status:
let rows = sqlx::query(
    "SELECT d.platform, d.guild_id, d.channel_id, d.thread_id, d.message_id, \
            d.author_id, d.author_name, d.content, d.timestamp, \
            CASE \
              WHEN d.guild_id IS NULL OR d.guild_id = '' THEN 'dm' \
              WHEN EXISTS ( \
                SELECT 1 FROM message_search_documents parent \
                WHERE parent.platform = 'discord' \
                  AND parent.channel_id = d.channel_id \
                  AND parent.message_id = json_extract(d.metadata_json, '$.referenced_message_id') \
                  AND parent.author_id = ? \
              ) THEN 'owner_reply' \
              ELSE 'channel' \
            END AS calculated_room_type \
     FROM message_search_documents d \
     WHERE d.platform = 'discord' \
       AND d.message_id > ? \
       AND d.author_id <> ? \
       AND (? IS NULL OR d.author_id <> ?) \
       AND ( \
         (d.guild_id IS NULL OR d.guild_id = '') \
         OR instr(d.content, ?) > 0 \
         OR instr(d.content, ?) > 0 \
         OR EXISTS ( \
           SELECT 1 FROM message_search_documents parent \
           WHERE parent.platform = 'discord' \
             AND parent.channel_id = d.channel_id \
             AND parent.message_id = json_extract(d.metadata_json, '$.referenced_message_id') \
             AND parent.author_id = ? \
         ) \
       ) \
     ORDER BY d.timestamp, d.message_id",
)
// Bind parameters matching the updated query:
// .bind(&self.owner_id) // for the CASE statement
// .bind(after)
// ... remaining bindings

// 2. Set room_type on InboxItem:
let room_type: String = row.try_get("calculated_room_type")?;
item.room_type = Some(room_type);
```

### In `src/storage/message_search.rs` (Narrow Path Authorized for 23 Owner):
In normal ingress (`src/multiplexer/actor.rs:persist_inbound`), `message_search_documents` is populated by the `message_search_transcript_ai` trigger which lacks `referenced_message_id`.
To make `parent.message_id = json_extract(d.metadata_json, '$.referenced_message_id')` match reliably during live ingress without awaiting REST backfill:
- In `storage/message_search.rs`, expose an upsert helper:
```rust
pub async fn update_referenced_message(
    pool: &SqlitePool,
    platform: &str,
    channel_id: &str,
    message_id: &str,
    referenced_message_id: &str,
) -> Result<()> {
    sqlx::query(
        "UPDATE message_search_documents \
         SET metadata_json = json_set(metadata_json, '$.referenced_message_id', ?) \
         WHERE platform = ? AND channel_id = ? AND message_id = ?",
    )
    .bind(referenced_message_id)
    .bind(platform)
    .bind(channel_id)
    .bind(message_id)
    .execute(pool)
    .await?;
    Ok(())
}
```

---

## 3. Exact `main.rs` Startup and Shutdown Wiring Request for Coordinator

To replace the 5-minute timer loop (`tokio::time::sleep(Duration::from_secs(300))`), the coordinator should apply the following wiring:

```rust
// In src/main.rs: replace spawn_inbox_collector:

fn spawn_inbox_collector(
    pool: &SqlitePool,
    shutdown: tokio::sync::watch::Receiver<bool>,
) -> Result<(tokio::task::JoinHandle<()>, omon_gateway::inbox::watchers::InboxEventSender)> {
    let rules = omon_gateway::inbox::default_filters()?;
    let sources: Vec<Box<dyn omon_gateway::inbox::InboxSource>> = vec![
        Box::new(omon_gateway::inbox::sources::mail::MailSource),
        Box::new(omon_gateway::inbox::sources::kakao::KakaoSource::default()),
        Box::new(omon_gateway::inbox::sources::imessage::IMessageSource),
        Box::new(omon_gateway::inbox::sources::discord::DiscordSource::new(pool.clone())),
    ];

    let (inbox_tx, inbox_rx) = omon_gateway::inbox::watchers::event_channel(
        omon_gateway::inbox::watchers::DEFAULT_EVENT_CHANNEL_CAPACITY,
    );
    let collection_pool = pool.clone();
    let mut shutdown_rx = shutdown.clone();

    // 1. Run initial startup collection pass:
    let startup_pool = pool.clone();
    let startup_sources: Vec<Box<dyn omon_gateway::inbox::InboxSource>> = vec![
        Box::new(omon_gateway::inbox::sources::mail::MailSource),
        Box::new(omon_gateway::inbox::sources::kakao::KakaoSource::default()),
        Box::new(omon_gateway::inbox::sources::imessage::IMessageSource),
        Box::new(omon_gateway::inbox::sources::discord::DiscordSource::new(pool.clone())),
    ];
    let startup_rules = rules.clone();
    tokio::spawn(async move {
        let borrowed: Vec<&dyn omon_gateway::inbox::InboxSource> = startup_sources.iter().map(Box::as_ref).collect();
        let _ = omon_gateway::inbox::watchers::collect_once(&startup_pool, &borrowed, &startup_rules).await;
    });

    // 2. Spawn event-driven collector loop:
    let join_handle = tokio::spawn(async move {
        omon_gateway::inbox::watchers::run_event_watch_with_shutdown(
            collection_pool,
            sources,
            rules,
            inbox_rx,
            None,
            Some(shutdown_rx),
        )
        .await;
    });

    Ok((join_handle, inbox_tx))
}
```

### Event Hook Sites:
1. **Discord Inbound Ingress (`src/discord/adapter.rs` or `src/multiplexer/actor.rs`):**
   ```rust
   let _ = omon_gateway::inbox::watchers::try_on_gateway_message(&inbox_tx);
   ```
2. **iMessage Event Watcher:**
   When `imsg watch` child process yields a line:
   ```rust
   if let Some(rowid) = omon_gateway::inbox::watchers::parse_imsg_watch_line(&line) {
       let _ = omon_gateway::inbox::watchers::on_imessage_event(&inbox_tx, Some(rowid)).await;
   }
   ```
3. **Kakao 2-Minute Cheap Check (no models):**
   ```rust
   let kakao_tx = inbox_tx.clone();
   tokio::spawn(async move {
       let mut interval = tokio::time::interval(std::time::Duration::from_secs(120));
       loop {
           interval.tick().await;
           if omon_gateway::inbox::watchers::on_periodic(&kakao_tx).await.is_err() {
               break;
           }
       }
   });
   ```

---

## 4. Verification Evidence

1. **Rustfmt:**
   ```bash
   rustfmt --check --edition 2021 \
     src/inbox/mod.rs \
     src/inbox/watchers.rs \
     src/inbox/prefilter.rs \
     tests/test_inbox_store.rs
   ```
   **Output:** Exit code 0 (all 4 files strictly conform to edition 2021 formatting).

2. **Scope Compliance:**
   - Files modified: `src/inbox/watchers.rs`, `src/inbox/prefilter.rs`, `tests/test_inbox_store.rs`.
   - Files untouched: `src/inbox/sources/**` (frozen per 23-owner boundary), `src/main.rs`, `Cargo.toml`.
   - No cargo build, full clippy run, live service restarts, or git commits were executed prior to coordinator batch integration.

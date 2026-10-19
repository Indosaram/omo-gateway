# Lane discord-adapter-a

### [P1] The owner's "제안" proposal list is sent as one unchunked message, so a long backlog cannot be shown or approved
- Location: src/discord/adapter.rs:919
- Trigger:
  - Inbox triage is on, which is the default: `OMON_INBOX_ENABLED` unset makes `triage::enabled()` true.
  - The owner DMs "제안" to the primary bot.
  - The pending proposals, joined, are longer than 2000 characters. Each entry is `#n <source: title> -> <proposed_action>\n[route]\n<link>` (`Proposal::display`, src/inbox/proposals.rs:74-90). `proposed_action` is LLM-written and has no length cap. A handful of proposals with mail subjects, links and one-sentence actions is enough.
- Impact:
  - Discord rejects the message (content over 2000 chars). The `?` makes the handler return `Err`, and the owner gets no reply.
  - `presented()` never runs, so no proposal gets a `message_id`.
  - `Proposals::approve` only matches a proposal by `message_id = <replied-to message>` (proposals.rs:401). So no proposal can be approved from Discord until the backlog shrinks below the limit.
  - Every later "제안" fails the same way.
  - Last round fixed this limit only on the approval-result path (`owner_dm_chunks`), not on this listing path.
- Evidence:
  ```rust
  let text = pending.iter().map(|p| p.display()).collect::<Vec<_>>().join("\n\n");
  let message = new_message.channel_id.send_message(&ctx.http,
      serenity::CreateMessage::new().content(format!("{text}\n\n이 메시지에 답장으로 '<번호> 해'를 보내 승인하세요."))
      ...).await?;
  for proposal in pending { proposals.presented(proposal.number, &channel, &message.id.to_string()).await?; }
  ```
  The whole list goes out as one message, and every proposal is bound to that single message id.
- Fix: Pack proposals into messages of at most 2000 characters, splitting only between proposals and putting the instruction line on the last message. Call `presented(number, channel, <id of the message that contains it>)` for each proposal, so a reply to any chunk approves the right one. A single proposal longer than the limit should be truncated, for example with `bound_owner_text`.

### [P2] Debounce coalescing merges different users' messages in shared guild sessions and credits all of it to the last author
- Location: src/discord/adapter.rs:620
- Trigger:
  - In guild channels and threads the session key has `user_id = ""` (adapter.rs:2100-2104), so all users in a channel share one `SessionKey`. The debouncer is keyed by `SessionKey`.
  - User A (for example the owner) and user B each post within the 600 ms window, in a free-response channel or an active thread.
  - Both messages are enqueued and flushed as one batch.
- Impact:
  - `coalesce_inbound_events` joins both texts into one event with no speaker labels.
  - That event takes the last constituent's `author_id`, `platform_message_id` and `delivery_id`, so A's text is indexed in `message_search_documents` under B's author id and message id.
  - `inbox/sources/discord.rs` filters on `d.author_id <> owner` and on bot ids. An owner message followed by a non-owner message therefore surfaces the owner's own text as an inbox item. The reverse order hides the non-owner text from the inbox.
  - Owner-reply classification through `referenced_message_id` is also credited to the wrong author.
- Evidence:
  ```rust
  coalesced.author_id = unique_events
      .iter()
      .rev()
      .find_map(|event| event.author_id.clone());
  ```
  Constituents with different authors are merged without any check.
- Fix: Coalesce only consecutive events with the same `author_id`. Either key the debounce batch by `(SessionKey, author_id)`, or flush the pending batch when an event from a different author arrives.

### [P2] Each `Ready` starts a new missed-message backfill even if the previous one is still running, and the concurrent run can push the cursor past a message whose turn later fails
- Location: src/discord/adapter.rs:1420
- Trigger:
  - `DISCORD_MISSED_BACKFILL=true`.
  - A new `Ready` arrives while an earlier backfill is still awaiting a replayed turn. Turns are awaited one by one and can take minutes. A non-resume reconnect, or the receive watchdog's `restart_shard`, produces that new `Ready`.
- Impact:
  - Run 2 fetches the same messages. For message M, which run 1 has claimed and is still awaiting, `route_claimed_event_awaiting_turn` returns `Ok(false)` (duplicate).
  - The backfill loop treats `Ok(_)` as done and moves the bot cursor past M (adapter.rs:4622-4633).
  - If M's turn then fails, run 1 marks M failed and stops "for retry", but the cursor is already beyond M. M is never replayed, so a missed user message is lost.
  - The two runs also double the REST fetch load.
- Evidence:
  ```rust
  if data.missed_backfill {
      ...
      tokio::spawn(async move {
          match run_missed_message_backfill(&pool, &http, &poise_data, bot_id).await {
  ```
  There is no single-flight guard per bot.
- Fix: Run at most one backfill per bot id at a time, for example with a per-bot `AtomicBool` or `tokio::sync::Mutex` held for the whole run, and skip or queue a backfill that is already in flight.

I checked all of lines 1-2800 against the declined and recorded items in `CONFIRMED.md`, `CONFIRMED-r3.md` and the r1/r2 lane reports, and left those out. Every cited line was re-opened and matches the quoted code.
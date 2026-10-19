<!-- task=st_01a1450f -->
st_01a1450f [completed] model mahoquot/claude-opus-5-5
Ask: map the inbox area of omon-gateway (sources, triage, proposals, digest, proactivity) - wanted: capabilities, data model, and gaps toward an always-on autonomous agent, each with file:line evidence. For you: mapping is done (1/1) and the report is below. The inbox is a read-only collector with a model-based triage step and an owner-approved proposal flow, all on one machine and hardwired to one owner. Nothing was built or run; everything below comes from reading the code. Now: none. Next: none.

## Capabilities

- **Common source interface.** `InboxSource` has two methods, `name()` and `fetch_since(cursor) -> (items, next_cursor)` (src/inbox/mod.rs:116-120). `collect_all` reads each source's cursor, gives each fetch 60s, and on failure writes the error into `inbox_cursors.error` (mod.rs:136-196, 304-315). Snippets are capped at 1000 chars, raw bodies are never stored, and the hashes are SHA-256 (mod.rs:23, 79-102).
- **Dedup and refresh.** New items use `ON CONFLICT(source, source_ref) DO NOTHING` (mod.rs:266-287). A mail thread whose snippet changed is updated in place and its old decisions are deleted so it gets triaged again (mod.rs:224-264).
- **Mail.** Shells out to the `zele mail list` CLI with a hardcoded account and the filter "unread, not promotions/social/updates", limit 100. The cursor is the newest timestamp seen (src/inbox/sources/mail.rs:9-33, 37-82, 127-136). Wakeups come from a long-running `zele mail watch --interval 60` child process (src/main.rs:2751-2808).
- **Kakao.** Calls the PirateTalk HTTP API (`/api/rooms` and `/api/rooms/{id}/messages`, paging with `before_id`). The cursor is a JSON map of room to last message id, with an optional room allowlist (src/inbox/sources/kakao.rs:18-42, 125-232). A 120s timer triggers it (src/main.rs:2378-2397).
- **iMessage.** Uses the `imsg chats` and `imsg history` CLIs; the cursor is an RFC3339 timestamp and messages from the owner are skipped (src/inbox/sources/imessage.rs:12-30, 50-127, 135-175). Wakeups come from `imsg watch --since-rowid` (src/inbox/watchers.rs:208-235; src/main.rs:2618-2682).
- **Discord.** Reads the local `message_search_documents` index, not the live API. It keeps DMs, mentions of the owner, and replies to the owner, and drops bots (src/inbox/sources/discord.rs:41-78). Every gateway message triggers it (src/multiplexer/actor.rs:1170; src/discord/adapter.rs:1631).
- **Event routing.** The watcher events are `Startup`, `GatewayMessage`, `IMessage`, `Mail` and `Periodic`, and each maps to one source (watchers.rs:11-18, 65-96). The drain loop never blocks: sends use `try_send`, and progress results are dropped if the consumer lags (watchers.rs:27-38, 119-161). Everything is wired up in `spawn_enabled_inbox_collector` (src/main.rs:2276-2341), and the source list is in main.rs:2345-2353.
- **Prefilter.** Rules live in a JSON file at `~/.omon/inbox-filters.json`, created on first load (prefilter.rs:39-56). It drops mail promotions/social/updates, excluded Kakao rooms, the owner's own Kakao messages, and open/plus Kakao rooms without a keyword match. For iMessage it drops shortcodes, ads and OTP codes, and for Discord anything that is not a DM, mention or reply (prefilter.rs:58-165). Each drop is counted per KST day (prefilter.rs:197-222).
- **Triage.** Starts only after a collection that inserted something; work is batched 20 at a time with a 120s deadline and up to 3 retries (triage.rs:16-20, 162-209). Items older than 24h are filed as `fyi` without a model call (triage.rs:289-293). The `jev` judge sorts each item into owner / bot / fyi / ignore (triage.rs:24-25, 296-308). For owner and bot items, a small-model writer then produces a reason and a proposed action in strict JSON (triage.rs:225-279, 486-591). If the writer fails, the item is stored with a fixed fallback reason instead of being retried forever (triage.rs:333-372). Deadlines are parsed by regex for ISO dates and today/tomorrow (KST) (triage.rs:458-494). An owner item due within 24h raises an `inbox_urgent` lane event (triage.rs:395-400). Pending work is recovered on startup (triage.rs:416-426).
- **Proposals and approval.** There are three action kinds: `Repo{cwd, done}`, `Lookup` and `Draft` (proposals.rs:51-57). The route comes from trusted config (`OMON_INBOX_PROPOSAL_ACTION`), never from inbox text (triage.rs:409-415; proposals.rs:319-321). Approval works three ways:
  - **Discord DM:** the owner types `제안` ("proposals") to get the numbered list, then replies `<n> 해` ("do #n") to approve (src/discord/adapter.rs:899-1000; proposals.rs:391-407, 571-579).
  - **CLI:** `inbox set --status approved` (set.rs:30-71; proposals.rs:410-428).
  - **Auto-lookup:** opt-in, lookup actions only (proposals.rs:521-547; triage.rs:431-459).
- **Execution safety.** The state is claimed with a compare-and-swap before any effect runs, so a crash or failure never retries automatically (proposals.rs:430-505). `Repo` starts a coding lane (proposals.rs:226-262). `Lookup` is an HTTP GET capped at 64 KiB (proposals.rs:119-150). `Draft` produces text for the owner only; the `Actions` trait has no way to send a message (proposals.rs:98-108, 151-160).
- **Digest.** A daily 09:00 KST cron job delivers to the owner's home DM only and is not attached to the session transcript (digest.rs:18-46). It lists owner tasks, bot proposals, usage, cron problems and sources that need re-login (digest.rs:69-125), and checks the delivery ledger so it is not re-sent (digest.rs:82-89, 133-160).
- **Proactivity (kibitzer).** A SQLite trigger on `inbox_items` inserts bumps a counter (src/lanes/kibitzer.rs:180-195). The scheduler spaces wakes out, caps them at 6 per day, and stays quiet before 08:00 KST (kibitzer.rs:211-240). It builds a state summary from home-DM messages, lanes, events and owner-preference memories (kibitzer.rs:376-428). A judge must score at least 0.7, then a model turn returns `{wake, reason, suggested, target_lane}`, which is written as a `lane_events` row of kind `kibitzer` (kibitzer.rs:242-366).
- **CLI.** `inbox list` and `inbox set` (src/inbox/cli.rs:20-35), plus `inbox-propose` (src/entry.rs:63, 103).

## Data model

- **`inbox_items`**: id, source (CHECK limited to mail/kakao/discord/imessage), source_ref, thread_ref, author, title, snippet, received_at, link, raw_hash, collected_at, unique on (source, source_ref) (migrations/0034_inbox.sql:1-14).
- **`inbox_decisions`**: item_id, class, reason, due_at, proposed_action, lane_key, status (open/proposed/approved/done/dismissed), decided_at, model (0034_inbox.sql:16-26). An `outcome` column is added at runtime (proposals.rs:306-318).
- **`inbox_cursors`** (source, cursor, updated_at, error) and **`inbox_filter_stats`** (day, source, rule, count) (0034_inbox.sql:28-41).
- **`inbox_proposals`**: number, item_id, owner_id, channel_id, message_id, action_json, proposed_action, link, title, state (pending/running/completed/failed), approval_message_id, error. It is created at runtime, not by a migration (proposals.rs:294-305). There is also an `inbox_items_cron` table (migrations/0041_cron_remediation.sql:2).
- **Tables read or written elsewhere:** `lane_events` (`inbox_urgent`, `kibitzer`), `kibitzer_state`, `cron_runs`/`cron_outputs`, `sessions`, `message_search_documents`, `bot_profiles`, `messages`, and the memory store.
- **Rust types:** `InboxItem`, `InboxSourceKind` (mod.rs:41-77); `Class`, `WritingItem`, `TriageWriter` (triage.rs:42-98); `Action`, `Proposal`, `Actions`/`DirectActions`, `ApprovalResult`, `Reply` (proposals.rs:51-290); `PrefilterRules` (prefilter.rs:11-20); `InboxWatcherEvent` (watchers.rs:11-18).

## Limits and gaps (toward a 24/7 autonomous agent platform)

1. **One owner, hardcoded.** The owner id is a constant (proposals.rs:10), and is hardcoded again in discord.rs:18 and prefilter.rs:31. The mail account is hardcoded too (mail.rs:9). There are no tenants or users. Hook: turn `OWNER_ID` into a per-user setting threaded through `Proposals::new` (proposals.rs:292), `DiscordSource::new` (discord.rs:15) and `MailSource::args` (mail.rs:16).
2. **The source list is closed.** It is a Rust enum plus a SQL CHECK (mod.rs:41-48; 0034_inbox.sql:3, 37). Adding a connector (Slack, Calendar, Notion, SMS, web) needs a migration and a change to the enum. Hook: make `InboxSourceKind` open-ended or string-keyed, and extend `inbox_sources()` (main.rs:2345) and `filter_sources_for_event` (watchers.rs:65).
3. **Tied to the owner's laptop.** Sources depend on local CLIs and a tailnet URL (`zele`, `imsg`, PirateTalk) (sources/mod.rs:40-68; kakao.rs:20-22). There is no cloud execution and no OAuth or connector registry; an expired login only shows up in the digest (digest.rs:93-97). Hook: an `InboxSource` implementation backed by a connector service and a credential store, plus re-auth prompts driven from `record_source_error` (mod.rs:304).
4. **Strictly read-only, with a narrow action set.** Only `Repo`, `Lookup` and `Draft` exist (proposals.rs:51-57), and the trait deliberately cannot send or reply (proposals.rs:98-100). There is no "send reply as owner", calendar booking, payment or form-fill. Hook: new `Action` variants and `Actions` methods, plus the branches in `execute_approved` (proposals.rs:468-478).
5. **Approval is all or nothing.** The only automatic action is lookup (proposals.rs:539-541). There are no policy tiers, risk scores, spend limits or per-connector auto-approval, and the approval grammar is a fixed Korean regex (proposals.rs:571). Hook: an approval policy layer between `propose_in` (proposals.rs:337) and `auto_lookup`, plus the approval branch in the adapter (adapter.rs:960).
6. **No retry or multi-step plans.** A failed proposal is final (proposals.rs:498-502). `Repo` hands off to a lane, but there is no plan-and-execute flow across connectors. Hook: `execute_approved`, together with lane orchestration (`start_lane`, proposals.rs:226).
7. **Thin memory use.** Triage never reads memory or contact history; each item is classified on its own snippet (triage.rs:296-308). Only the kibitzer reads memories, using a fixed query, "owner preferences" (kibitzer.rs:384-386). Nothing learns from approve or dismiss outcomes. Hook: add memory or contact context in `Item::text` (triage.rs:100-123), and write outcomes back to memory from `execute_approved` and `update_status` (set.rs:73).
8. **Proactivity is gated and only suggests.** The kibitzer may only emit a suggestion: tools are disabled, 6 wakes per day, quiet hours, and a fixed lead (kibitzer.rs:291-301, 360-366). It never runs a proposal itself, and triage's `inbox_urgent` event only sets a flag (triage.rs:395). Hook: route kibitzer decisions with `target_lane` into `Proposals`/lane start, and make the caps configurable (kibitzer.rs:229-238).
9. **One delivery channel.** Delivery is Discord DM only: the digest refuses any other destination (digest.rs:19-30) and proposals are presented only in the adapter (adapter.rs:913-958). There is no push, email, SMS or web UI. Hook: `digest::spec` and the presentation path, behind a channel abstraction (`OutboundDispatcher` is already used in triage.rs:520).
10. **Shallow content.** Snippets max out at 1000 chars, with no attachments or full bodies (mod.rs:23; kakao.rs:94-98, imessage.rs:100-104). Deadline parsing is a regex for ISO dates and today/tomorrow only (triage.rs:458). The `direct_class` helper (prefilter.rs:167) is never called anywhere in src/.
11. **Dead or unwired code.** `run_cursor_watch` and `run_imessage_watch` (watchers.rs:240-276) are not called from main.rs, which uses its own supervisors instead (main.rs:2618, 2751).
12. **Schema drift risk.** `inbox_proposals` and the `outcome` column are created at runtime rather than by migrations (proposals.rs:294-318).

## EXPAND

- src/lanes/dispatch.rs:213 handles `inbox_urgent` and similar lane events, which is how they reach the owner.
- `crate::judge` (src/judge/mod.rs:69-139) defines the `InboxClass` fallback (`owner_must_act`) and the daily cap.
- src/cron/report.rs:233 and src/mirror.rs:236 and 384 connect the digest to cron reporting and mirroring.
- The tests that pin the current behavior: tests/test_inbox_triage.rs, tests/test_inbox_proposals.rs, tests/test_inbox_store.rs, tests/test_inbox_sources.rs.

<results>
<files>
- /Users/indo/code/project/omon-gateway/src/inbox/mod.rs - InboxItem, InboxSource trait, collect_all and commit_source with dedup and cursors
- /Users/indo/code/project/omon-gateway/src/inbox/sources/mod.rs - CLI runner (run_json) and structured-output decoding
- /Users/indo/code/project/omon-gateway/src/inbox/sources/mail.rs - zele Gmail source, hardcoded account
- /Users/indo/code/project/omon-gateway/src/inbox/sources/kakao.rs - PirateTalk HTTP source, per-room cursor map
- /Users/indo/code/project/omon-gateway/src/inbox/sources/imessage.rs - imsg CLI source
- /Users/indo/code/project/omon-gateway/src/inbox/sources/discord.rs - Discord source over the local message_search_documents index
- /Users/indo/code/project/omon-gateway/src/inbox/prefilter.rs - drop rules, filter stats, unused direct_class
- /Users/indo/code/project/omon-gateway/src/inbox/watchers.rs - event channel, routing events to sources
- /Users/indo/code/project/omon-gateway/src/inbox/triage.rs - jev classification, small-model writer, deadline parsing, inbox_urgent, auto-lookup loop
- /Users/indo/code/project/omon-gateway/src/inbox/proposals.rs - Action kinds, approval with compare-and-swap, GatewayActions
- /Users/indo/code/project/omon-gateway/src/inbox/digest.rs - daily 09:00 KST owner-DM digest
- /Users/indo/code/project/omon-gateway/src/inbox/set.rs - CLI status changes and CLI approval
- /Users/indo/code/project/omon-gateway/src/inbox/cli.rs - inbox list/set commands
- /Users/indo/code/project/omon-gateway/src/lanes/kibitzer.rs - proactive wake loop triggered by inbox inserts
- /Users/indo/code/project/omon-gateway/src/main.rs - collector, triage and watcher wiring (lines 2270-2830)
- /Users/indo/code/project/omon-gateway/src/discord/adapter.rs - "제안" listing and "<n> 해" approval in DMs (lines 899-1000)
- /Users/indo/code/project/omon-gateway/migrations/0034_inbox.sql - inbox schema
</files>
<answer>The inbox is a read-only, event-driven collector over four hardwired local sources, followed by model-based triage (jev classification plus a small-model writer) and owner-approved proposals limited to repo lane, lookup and draft. A daily DM digest and the kibitzer proactive loop sit on top. The main gaps for an always-on agent are: a single hardcoded owner and account, a closed source enum and CHECK constraint, dependence on local CLIs, no actions that act for the owner, all-or-nothing approval policy, no memory in triage, a kibitzer that only suggests, and Discord DM as the only output channel. The exact places to extend each one are listed above.</answer>
<next_steps>Ready to proceed - no follow-up needed.</next_steps>
</results>
<!-- task=st_01a14514 -->
st_01a14514 [completed] model mahoquot/claude-opus-5-5
I read this as an information ask: map the tools, tracker, ledger, storage and migrations areas and report. I'll stop when the report with file:line evidence is delivered.

<analysis>
**Literal Request**: Map capabilities in src/tools, src/tracker, src/ledger, src/storage and migrations/.
**Actual Need**: Know what the tool surface, integrations and schema can do today, and where to hook extensions toward an always-on personal agent platform.
**Success Looks Like**: Capabilities, data model and gaps, each tied to file:line evidence.
</analysis>

## Capabilities

**Tool framework (src/tools)**
- Every tool implements `Tool` with a name, description, JSON schema and `execute`. Two optional hooks exist: `requires_approval` and `execute_with_context(session)` (src/tools/mod.rs:39-56).
- `ToolRegistry` keeps tools in a HashMap with an approval timeout (default 900s) (mod.rs:59-73).
- `execute_with_context` gates any tool that asks for approval. It skips the gate for yolo sessions and keys the approval on `tool-rule:v1:<sha256(name,reason)>`. Decisions are Once, Session, Always or Deny (mod.rs:125-195).
- Production registration happens in src/main.rs:1046-1096: terminal (with an optional Tirith scanner, main.rs:1059-1076), file, `McpTool::default()`, cron, web_search, web_fetch, browser, and skills (hermes skill dirs). `message_context` is registered automatically when a Discord token is in the environment (mod.rs:76-81).
- **terminal**: shell commands inside the workspace and authorized roots. In order, it applies a hardline blocklist, the `APPROVALS_DENY` deny globs, the policy (Never, Always or Smart), the external scanner verdict, and finally an interactive approval (terminal.rs:202-307). It also has a timeout and an output cap (terminal.rs:114-124).
- **file**: read, write, list and search, confined to the root (file.rs:293-304). Write approval is optional (file.rs:306-316).
- **mcp**: one dispatcher tool that routes `tool` to a configured client over stdio or SSE (`McpTransport`, mcp.rs:18-28). It sends JSON-RPC `tools/call` (mcp.rs:76-96) and supports per-client approval (mcp.rs:244-261).
- **cron**: around 20 actions covering list, add, update, pause, resume, trigger, runs, incident ack and notepads (cron.rs:172-244). A job can carry a prompt and/or a script, a `deliver` target like `discord:123`, and `enabled_toolsets`. Expression forms are cron, `@every`, `once:` and `delay`. Adding or updating a script requires approval (cron.rs:247-275).
- **web_search** scrapes DuckDuckGo HTML (web.rs:59). **web_fetch** blocks internal IPs and redirects (web.rs:289-350) and can route through an optional reader prefix such as jina (web.rs:276).
- **browser** drives Chrome over CDP: navigate, snapshot, eval and screenshot (browser.rs:290-308). It opens tabs via `/json/new` (browser.rs:422) and refuses internal destinations (browser.rs:220-280).
- **skills**: list, search, read and write over the hermes skill dirs. Writes can be staged through pending_writes (skills.rs:38, 140-162).
- **message_context**: recent, search, get_message and replies over Discord, using FTS5 (message_context.rs:266-288). The provider trait is keyed by platform (message_context.rs:219-228).
- Other surfaces that use the registry: Discord slash commands call `skills` and `cron` directly (src/discord/commands.rs:575, 746). The dashboard lists tool schemas (src/dashboard.rs:1926-1942).

**Ledger (src/ledger)**
- `DeliveryLedgerService` handles inbound dedupe and completion (`record_incoming`, `is_duplicate`, `mark_delivered`/`mark_failed`, service.rs:116-263).
- It also tracks outbound delivery obligations through the states Pending, Attempting, Delivered, Failed and Abandoned, with owner pid and start time (service.rs:13-55, 376-424). `recover_pending_delivery_obligations` resends obligations stranded by a crash, prefixed with `RECOVERED_REPLY_MARKER` (service.rs:8, ledger/mod.rs:3-6).

**Storage (src/storage)**
- The `Database` pool runs migrations (db.rs:151-232).
- Session resume and suspend recovery: db.rs:322-525.
- Pending writes, an approval queue for memory and skill writes: storage/mod.rs:9-18, used at memory/store.rs:43-51.
- Work items and dead targets: storage/mod.rs:13-17.
- Lanes CRUD, transitions, transcripts and an event outbox (lanes.rs:174-675).
- FTS5 message index (message_search.rs:31-255).
- Per-platform messenger policy overrides (messenger_policy.rs:6-66).

**Tracker (src/tracker)**
- This is a local clone of Linear, not a Linear API integration.
- Lane-to-issue sync into the AGT team: tracker/lanes.rs:99.
- `delegate_issue` writes a brief and starts a lane: lanes.rs:477-567. `execute_lane_action` covers send, object, close and reopen: lanes.rs:571-640.
- Seeding: seed.rs:83. Model structs: model.rs:12-252.
- **The module is not compiled.** src/lib.rs:1-24 has no `pub mod tracker;`. tracker/lanes.rs:771 references `crate::tracker::api`, and that file does not exist (src/tracker contains only lanes.rs, model.rs and seed.rs). The planned API is in .omo/plans/linear-tracker.md:27, 107.

**External integrations found**
- Discord is the main channel.
- Gmail is read through a CLI, with the account hardcoded in src/inbox/sources/mail.rs:9 and src/main.rs:686.
- Inbox sources are limited to mail, kakao, discord and imessage (migrations/0034_inbox.sql:3).
- MCP is supported, but no servers are configured.
- There is no Linear, GitHub, Slack or OAuth connector code: a grep across src found none.

## Data model

Migrations 0001-0044 (0017 and 0036 are absent):
- **Core**: sessions, messages (rebuilt as messages_v2 in 0003), delivery_ledger, cron_jobs, memories (0001); delivery_obligations (0006); approval_allowlist (0007); pending_writes (0010).
- **Identity and pairing**: pairing_codes, paired_users (0011); pairing_platform_lockout, pairing_notifications (0024); bot_profiles (0014).
- **Discord cursors and policy**: discord_channel_cursors (0012); discord_bot_cursors (0018); dead_targets (0019); messenger_policy_overrides (0016); message_search_documents and the FTS5 table message_search_fts (0015).
- **Cron**: cron_runs (0004); cron_outputs (0021); cron_cutover_receipts and cron_cutover_receipt_stores (0023); cron_monitor_states (0025); cron_incidents (0026); cron_notepads (0027); cron_problems (0035); inbox_items_cron and cron_remediation_outcomes (0041).
- **Agent work**: work_items (0030); lanes, lane_history, lane_events, lane_transcript (0031); judge_calls (0032); lane_inbox (0033); lane_thread_bindings, lane_close_intents, lane_owned_worktrees, lane_owner_replies (0038).
- **Inbox**: inbox_items, inbox_decisions (classes owner, bot, fyi, ignore; statuses open, proposed, approved, done, dismissed), inbox_cursors, inbox_filter_stats (0034).
- **Telemetry and sessions**: model_usage, lane_usage_context (0039); external_sessions, external_session_items (0040); session_watches, session_watch_notices (0043).
- **Tracker** (0044): 20 `tracker_*` tables (users with kind human or agent, teams, states mapped to lane_status, labels, projects, issues with `lane_key` pointing at lanes, comments, relations, activity, views, favorites, and more).
- **Legacy**: legacy_guild_session_conflicts, legacy_guild_pending_auth (0028).

## Limits and gaps for an always-on personal agent platform

1. **Gateway tools are not the agent's tools.** The production agent is the OMO app-server, which runs its own tool loop. The gateway only forwards `enabledToolsets` (src/agent/omo_protocol.rs:43-44; design note at src/agent/backend.rs:19-26). `ToolRegistry` serves slash commands and the dashboard. New connectors therefore have to land as MCP servers or OMO toolsets, or an MCP bridge has to expose `ToolRegistry`. Hook: main.rs:1096 (`tool_names`).
2. **No MCP servers are configured.** `McpTool::default()` has an empty client list (main.rs:1083), and there is no config loader. The only `McpClientTool::new` call is in a test (mod.rs:299). Hook: main.rs:1083, which needs to be fed from a config file or DB table.
3. **No connector or credential layer**: no OAuth, token vault or per-connector accounts. Gmail is a hardcoded CLI account (inbox/sources/mail.rs:9). Hook: a new migration, plus `InboxSource` implementations under src/inbox/sources, plus MCP auth through `McpTransport::Sse.bearer_token` (mcp.rs:24-27).
4. **The tracker is dead code.** The module is not in lib.rs and `api.rs` is missing (tracker/lanes.rs:771). It cannot serve as the task surface for proactive work until it is wired in at lib.rs:1-24 and the dashboard router.
5. **Approvals are Discord-centric and synchronous.** An approval blocks the tool call until a timeout (mod.rs:155-167; default 900s at mod.rs:70). There is no durable approve-later queue for actions, only for memory and skill writes (pending_writes). Hook: `ApprovalRequester` plus `stage_pending_write` (storage/mod.rs:9-18), generalized to tool actions.
6. **Memory is basic.** One `memories` table keyed by session (0001_initial.sql:53-63) with in-process scoring over at most 10k rows (memory/store.rs:22-23). There are no embeddings, no cross-session user profile and no memory tool in the registry. Hook: a `Tool` for MemoryStore registered at main.rs:1090.
7. **Multi-channel coverage is thin.** `MessageContextProvider` supports only Discord (message_context.rs:255-263). Inbox sources stop at four platforms, enforced by a SQL CHECK (0034_inbox.sql:3, 37), so adding Slack, Telegram or SMS needs a migration that relaxes the CHECK. Ledger obligations assume `channel_id`/`thread_id` (ledger/service.rs:41-55).
8. **There is no isolated cloud computer.** terminal and file run on the host, limited by root and deny-glob checks (terminal.rs:61-160). browser assumes a local CDP endpoint on 127.0.0.1 (browser.rs:39, 422). Hook: `TerminalTool::new` and `BrowserTool::new(cdp_port)` would need a remote-sandbox transport.
9. **Proactivity rests on cron plus the inbox triage judge.** There are no event triggers or webhooks. Cron `deliver` is a string target (cron.rs:213). Hook: the cron tool schema and CronScheduler (cron.rs:119-160), and `inbox_decisions.proposed_action` (0034_inbox.sql:20).
10. **`build_cron_tools` (main.rs:614) has no call site in src.** Per-job sandboxed registries may never be applied.
11. **web_search is a fragile DuckDuckGo HTML scrape** (web.rs:59).

## EXPAND

- Check whether `build_cron_tools` is used outside src (tests/ or a cfg path), and how cron prompt jobs reach OMO.
- Read the OMO app-server toolset definitions to see whether connectors already exist on that side.
- Check how far .omo/plans/linear-tracker.md got, i.e. whether tracker/api.rs and the dashboard wiring are pending work.

<results>
<files>
- /Users/indo/code/project/omon-gateway/src/tools/mod.rs - Tool trait, ToolRegistry and the approval gate
- /Users/indo/code/project/omon-gateway/src/tools/terminal.rs - shell tool and its layered approval and blocklist checks
- /Users/indo/code/project/omon-gateway/src/tools/mcp.rs - MCP stdio/SSE client and dispatcher
- /Users/indo/code/project/omon-gateway/src/tools/cron.rs - scheduled prompt and script jobs
- /Users/indo/code/project/omon-gateway/src/tools/web.rs - DuckDuckGo search and SSRF-guarded fetch
- /Users/indo/code/project/omon-gateway/src/tools/browser.rs - CDP browser tool
- /Users/indo/code/project/omon-gateway/src/tools/file.rs - workspace file tool
- /Users/indo/code/project/omon-gateway/src/tools/skills.rs - skills tool
- /Users/indo/code/project/omon-gateway/src/tools/message_context.rs - Discord context and FTS search provider trait
- /Users/indo/code/project/omon-gateway/src/main.rs - production tool registration (lines 1046-1096)
- /Users/indo/code/project/omon-gateway/src/agent/omo_protocol.rs - only enabledToolsets reaches the agent
- /Users/indo/code/project/omon-gateway/src/ledger/service.rs - delivery ledger and obligations
- /Users/indo/code/project/omon-gateway/src/storage/mod.rs - storage exports (pending writes, lanes, search, policy)
- /Users/indo/code/project/omon-gateway/src/tracker/lanes.rs - Linear-clone sync and delegation (not compiled)
- /Users/indo/code/project/omon-gateway/src/lib.rs - no tracker module
- /Users/indo/code/project/omon-gateway/migrations/0044_tracker.sql - tracker schema
- /Users/indo/code/project/omon-gateway/migrations/0034_inbox.sql - inbox schema and source CHECK
- /Users/indo/code/project/omon-gateway/src/inbox/sources/mail.rs - hardcoded Gmail CLI source
</files>
<answer>
The tool surface is terminal, file, mcp, cron, web_search, web_fetch, browser, skills and message_context, all behind an approval gate keyed by session. Production agent turns, however, run in the OMO app-server, which uses its own tools; the gateway registry serves slash commands and the dashboard. There is no Linear or other SaaS connector: the "tracker" is an internal Linear clone that is not compiled into the crate, and the MCP client list is empty in production. Persistence is SQLite with roughly 70 tables across 42 migration files. The biggest platform gaps are: no connector or OAuth layer, empty MCP config, Discord-only messaging and approvals, simple session-scoped memory, host-local execution with no sandboxed cloud computer, and proactivity limited to cron plus inbox triage.
</answer>
<next_steps>
To add connectors, start at the MCP config loader (main.rs:1083) or the OMO toolsets. Wire the tracker in by adding `pub mod tracker` to lib.rs and creating tracker/api.rs. Generalize pending_writes into a durable action-approval queue.
</next_steps>
</results>
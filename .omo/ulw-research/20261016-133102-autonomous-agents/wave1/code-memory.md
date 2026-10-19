<!-- task=st_01a14510 -->
st_01a14510 [completed] model mahoquot/claude-opus-5-5
Ask: Map what the memory area of omon-gateway can do today (src/memory, src/profiles, bot_profiles/) - wanted: capabilities, data model and platform gaps, each with file:line evidence. For you: the report below is complete and every claim comes from a file I read. Nothing was built or run. The main finding is that the gateway's own memory store is small and keyword-only, and the agent can't write to it with any tool. The agent's working memory lives in the omo runtime, keyed per workspace. Now: final report. Next: none.

## Capabilities

- **Per-session memory store.** `MemoryStore` has `remember`, `get`, `delete` and `search` (`/Users/indo/code/project/omon-gateway/src/memory/store.rs:30-155`). `remember` first creates the session row if it is missing, using `ensure_session` (`store.rs:42`, `store.rs:213-229`).
- **Lexical search only.**
  - The score is term-frequency cosine, plus 0.25 per overlapping term, plus 1.0 for a phrase match (`store.rs:119-133`). Ties go to the most recently updated row (`store.rs:134-139`).
  - A search reads at most the 10,000 newest rows of one session (`store.rs:23`, `store.rs:108-116`).
  - `keyword_search` and `similarity_search` are just aliases of `search`; there are no embeddings (`store.rs:157-173`).
  - A row with corrupt metadata is skipped instead of failing the search (`store.rs:141-154`, test at `store.rs:258`).
- **Memory write approval.**
  - With `WRITE_APPROVAL=1/true/yes/on` (`/Users/indo/code/project/omon-gateway/src/storage/db.rs:730-740`), `remember` stages the write in `pending_writes` and returns a fake `Memory` with score 0 (`store.rs:43-62`; staging at `db.rs:742-754`).
  - Approving a staged write inserts it into `memories` inside one transaction (`db.rs:832-860`). Skill writes go through the same gate (`/Users/indo/code/project/omon-gateway/src/tools/skills.rs:232-265`).
- **Owner-facing memory surfaces.**
  - Discord `/memory`: `list` shows the 10 latest memories; `pending`, `review`, `approve` and `reject` are scoped to the caller's session (`/Users/indo/code/project/omon-gateway/src/discord/commands.rs:680-731`). A test proves another session can't review your staged memory (`commands.rs:1749`).
  - `/reset confirm:true` wipes a session's memories (`commands.rs:846`, `commands.rs:874`).
  - Dashboard: a memory list/search endpoint (`/Users/indo/code/project/omon-gateway/src/dashboard.rs:2291-2330`) and a memory count in stats (`dashboard.rs:905-946`).
- **Who reads gateway memory at runtime.** Only the kibitzer, which pulls the top 10 hits for the fixed query "owner preferences" (`/Users/indo/code/project/omon-gateway/src/lanes/kibitzer.rs:385-387`). The only code that writes to `MemoryStore` is a test (`skills.rs:382`).
- **Agent memory through per-agent workspaces.**
  - When `per_agent_workspace` is on (default true, env `OMON_PER_AGENT_WORKSPACE`; `/Users/indo/code/project/omon-gateway/src/agent/omo_config.rs:179`, `omo_config.rs:322`), each session gets `<root>/agents/<slug>` plus a shared `<root>/shared` (`/Users/indo/code/project/omon-gateway/src/agent/agent_workspace.rs:76-80`).
  - It also seeds `.omo/omo.json` with `{"memory":{"agent":slug}}`, so the omo runtime keeps memory per agent (`/Users/indo/code/project/omon-gateway/src/agent/omo_backend.rs:481-533`).
  - The slug is chosen in order bot, cron, web, user, sys, and gets a hash suffix when sanitized or truncated (`agent_workspace.rs:40-74`).
  - The root is `OMON_WORKSPACE_ROOT`, else `~/.omon/workspace` (`omo_config.rs:152`, `omo_config.rs:338`).
- **Persona (bot profiles).**
  - The `bot_profiles` row is keyed by Discord bot id. It supplies the system prompt and toolsets for a session only when the session state hasn't set them; the model comes from the profile unless the owner-lead model applies; the profile router fills whatever is still empty (`/Users/indo/code/project/omon-gateway/src/multiplexer/actor.rs:1344-1393`, fresh session at `actor.rs:1404-1444`).
  - Toolsets are stored as a comma-separated string (`actor.rs:1365`).
  - Dashboard has full CRUD for profiles (`dashboard.rs:2052-2260`).
- **Managed prompt block.**
  - `apply_block` swaps exactly one `<!-- omon:lanes:begin/end -->` region inside `system_prompt`. It runs in a `BEGIN IMMEDIATE` transaction, rejects malformed markers, and returns before/after sha256 digests (`/Users/indo/code/project/omon-gateway/src/profiles/lanes_block.rs:4-5`, `lanes_block.rs:62-189`).
  - The CLI subcommands `profile export` and `profile apply-block` call it (`/Users/indo/code/project/omon-gateway/src/entry.rs:161-185`).
  - The source block is `/Users/indo/code/project/omon-gateway/bot_profiles/blocks/lanes.md`. It covers lead routing, lane briefs, the done rule, kibitzer, onboarding (consent per integration), inbox, sessions and cron health.
- **Seed profile.** `/Users/indo/code/project/omon-gateway/bot_profiles/omomeow.json` defines OmOMeow: a Korean persona, toolsets terminal/file/mcp/cron/skills/web/message_context, and `custom_settings.per_agent_workspace=true`, `delegate_command`, `heartbeat_interval_secs: 900`.
- **Kibitzer (proactive wake).** Each step, with its evidence:
  - **Triggers:** SQLite triggers on `inbox_items` inserts and on `lane_events` of kind blocked/question/stale/not_done/cron_problem bump `kibitzer_state.generation` (`kibitzer.rs:180-198`). An owner message in the home DM also arms it (`actor.rs:1153-1187`, `kibitzer.rs:477`).
  - **Pacing:** at least 30 minutes between wakes, at most 6 per KST day, nothing before 08:00 KST (`kibitzer.rs:23`, `kibitzer.rs:229-237`). It checks state every 5s but calls no model while idle (`kibitzer.rs:31`, `kibitzer.rs:442-475`).
  - **Two-stage gate:** first the judge (`noul`) must score at least 0.7 (`kibitzer.rs:266-283`). Then a model turn with no tools, 60s timeout, must return strict `Decision` JSON: wake, reason, suggested, target_lane (`kibitzer.rs:50-69`, `kibitzer.rs:284-339`).
  - **Output:** a scrubbed `lane_events` row of kind `kibitzer`; the daily cap is enforced inside the same atomic insert (`kibitzer.rs:341-363`). A failed evaluation re-arms itself (`kibitzer.rs:366-373`).
  - **Its context document:** the last 30 home-DM messages, open lanes, the last 10 events and memories, capped at 4,000 chars (`kibitzer.rs:376-439`). Its capture sink refuses uploads and approval requests (`kibitzer.rs:94-98`).
  - **Single owner:** the lead is hard-coded to one owner, one bot and one DM channel (`kibitzer.rs:21-22`, `kibitzer.rs:125-134`). It can be disabled with `OMON_KIBITZER_ENABLED=false` (`kibitzer.rs:147`).

## Data model

| Table / struct | Fields | Evidence |
|---|---|---|
| `memories` | id, session_key (FK sessions ON DELETE CASCADE), content, metadata_json, created_at, updated_at; index (session_key, created_at) | `/Users/indo/code/project/omon-gateway/migrations/0001_initial.sql:53-63` |
| `pending_writes` | id (8-char), kind (`memory` or `skill`), payload JSON, created_at | `/Users/indo/code/project/omon-gateway/migrations/0010_pending_writes.sql:2-10` |
| `bot_profiles` | bot_id PK, name, model, system_prompt, enabled_toolsets (CSV), custom_settings_json, timestamps | `/Users/indo/code/project/omon-gateway/migrations/0014_bot_profiles.sql:1-10` |
| `kibitzer_state` (created at runtime, not a migration) | lead PK, generation, consumed, admitted_ms; plus triggers `kibitzer_inbox_insert` and `kibitzer_lane_insert` | `kibitzer.rs:182-191` |
| Read by the kibitzer | `lanes`, `lane_events`, `messages`, `sessions` | `kibitzer.rs:378-384` |
| Rust types | `Memory` (`store.rs:11-20`), `BotProfileExport` / `PromptDigest` / `ApplyBlockResult` (`lanes_block.rs:7-30`), `Decision` / `TargetLane` (`kibitzer.rs:33-56`), `AgentWorkspace` (`agent_workspace.rs:76`) | |

## Limits and gaps for an always-on personal-agent platform

1. **The agent can't write to or read gateway memory with a tool.** No tool calls `MemoryStore` (only the test at `skills.rs:382`). Durable memory is left to the omo runtime's per-slug store (`omo_backend.rs:514`), which the gateway can't see. Hook: a `memory` toolset next to `/Users/indo/code/project/omon-gateway/src/tools/skills.rs`, calling `MemoryStore::remember/search` (`store.rs:36`, `store.rs:100`). The onboarding block already tells the agent to "record useful memory" (`bot_profiles/blocks/lanes.md`, Onboarding).
2. **Memory is siloed per session.** Memories are keyed by `session_key` (`store.rs:111`), which includes channel and thread. A fact learned in one thread is invisible from the DM or another channel. The kibitzer query only finds lead-DM memories (`kibitzer.rs:386`). There is also a `session_key = 'owner-scope'` row seen in a test (`commands.rs:1865`), which suggests partial owner scoping. Hook: add a `scope`/`owner_id` column to `memories` (`0001_initial.sql:53`) and change the `search` WHERE clause (`store.rs:111`).
3. **No semantic retrieval, no consolidation or decay.**
   - Scoring is lexical (`store.rs:119-133`), and "similarity_search" is a misleading alias (`store.rs:166`).
   - Nothing ever updates `updated_at` or `content`; there is no update API, dedup, summarization, TTL or importance field.
   - Hook: an embedding column or a sqlite-vec table, with ranking in `search`, plus a consolidation cron next to the kibitzer.
4. **Memory isn't injected into normal turns.** The persona prompt comes from `bot_profiles` (`actor.rs:1356-1361`), and no retrieved memory is ever added to it. Hook: in `fresh_context` / session load (`actor.rs:1344`, `actor.rs:1404`), or in `thread_start_request` (`omo_backend.rs:548`).
5. **Approval is a global environment switch.** `WRITE_APPROVAL` applies to all writes (`db.rs:730`). There's no per-kind, per-source or per-risk policy, no expiry, and no proactive approval-request message (the kibitzer explicitly refuses `ApprovalRequest`, `kibitzer.rs:94`). The staged return value fakes a `Memory` with a made-up id (`store.rs:54-61`). Hook: `stage_pending_write` (`db.rs:742`) and `OutboundAction::ApprovalRequest`.
6. **The kibitzer only serves one hard-coded owner.**
   - Owner, bot and channel ids are hard-coded (`kibitzer.rs:21-22`, `kibitzer.rs:125-134`).
   - It has no tools, so it only suggests (`kibitzer.rs:294`).
   - Its output is a lane event, not an action (`kibitzer.rs:359`).
   - The pacing constants are fixed (`kibitzer.rs:23-31`, `kibitzer.rs:229-237`).
   - Only inbox and lane triggers arm it (`kibitzer.rs:185-191`); there are no connector or calendar triggers.
   - Multi-tenant, configurable proactivity would hook at `Kibitzer::new`/`from_env` (`kibitzer.rs:115-178`) and at the trigger SQL.
7. **Persona is per bot id only.** It is keyed by Discord `bot_id` (`actor.rs:1344`). There is no per-channel or per-user persona and no versioning beyond the digest (`lanes_block.rs:62`). `custom_settings_json` is exported, but no code reads keys like `heartbeat_interval_secs` or `delegate_command` out of it. The workspace toggle comes from env instead (`omo_config.rs:322`). Hook: the profile load in `actor.rs:1347`.
8. **The workspace is a local folder, not an isolated computer.** `resolve_workspace` is a local directory under a shared root, with a shared `shared/` directory across agents (`agent_workspace.rs:76-80`, `omo_backend.rs:494`). There's no sandbox, quota or remote VM. Hook: `omo_backend.rs:481-533` (where the AgentWorkspace is provisioned).
9. **The `kibitzer_state` schema sits outside migrations** (`kibitzer.rs:178-198`), so it is invisible to the backup/migrate tables list (`db.rs:123`, `main.rs:4585`). That is a risk for moving the platform to another machine.

## EXPAND

- How the omo runtime actually stores memory under `.omo/omo.json` `memory.agent` (an external repo, code-yeongyu/oh-my-openagent). This determines whether gap 1 means building a gateway memory or bridging to omo's.
- `src/lanes/guard.rs:967` (`has_reason("memory")`): a guard decision references memory, and I didn't explore it.
- `/Users/indo/code/project/omon-gateway/src/inbox/proposals.rs:197` and `/Users/indo/code/project/omon-gateway/src/cron/health.rs:298` also fall back to the hard-coded `DEFAULT_LEAD`, so the single-owner assumption reaches beyond the kibitzer.

<results>
<files>
- /Users/indo/code/project/omon-gateway/src/memory/store.rs - MemoryStore: CRUD, lexical search, approval staging
- /Users/indo/code/project/omon-gateway/src/profiles/lanes_block.rs - bot profile export and the managed lanes prompt block
- /Users/indo/code/project/omon-gateway/bot_profiles/omomeow.json - OmOMeow persona seed, toolsets and custom settings
- /Users/indo/code/project/omon-gateway/bot_profiles/blocks/lanes.md - lanes/kibitzer/onboarding/inbox policy block
- /Users/indo/code/project/omon-gateway/src/lanes/kibitzer.rs - proactive wake engine and the memory consumer
- /Users/indo/code/project/omon-gateway/src/agent/agent_workspace.rs - per-agent workspace slug and paths
- /Users/indo/code/project/omon-gateway/src/agent/omo_backend.rs - workspace provisioning and omo memory.agent seeding (481-533)
- /Users/indo/code/project/omon-gateway/src/multiplexer/actor.rs - applies the bot profile to sessions (1344-1444), arms the kibitzer (1153-1187)
- /Users/indo/code/project/omon-gateway/src/storage/db.rs - WRITE_APPROVAL, pending_writes stage and approve
- /Users/indo/code/project/omon-gateway/src/discord/commands.rs - /memory and /reset commands
- /Users/indo/code/project/omon-gateway/src/dashboard.rs - memory listing, bot profile CRUD
- /Users/indo/code/project/omon-gateway/migrations/0001_initial.sql, 0010_pending_writes.sql, 0014_bot_profiles.sql - schemas
</files>
<answer>The gateway memory is a per-session, lexical-search SQLite store with optional write approval. The only runtime reader is the kibitzer, and the agent has no tool to write to it. Agent memory actually lives in the omo runtime, keyed per workspace slug. Persona is a bot_profiles row whose system prompt carries a CLI-managed lanes block. The kibitzer is an event-armed, paced, judge-gated proactive suggester, hard-coded to one owner's DM.</answer>
<next_steps>Ready to proceed. The highest-value hooks are a memory tool in src/tools, cross-session owner-scoped memory, memory injected into the session prompt at actor.rs:1344, and making the kibitzer lead configurable.</next_steps>
</results>
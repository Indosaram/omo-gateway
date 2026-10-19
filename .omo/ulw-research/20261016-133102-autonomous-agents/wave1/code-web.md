<!-- task=st_01a14515 -->
st_01a14515 [completed] model mahoquot/claude-opus-5-5
I read this as an information request: map the web/dashboard/usage surfaces and say where agent-platform features would plug in. I'll stop when the report is complete with file:line evidence.

<analysis>
**Literal Request**: Map capabilities of web/, src/dashboard.rs, src/dashboard_runtime.rs, src/usage.rs.
**Actual Need**: Know what the dashboard does today, what is real versus unbacked, and where an always-on autonomous-agent platform would hook in.
**Success Looks Like**: Capabilities, data model and gaps, each tied to file:line.
</analysis>

<results>
<files>
- /Users/indo/code/project/omon-gateway/src/dashboard.rs - Axum dashboard server: the full `/api` router, WebSockets, approvals, cron, bots, memory, logs, static UI.
- /Users/indo/code/project/omon-gateway/src/dashboard_runtime.rs - Standalone dashboard runtime: wires the tools, OMO backend, multiplexer, cron scheduler and approvals.
- /Users/indo/code/project/omon-gateway/src/usage.rs - Judge and inbox usage report, lane purpose tagging, `usage` CLI.
- /Users/indo/code/project/omon-gateway/web/src/App.tsx - The live single-page UI: overview, bots, chat, cron, sessions, capabilities, settings, logs.
- /Users/indo/code/project/omon-gateway/web/src/api.ts - The API client the live UI actually calls; it matches the Rust router.
- /Users/indo/code/project/omon-gateway/web/src/lib/api.ts - A large Hermes-style client whose endpoints mostly have no backend.
- /Users/indo/code/project/omon-gateway/web/src/tracker/api.ts - Linear-style tracker client calling `/api/tracker/*`.
- /Users/indo/code/project/omon-gateway/web/src/pages/BotsPage.tsx - Bot profile CRUD UI.
- /Users/indo/code/project/omon-gateway/web/vite.config.ts - Dev proxy from `/api` to 127.0.0.1:9119.
- /Users/indo/code/project/omon-gateway/src/main.rs - Combined mode: `CompositeDispatcher` at :336, `spawn_server` at :1467.
- /Users/indo/code/project/omon-gateway/migrations/0039_model_usage.sql - `model_usage` and `lane_usage_context` tables.
</files>

<answer>
## Capabilities

**Server and security**
- Axum router with about 30 routes; anything else falls through to static file serving (`src/dashboard.rs:742-786`).
- It only binds to loopback. A non-loopback host is refused because there is no transport auth (`dashboard.rs:102-115`).
- Middleware checks the Host header, requires a same-origin Origin on WebSocket upgrades, and rejects cross-origin state-changing requests (`dashboard.rs:208-272`).
- Configured from env (`DASHBOARD_ENABLED/PORT/HOST/INSECURE`, `dashboard.rs:76-100`) or CLI args (`:44-74`). The UI is served from `DASHBOARD_WEB_ROOT`, default `web/dist` (`:122-126`); if that is missing, a fallback HTML page is served (`:2529-2588`).

**Health and status**
- `/api/health`, and `/api/readiness` (also `/api/ready`), which checks the DB, workspace, disk pressure and the backend app-server (`dashboard.rs:835-899`).
- `/api/status` returns counts of sessions, messages, cron jobs and memories, plus active sessions, process memory and disk headroom (`:901-964`).

**Sessions and chat**
- Sessions can be listed (paginated, with a LIKE search), fetched and deleted. Deletion is refused if cron jobs are bound to the session (`dashboard.rs:1056-1248`).
- `POST /api/sessions/{id}/chat` builds an `InboundEvent` and routes it through `SessionMultiplexer`, with an owner-status intercept (`:1250-1297`). `/stop` stops a session (`:1299-1316`).
- The `/api/sessions/{id}/ws` WebSocket accepts `message`/`chat`/`stop`/`ping` and pushes `ready`, `accepted`, `event` (an `OutboundAction`), `warning` and `error` messages (`:1318-1485`). The UI uses it at `web/src/App.tsx:651,724`.
- A detached dashboard refuses to drive messenger sessions (`dashboard.rs:1487`).

**Cron (the proactive surface)**
- Full CRUD plus trigger, pause, resume and a runs list (`dashboard.rs:757-767`, `:1636-1920`). Edits keep the fields the scheduler owns (`:1605-1634`).
- A detached dashboard will not trigger a job that delivers to a messenger (`:1790-1803`).
- UI: `CronManagerPage` (`App.tsx:1016`).

**Approvals (human in the loop)**
- `WebDashboardDispatcher` keeps pending approvals in memory from `ApprovalRequest`/`ExpireApproval` actions, and fails closed for non-web delivery (`dashboard.rs:437-521`).
- `/api/approvals/pending` (`:2356`). `/api/approvals/{id}/resolve` accepts once, session, always or deny and goes through `SmartApprovalGuard::resolve_custom_id` (`:2374-2406`). `/api/approvals/allowlist` (`:2415`).
- In standalone mode the policy, timeout and deny globs come from env (`dashboard_runtime.rs:84-128`). UI: `App.tsx:1435`.

**Tools, skills, config**
- `/api/tools` lists the registry with JSON schemas (`dashboard.rs:1926`).
- `/api/skills` scans the skill directories on disk (`:1952-2040`).
- `/api/config` is a read-only view with secrets redacted (`dashboard_runtime.rs:302-364`).
- Standalone mode registers these tools: Terminal, File, MCP, Cron, WebSearch, WebFetch, Browser, Skills (`dashboard_runtime.rs:115-143`).

**Bots, memory, logs**
- `bot_profiles` CRUD covering model, system prompt, toolsets and custom settings (`dashboard.rs:2046-2280`; UI `pages/BotsPage.tsx:40-113`).
- `/api/memory` is a read-only, paginated LIKE search over `memories` (`dashboard.rs:2291-2351`).
- Logs: a 2000-entry ring buffer fed by a tracing layer (`:275-425`), served at `/api/logs` and `/api/logs/ws` (`:2433-2527`).

**Runtime wiring**
- `run_standalone` starts the OMO daemon supervisor, the `SessionMultiplexer` with scale-to-zero, a separate cron backend and the `CronScheduler`, which is opt-in (`dashboard_runtime.rs:65-246`).
- Combined mode fans actions out through `CompositeDispatcher`: web actions go to the dashboard, everything else to the primary (Discord) dispatcher (`dashboard.rs:524-566`, `main.rs:336`).

**Usage**
- `usage::report` counts judge calls, fallback rate, whether the cap was hit, and inbox collected/passed in a time window (`src/usage.rs:46-92`).
- It is used in a Korean digest line (`:94-104`) and the inbox digest (`src/inbox/digest.rs:99`), and exposed as the `usage --since Nh --json` CLI (`usage.rs:107-165`, `entry.rs:106,145`).
- `Purpose` enum (`usage.rs:10-35`). Lane start commit capture (`:167-192`). Session to purpose mapping (`:194-226`).

## Data model
- **Read:** `sessions`, `messages`, `cron_jobs` and `cron_runs` (`migrations/0004`), `memories` (`0001`), `approval_allowlist` (`0007`).
- **Read and written:** `bot_profiles` (`0014`).
- **usage.rs:** reads `judge_calls` (`0032`) and `inbox_items` / `inbox_filter_stats` (`0034`); reads and writes `lane_usage_context` (`0039`).
- `messenger_policy_overrides` feeds the config view only (`dashboard_runtime.rs:178-180`).
- **In-memory only:** `PendingApprovalView` (`dashboard.rs:428`) and `DashboardLogEntry` (`:275`).
- `model_usage` (`0039`) has no reader or writer anywhere in `src` (grep for `model_usage` found nothing).

## Limits & gaps (for an always-on Dots/Grok-style platform)
1. **No auth, loopback only.** `validate()` (`dashboard.rs:102-115`) and `validate_host_and_origin` (`:208`) block remote or mobile access. Hook: an auth layer before `:784` and relaxing `is_loopback_host` (`:140`).
2. **Approvals do not survive a restart and only show on the dashboard.** Pending approvals live in a `RwLock<HashMap>` (`dashboard.rs:437-447`). A detached dashboard cannot see gateway approvals (`:2354`). There are no push notifications, approval history or per-tool risk tiers. Hooks: persist in `WebDashboardDispatcher::dispatch` (`:476`); add channel fan-out through `CompositeDispatcher` (`:557`).
3. **Memory is read-only.** There is no create, edit, delete or pinning, and search is LIKE rather than semantic (`dashboard.rs:2291-2351`). Hook: add routes next to `:771`.
4. **Proactivity is cron only.** There are no event or webhook triggers, goals or background "missions". The scheduler is opt-in in standalone mode (`dashboard_runtime.rs:58-63,172-174`). The UI calls `/api/webhooks` and `/api/cron/blueprints` (`web/src/lib/api.ts:596,1063`) but no Rust route serves them. Hook: the router at `:757-767` and `AgentCronExecutor` (`dashboard_runtime.rs:162`).
5. **Connectors and channels are Discord only.** The config view only knows Discord (`dashboard_runtime.rs:278-295,333-344`), and the dashboard dispatcher rejects non-web platforms (`dashboard.rs:496-511`). `web/src/lib/api.ts` has Telegram/WhatsApp onboarding, OAuth, MCP servers, credentials and pairing (`:783-1058`), all with no backend routes. Hook: the `OutboundDispatcher` impls plus new `/api/messaging/*` routes.
6. **No "own computer" sandbox.** Tools run on the host under `OMON_WORKSPACE_ROOT` (default `~/.omon/workspace`), and `OMON_TOOL_ROOTS` defaults to `$HOME` (`dashboard_runtime.rs:248-276`). Hook: the `TerminalTool`/`BrowserTool` registration at `:115-138` is where a VM or container executor would go.
7. **No cost or token accounting in the UI.** `model_usage` is never written. `usage.rs` covers only judge and inbox metrics, has KST hard-coded (`usage.rs:63-73,229`) and has no HTTP route. Hook: add `/api/usage` at `dashboard.rs:780`, plus a writer in the backend turn completion.
8. **Config is read-only.** `get_config` returns a snapshot taken at startup (`dashboard.rs:1922`), so model, policy and env cannot be changed live, even though the UI client expects `PUT /api/config` and `/api/env` (`lib/api.ts:526-555`).
9. **The tracker UI has no backend.** `web/src/tracker/api.ts:39` calls `/api/tracker/*`, but there is no Rust route (only `src/tracker/{lanes,model,seed}.rs` exist). These calls hit the `/api/` 404 in `serve_static` (`dashboard.rs:2530`). This tracker is the natural task board for agent work.
10. **Bot profiles are not tied to runtime connections.** `runtime_connections` is only a count of tokens (`dashboard.rs:2093`, `dashboard_runtime.rs:278`), and creating a profile does not start a bot.
11. **Logs are an in-memory ring of 2000 entries** (`dashboard.rs:40,285`), so there is no durable audit trail of autonomous actions.
12. **No multi-user or tenancy model.** Web sessions are keyed with the fixed user `dashboard` (`dashboard.rs:1503`).

## EXPAND
- `web/src/lib/api.ts` plus `web/src/components/*` and most of `web/src/pages/*` look like a Hermes dashboard port. `App.tsx` only imports `./api`, `BotsPage` and the tracker (`App.tsx:53-100`), so most of these appear orphaned. I did not trace every import, so confirm before deleting or reusing.
- A `/api/tracker` backend should be built on top of `src/tracker/lanes.rs` (lane-to-issue sync).
- The `SmartApprovalGuard` and `DiscordApprovalRequester` sources, for persisting approvals.
- `migrations/0044_tracker.sql` for the tracker schema.
</answer>

<next_steps>
Highest-leverage hooks, in order: (1) an auth layer plus a non-loopback bind, (2) persistent, multi-channel approvals, (3) `/api/tracker` and webhook/event triggers wired to the scheduler, (4) a `model_usage` writer and an `/api/usage` route, (5) a sandboxed executor behind the tool registration in `dashboard_runtime.rs:115-138`. Nothing here was built or run; all findings come from reading the code and grep.
</next_steps>
</results>
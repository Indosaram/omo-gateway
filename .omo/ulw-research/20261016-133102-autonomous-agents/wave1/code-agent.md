<!-- task=st_01a14512 -->
st_01a14512 [completed] model mahoquot/claude-opus-5-5
Ask: map the agent area of omon-gateway (src/agent, src/multiplexer, src/sessions, src/models) - wanted: capabilities, data model, and the gaps between it and an always-on autonomous agent platform, each with file:line evidence. For you: the report is complete (1/1 done); one item is unverified and marked below. Now: none. Next: none.

<results>
<files>
- /Users/indo/code/project/omon-gateway/src/agent/backend.rs - The `AgentBackend` trait that every agent backend implements: `run`, `run_cancelable`, `cancel`, `steer`.
- /Users/indo/code/project/omon-gateway/src/agent/omo_backend.rs - The WebSocket client for the omo app-server: thread start/resume, turns, approvals, model fallback, workspace setup.
- /Users/indo/code/project/omon-gateway/src/agent/omo_protocol.rs - Builds the JSON-RPC frames sent to the app-server.
- /Users/indo/code/project/omon-gateway/src/agent/omo_config.rs - Backend config and model routing by purpose.
- /Users/indo/code/project/omon-gateway/src/agent/omo_daemon.rs - Spawns and supervises the local app-server daemon.
- /Users/indo/code/project/omon-gateway/src/agent/agent_workspace.rs - Gives each agent its own working directory, plus a shared one.
- /Users/indo/code/project/omon-gateway/src/agent/llm.rs - Direct LLM client; only used by inbox/main, not by the agent turn path.
- /Users/indo/code/project/omon-gateway/src/multiplexer/router.rs - `SessionMultiplexer`: admission, routing, drain, set model/yolo, reset.
- /Users/indo/code/project/omon-gateway/src/multiplexer/actor.rs - One actor per session: turn queue, steering, crash recovery, kibitzer arming.
- /Users/indo/code/project/omon-gateway/src/multiplexer/profile_routing.rs - Maps a guild/channel/thread to a model, prompt, toolsets and allowed users.
- /Users/indo/code/project/omon-gateway/src/multiplexer/gc.rs - Evicts idle actors on a timer ("scale to zero").
- /Users/indo/code/project/omon-gateway/src/multiplexer/restart_loop_guard.rs - Stops boot crash loops.
- /Users/indo/code/project/omon-gateway/src/sessions/control.rs - Controls omo sessions on other machines over SSH (read, send, interrupt, answer).
- /Users/indo/code/project/omon-gateway/src/sessions/registry.rs - Lists sessions across hosts and discovers SSH hosts.
- /Users/indo/code/project/omon-gateway/src/sessions/external.rs - Adopts an outside session as an "external" lane and drives it toward a goal.
- /Users/indo/code/project/omon-gateway/src/sessions/watch.rs - Polls watched sessions and raises question/blocked/completion notices.
- /Users/indo/code/project/omon-gateway/src/sessions/probe.rs and /Users/indo/code/project/omon-gateway/src/sessions/probe.py - Remote probe script for snapshot, interrupt and answer.
- /Users/indo/code/project/omon-gateway/src/sessions/status_reply.rs and /Users/indo/code/project/omon-gateway/src/sessions/overview.rs - Owner-DM status replies and overview formatting for Discord.
- /Users/indo/code/project/omon-gateway/src/models/session.rs, /Users/indo/code/project/omon-gateway/src/models/events.rs, /Users/indo/code/project/omon-gateway/src/models/ledger.rs - Core types: `SessionKey`, `SessionState`, `InboundEvent`, `DeliveryReceipt`.
- /Users/indo/code/project/omon-gateway/migrations/0001_initial.sql, /Users/indo/code/project/omon-gateway/migrations/0040_external_sessions.sql, /Users/indo/code/project/omon-gateway/migrations/0043_session_watch.sql - The schemas these modules read and write.
</files>

<answer>
## Capabilities

**Agent backend (omo app-server only)**
- There is one pluggable trait at the runner level: `AgentBackend::{run, run_cancelable, cancel, steer}` in `src/agent/backend.rs:33-62`.
- The direct LLM backend has been removed. `OMON_AGENT_BACKEND` only accepts `omo`, `appserver` or `omo-appserver`; `llm`, `hermes` and `direct` are rejected (`src/agent/omo_config.rs:6-30`).
- `OmoBackend` (`src/agent/omo_backend.rs:183`) keeps a JSON-RPC WebSocket open to the daemon (default `ws://127.0.0.1:19742`, `omo_config.rs:81`). It supports:
  - initialize, `thread/start` (developer instructions, model, cwd, workspace roots, enabled toolsets), `thread/resume`, `turn/start` with an effort setting, and `turn/steer` (`omo_protocol.rs:5-121`);
  - a thread binding that persists per session (`omo_backend.rs:318`, `2331-2366`).
- Turn handling in `omo_backend.rs`:
  - interactive turns are capped at 30 minutes (`:181`, `:1955`);
  - a turn fails fast after 45s with no first response (`omo_config.rs:90`);
  - interim progress reports are relayed (`:1854`);
  - if the daemon stalls, the gateway restarts the daemon it owns and retries the turn once (`:2081-2107`);
  - a transport reset gets one reconnect (`:2108`);
  - if the model is rejected, the turn falls back to `fallback_model` (`:40`, `:2054`, `:2153-2172`).
- Approvals (`omo_backend.rs:1426-1496`):
  - `enabled_toolsets` blocks command execution unless terminal/bash/shell is in the list;
  - `yolo` auto-allows;
  - by default everything else is auto-approved too, unless `APPROVAL_MODE` or `APPROVAL_POLICY` is set to `always`;
  - after 5 denials the turn is aborted (`:35`).
- Cron briefings are prepended to non-cron, non-lane prompts and marked injected only once the turn is accepted (`:803-810`, `:1411-1417`, `:1962`).
- Lane and activity items are captured for lanes (`:661`).
- Daemon supervision: `OmoDaemonSupervisor::ensure` autospawns a local daemon, takes over one owned by another omon process, and restarts with capped backoff indefinitely (`omo_daemon.rs:508-560`). `force_restart_owned_daemon` is at `:846`.
- Per-agent workspace: `agents/<slug>` plus `shared/`, with slug categories bot, cron, web, user and sys (`agent_workspace.rs:40-80`). Each one gets `.omo/omo.json` with `memory.agent=<slug>`, i.e. memory scoped per agent (`omo_backend.rs:481-538`).
- Model routing by purpose: Lead, Large, Small, LaneLarge, LaneSmall, Kibitzer, InboxWrite, Digest, JudgeFallback, CronFix map to three env tiers, `OMON_MODEL_LEAD/LARGE/SMALL` (`omo_config.rs:96-147`).

**Multiplexer**
- One actor per `SessionKey`, with a bounded channel of 64 (`router.rs:14`). Commands are Event, EventIfIdle, EventWithAck, Stop, Steer, SetModel, SetYolo, Reset, GetContext, EvictIfIdle (`actor.rs:102-146`).
- System-origin events (cron, proactive) are only admitted when the session is idle; otherwise they get "lead session is busy" (`router.rs:371-389`, `actor.rs:360-364`).
- Drain refuses new turns (`router.rs:358-366`). Shutdown marks sessions `resume_pending` (`actor.rs:905-915`), and unfinished user turns are re-dispatched at boot (`actor.rs:197-300`).
- `direct_reply` writes a delivery obligation into the ledger (`router.rs:406`).
- Idle actors are evicted after 10 minutes, with GC every 60s (`router.rs:22-28`, `gc.rs:9-45`). The restart-loop guard allows 3 restarts per 60s (`restart_loop_guard.rs:8-10`).
- Profile routes match Discord guild/channel/thread with precedence thread > channel > guild and apply model, system prompt, toolsets, profile and `allowed_users` (`profile_routing.rs:52-190`, `228-412`).
- An owner message in the lead DM arms the kibitzer (`actor.rs:1153-1187`).

**Sessions across machines**
- Hosts come from `OMON_SESSION_HOSTS` or `~/.omon/session-hosts.json`, seeded from `~/.ssh/config` (`registry.rs:37-101`, `455-477`).
- Transport is `ssh -o BatchMode=yes` with per-word quoting, or local exec (`control.rs:118-139`).
- Available operations: list, overview, read with a cursor, send (auto/steer/follow-up mode, idempotency key), interrupt, and answering pending questions (`control.rs:349-465`, `cli.rs:17-76`).
- `ExternalDriver` adopts a remote session as an `external` lane with a goal and a Done signal, drives it, and checks remotely (`external.rs:62-441`).
- `Watcher` polls every 60s and raises question, blocked and completion notices, deduplicated in SQLite (`watch.rs:15-234`).
- Owners can get a status reply over DM (`status_reply.rs:11-99`).

## Data model

| Type / table | Where | Fields |
|---|---|---|
| `SessionKey` | `models/session.rs:30` | platform, guild_id, channel_id, thread_id, user_id, bot_id; `storage_key()` round-trips |
| `SessionState` | `session.rs:268` | active_model, system_prompt, enabled_toolsets, yolo, suspended, metadata (free JSON: lane_cwd, profile, omo thread binding, fallback flag) |
| `InboundEvent` | `models/events.rs:180` | id, session, platform_message_id, delivery_id, author_id, origin (User/System), content, attachments, received_at |
| `DeliveryReceipt` | `models/ledger.rs:17` | delivery status |
| `sessions`, `messages`, `delivery_ledger`, `cron_jobs`, `memories` | `migrations/0001_initial.sql` | `state_json` holds `SessionState` |
| `lanes.kind` / `external_target`, `external_sessions`, `external_session_items` | `migrations/0040` | |
| `session_watches`, `session_watch_notices` | `migrations/0043` | |
| `resume_pending`, `model_usage`, `bot_profiles`, `approval_allowlist` | `migrations/0008`, `0039`, `0014`, `0007` | |

## Limits and gaps for an always-on personal agent platform

1. **Approvals are effectively off.** The default policy auto-approves everything (`omo_backend.rs:1438-1452`). There is no tiered human-in-the-loop path that parks a request, pings the owner, and resumes on reply; a request is either allowed or denied immediately. The hook is to replace the `auto_approve` branch at `omo_backend.rs:1438-1496` with a pending-approval record plus a dispatcher prompt. `migrations/0007_approval_allowlist.sql` exists, but this code path does not consult it.
2. **Only one cloud computer, and it is the local host.** Autospawn only works for local URLs (`omo_daemon.rs:531`, `is_local_url` at `:72`). There is no provisioning or lifecycle for remote or sandboxed VMs per user. Hooks: `OmoBackendConfig.appserver_url` is a single global value (`omo_config.rs:40`); it would need to be resolved per session/agent inside `connect_ws` (`omo_backend.rs:207`).
3. **Workspace isolation is just directories**, not a sandbox (`agent_workspace.rs:76-80`); the `shared/` dir is open to every agent. Hook: `resolve_workspace`.
4. **Proactivity is gated on idle.** System events are dropped with "busy" rather than queued (`actor.rs:360-364`), and the only proactive sources are cron, kibitzer and watch. There is no trigger or event bus for connector webhooks. Hook: `SessionMultiplexer::route` (`router.rs:358`) using `EventOrigin::System`.
5. **Multi-channel is Discord-shaped.** `ProfileRoute` matches only u64 guild/channel/thread ids (`profile_routing.rs:52-147`, `allowed_users: Vec<u64>`), and the status-reply owner logic assumes DMs and threads (`status_reply.rs:36-63`). `SessionKey.platform` is generic (`session.rs:30`), but routing and allowlists are not. Hook: generalize `match_event` (`profile_routing.rs:395`).
6. **No connector or credential model.** Tool gating is limited to toolset names, and only `commandExecution` is checked (`omo_backend.rs:1430-1445`). There is no per-user OAuth, secrets, or connector scopes passed into `thread/start` (`omo_protocol.rs:26-55`; only cwd, roots, toolsets and model are sent).
7. **Memory is delegated to omo** via `omo.json` `memory.agent` (`omo_backend.rs:514`). The `memories` table (`0001_initial.sql`) is not touched in these modules. There is no cross-session or user-profile memory injection besides cron briefings (`omo_backend.rs:1962`).
8. **Turn length is capped at 30 minutes interactive** (`:181`) and **1 hour for lanes** (`omo_config.rs:84`). Long autonomous work relies on lanes/external drivers. There is no checkpoint/continue loop in the backend itself.
9. **Model routing is static.** There are three env tiers plus one fallback (`omo_config.rs:126-147`, `:87`). It has no cost or budget awareness, even though a `model_usage` table exists (`migrations/0039`). Hook: `model_for_purpose` and the fallback block at `omo_backend.rs:2054-2172`.
10. **Cross-machine control depends on SSH plus a Python probe** with 20s probe and control timeouts (`control.rs:118`, `probe.rs:11-12`). Watch is poll-based every 60s (`watch.rs:15`), not push. There is no auth model beyond SSH keys and no web or mobile control surface in this area.
11. **The system is single-owner by design.** `owner_id()` comes from env (`status_reply.rs:36`) and `APPROVAL_MODE` is a global env var. There is no tenant/user model for multi-user hosting.
12. **One process, SQLite, in-memory actors** (`router.rs:278-289`). That is fine for 24/7 on one box with resume recovery (`actor.rs:197`), but it does not scale horizontally or fail over.

## EXPAND
- `src/lanes` (kibitzer, signals, lane lifecycle) and `src/cron` / `src/mirror.rs` hold the real proactive and autonomous machinery; the next mapping pass should cover them.
- `src/security` and `migrations/0007_approval_allowlist.sql`: check where the allowlist is actually enforced (Discord layer?). Unverified here.
- `src/memory`: whether anything uses the `memories` table.
- `src/agent/llm.rs` is only used by `src/inbox/proposals.rs:203` and `src/main.rs:1293-1303`. It could be reused for cheap routing or classification.
</answer>

<next_steps>
Use the gap list to decide build order. The cheapest high-impact hooks are the approval branch at `omo_backend.rs:1438-1496`, resolving `appserver_url` per session in `connect_ws` (`omo_backend.rs:207`), and queuing busy system events instead of dropping them at `actor.rs:360`. Map `src/lanes`, `src/cron` and `src/security` next to finish the autonomy picture.
</next_steps>
</results>
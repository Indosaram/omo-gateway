<!-- task=st_01a14516 -->
st_01a14516 [completed] model mahoquot/claude-opus-5-5
Ask: map what the omon-gateway docs and plans say the product does, where it is heading and what is unfinished - wanted: a report with file:line evidence and gaps toward a 24/7 autonomous personal agent platform. For you: the report is below. Every plan in scope is fully checked off except `discord-omo-session-control.md`, which has 10 open todos and 4 open final checks. `docs/AGENTS_DELEGATE.md` is out of date: it says settle, queued-lane retry and the kibitzer are missing, but the source has all three. Now: report delivered. Next: none.

## Capabilities
- **Discord multiplexer.** Routes by composite session key (`README.md:95`), runs many bots from one binary, evicts idle sessions to SQLite (`README.md:97`). Voice via Songbird (`README.md:98`).
- **Cron engine.** Persistent SQLite scheduler that runs agent prompts or scripts and delivers results to Discord (`README.md:99`). Script timeout defaults to 1800s (`README.md:166`).
- **Tools.** PTY terminal, file tools, Chrome CDP browser, web search, MCP over stdio and SSE (`README.md:100-104`). Tool access is limited to the workspace plus `OMON_TOOL_ROOTS` (`README.md:242`).
- **Approvals.** `APPROVAL_MODE` smart/always/never (`README.md:246`), a deny-list, a 900s timeout and Discord buttons (`README.md:105`). Wired into the terminal tool by `hermes-oneclick-migration.md:117-125`.
- **Agent engine.** OMO app-server reached over WebSocket (`README.md:194`). Each agent gets its own working folder and memory identity (`daemon-multiplexer-agent-workspaces.md:43-52`, D1-D7).
- **Hermes migration.** `migrate` with `--dry-run` and `--no-cutover` (`README.md:110-150`; plan `hermes-oneclick-migration.md`, all todos `[x]`).
- **Lanes (long-running work).** Each lane is its own app-server thread with its own folder and turn budget. CLI covers start/list/log/send/claim-done/object/close/reopen/can-launch/heavy (`docs/AGENTS_DELEGATE.md:23-112`). Done is checked by machine signals: merged, closed, published, command, file, url (`docs/AGENTS_DELEGATE.md:114-133`).
- **Event wake-ups.** Lane events are batched and wake the lead session (`dori-grade-agent-env.md:312-321`). The kibitzer is a proactive "what would the owner want" check that fires only on new events, at most 6 times a day (`:334-343`).
- **Inbox triage.** Read-only sources: mail (zele), KakaoTalk (PirateTalk), iMessage, Discord. Items are sorted owner / bot / FYI / ignore, with a 09:00 digest and one-reply approval of proposals (`:401-454`).
- **Cross-machine sessions.** List/read/send/interrupt/adopt OMO sessions over ssh (`:456-488`).
- **Cron watchdog.** Detects failed, stuck, missed and silently-empty runs when a run finishes, opens a fix lane and re-runs once to verify (`:490-522`).
- **Decision layer and models.** The jev judge (`typesafe-judge`) makes routing and yes/no calls, with daily caps and a circuit breaker. Models by purpose: Opus for the lead, DeepSeek flash for small work (`:524-545`).
- **Source confirmations.** Settle loop at `src/lanes/runner.rs:1033` and `src/lanes/settle.rs:14`. Guard and queued-lane retry at `src/lanes/runner.rs:1035-1086`. Kibitzer switch read at `src/lanes/kibitzer.rs:147`.

## Data model
- **Core:** `sessions` (`state_json` holds `omo_thread_id`; `daemon-multiplexer-agent-workspaces.md:55`), `messages`, `bot_profiles` (`migrations/0014`), `cron_jobs`/`cron_runs`/`cron_outputs`/`cron_incidents` (`0004`, `0021`, `0026`), `delivery_obligations` (`0006`), `approval_allowlist` (`0007`).
- **Lanes:** `lanes`, `lane_history`, `lane_events`, `lane_transcript` (`migrations/0031_lanes.sql`; DDL in `dori-grade-agent-env.md:202-211`), plus `lane_inbox` (`0033`), `0037`, `0038`.
- **Judge and usage:** `judge_calls` (`0032`), `model_usage` (`0039`). Token accounting was dropped by the owner (`dori-grade-agent-env.md:547`).
- **Inbox:** `inbox_items`, `inbox_decisions`, `inbox_cursors`, `inbox_filter_stats` (`0034`; spec `:401`).
- **Cron health:** `cron_problems` (`0035`), `0041_cron_remediation`.
- **Sessions:** `0040_external_sessions` (adds `external_target`), `0043_session_watch`.
- **Legacy:** `work_items` (`0030`). It was the old dead lane scaffold (`gateway-autonomy-lanes.md:11`), superseded by `lanes`.
- **Tracker:** `0044_tracker` (`linear-tracker.md`).

## Limits & gaps (vs Dots / Grok Bot / Muse-class always-on agent)
1. **No own cloud computer.** Everything runs on one Mac under launchd (`README.md:198-205`), and the Docker image still needs a host `omo app-server` (`README.md:194`). Workspace separation is explicitly not a security sandbox (`daemon-multiplexer-agent-workspaces.md:12`, `:47`). Extension point: `AgentWorkspace` resolution in `resolve_thread_id` (`src/agent/omo_backend.rs:137-259`, cited at plan `:50`) could hand out remote VM or container cwds.
2. **Single owner, single tenant.** Owner id, bot id and session hosts are hardcoded (`dori-grade-agent-env.md:401` owner `414011632306618368`; `:169`). There are no user accounts, so serving more than one person needs a tenant key on `SessionKey` (`src/models/session.rs:8-20`).
3. **Discord is the only chat surface.** Mail, Kakao and iMessage are read-only by design (`dori-grade-agent-env.md:150-151`). No Slack, Telegram or web chat beyond the dashboard (`:146`). Extension point: an adapter beside `src/discord/adapter.rs` that feeds `SessionMultiplexer::route` (`src/multiplexer/router.rs:312`).
4. **Connectors are CLI shell-outs, not OAuth.** zele, imsg, PirateTalk and gh (`dori-grade-agent-env.md:412-421`). Auth expiry only produces a daily alert (`:412`). No connector registry; extension point is the `InboxSource` trait (`src/inbox/mod.rs`, spec `:401`).
5. **Approvals cover shell commands only.** No general action-approval queue for outbound messages, payments or publishing. Inbox proposals use a separate "3 해" reply path (`dori-grade-agent-env.md:445-454`), and cron publishing uses `.ready` files (`:501`). Extension point: generalize `SmartApprovalGuard` (`src/discord/approval.rs:41-90`).
6. **Memory is per-agent but shallow.** It is keyed off the cwd via OMO memory (`daemon-multiplexer-agent-workspaces.md:48`). The kibitzer only does `MemoryStore::search("owner preferences")` (`dori-grade-agent-env.md:334`; `src/memory/store.rs:97`). No user-facing view or edit of memory, and multi-device memory sync is out of scope (`:13`).
7. **Proactivity is tightly capped.** At most 6 kibitzer wakes a day, only 08:00-24:00 KST, no timer, nothing without a new event (`dori-grade-agent-env.md:334`). Goals the agent sets for itself or long-horizon planning would hook into `src/lanes/kibitzer.rs`.
8. **Session control is unfinished.** `discord-omo-session-control.md:166-265` is all unchecked. Gaps: no `/sessions` slash command (`src/discord/commands.rs:178`), interrupt calls a command that does not exist, read/send fail outside the repo because `--all-scope` is missing, and questions cannot be answered remotely (plan `:41-50`).
9. **Docs lag the code.** `docs/AGENTS_DELEGATE.md:69,155,163,185` says settle, the kibitzer and queued retry are absent, but the source has them (`src/lanes/runner.rs:1033-1093`, `src/lanes/kibitzer.rs:147`). The README also does not mention lanes, inbox, jev or the session console. A lane dashboard view is still missing (`docs/AGENTS_DELEGATE.md:163`; listing would use `/api/sessions`, `src/dashboard.rs:720-728`).
10. **Owner-only guardrails block autonomy.** Owner-only steps remain. No `.env` edits, no auto-commit, no live deploy without the owner (`discord-omo-session-control.md:41`, IS-9), so the system cannot redeploy itself.
11. **Two-tier models with a fixed choice.** The model set is pinned (`dori-grade-agent-env.md:142`). There is no fallback chain beyond `DEFAULT_FALLBACK_MODEL` and no cost dashboard, since token accounting was removed (`:547`).

## EXPAND
- `.omo/plans/omo-appserver-backend.md` and `.omo/plans/linear-tracker.md` are outside this scope and not mapped; they cover the backend protocol and the dashboard issue tracker.
- `docs/research/omo-dori-mode-analysis.md` was not read; the critique (`docs/research/omo-dori-mode-critique.md`) argues against herdr and long silences and keeps only the done protocol.
- `.omo/evidence/dori-grade-agent-env/REPORT.md` should list each IS row's evidence and would confirm the deploy really happened (`dori-grade-agent-env.md:569-577`). Not read.

<results>
<files>
- /Users/indo/code/project/omon-gateway/README.md - product pitch, features, config surface, migration
- /Users/indo/code/project/omon-gateway/docs/AGENTS_DELEGATE.md - lane operator guide (stale on settle, kibitzer, retry)
- /Users/indo/code/project/omon-gateway/docs/PHASE4_GOD_SESSION_RETIREMENT.md - legacy god-session retirement and rollback
- /Users/indo/code/project/omon-gateway/.omo/plans/dori-grade-agent-env.md - main autonomy roadmap: 37 todos all done, IS-1..19, GAP-1..19
- /Users/indo/code/project/omon-gateway/.omo/plans/gateway-autonomy-lanes.md - original lanes and split-brain diagnosis (superseded)
- /Users/indo/code/project/omon-gateway/.omo/plans/daemon-multiplexer-agent-workspaces.md - per-agent cwd and memory identity (done)
- /Users/indo/code/project/omon-gateway/.omo/plans/hermes-oneclick-migration.md - migrate command and APPROVAL_MODE (done)
- /Users/indo/code/project/omon-gateway/.omo/plans/omomeow-gateway-migration.md - OmOMeow cutover, SLA watch, work_items (done)
- /Users/indo/code/project/omon-gateway/.omo/plans/discord-omo-session-control.md - only plan with open todos
- /Users/indo/code/project/omon-gateway/src/lanes/runner.rs - settle and guard loops (lines 1033-1093)
- /Users/indo/code/project/omon-gateway/src/lanes/kibitzer.rs - proactive wake logic (line 147)
</files>
<answer>The plans describe a single-owner, Mac-hosted, Discord-first personal agent. It already has delegated long-running work (lanes), machine-checked done, event-driven proactivity, read-only inbox triage, a cross-machine session console and a cron self-repair loop. Only discord-omo-session-control remains open. Compared with a Dots / Grok Bot / Muse-class platform, the main gaps are: no hosted or sandboxed compute, one tenant, Discord as the only chat surface, CLI-based connectors, approvals limited to shell commands, shallow memory, tightly capped proactivity, and docs that lag the code.</answer>
<next_steps>Use the gap list's extension points to scope work. Read omo-appserver-backend.md and the evidence REPORT.md if you need backend and deploy confirmation.</next_steps>
</results>
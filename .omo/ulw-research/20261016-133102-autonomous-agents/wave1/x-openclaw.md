<!-- task=st_01a14501 -->
st_01a14501 [completed] model mahoquot/claude-sonnet-5-5
I read this as an information ask on OpenClaw, Hermes Agent and similar gateways, so the report below is research only with no edits. The `web_search` tool wasn't available this turn, so I used `gh`, `curl` on the official docs and READMEs, and the HN Algolia API. I didn't read the Reddit or X sources.

## Identity

- **OpenClaw** (ex-Clawdbot, then Moltbot) is an MIT-licensed, TypeScript, self-hosted assistant gateway at https://github.com/openclaw/openclaw, with docs at https://docs.openclaw.ai. Its site is https://openclaw.ai.
  - The repo was created 2025-11-24. The "Clawd" name dates from November 2025, and the rename to OpenClaw was announced 2026-01-29 by Peter Steinberger.
  - It is now stewarded by the OpenClaw Foundation, a 501(c)(3). OpenAI is a donor, not an owner.
  - It had about 392k stars on 2026-10-16.
  - Confidence: high. [https://openclaw.ai/blog/introducing-openclaw] [https://github.com/openclaw/openclaw/blob/fd2d5d0ae8ca4fa7eb8f4ce4fddb03508713160e/README.md]
- **Hermes Agent** is built by Nous Research. It is MIT-licensed Python at https://github.com/NousResearch/hermes-agent, with docs at https://hermes-agent.nousresearch.com/docs/.
  - The repo was created 2025-07-22 and had about 254k stars on 2026-10-16.
  - Confidence: high. [https://github.com/NousResearch/hermes-agent/blob/b65e078cdc77ebe83979a2d5355179535f340e89/README.md]
- **Others** are far smaller. AstrBot is Python, about 42k stars, and calls itself an "openclaw alternative". nexo-rs is a Rust multi-agent framework with 8 stars, and GoGogot is a Go agent with 262 stars. [https://github.com/AstrBotDevs/AstrBot] [https://github.com/lordmacu/nexo-rs] [https://github.com/aspasskiy/GoGogot]

## Findings

**OpenClaw architecture**
- One long-lived Gateway process owns all messaging surfaces: WhatsApp, Telegram, Slack, Discord, Signal, iMessage and WebChat.
  - Clients (CLI, macOS app, web UI) connect to it over a typed WebSocket on 127.0.0.1:18789 by default.
  - Nodes (macOS, iOS, Android, headless) connect over the same socket with `role: node` and expose device commands such as camera and location. [https://docs.openclaw.ai/concepts/architecture.md]
- The wire protocol requires a `connect` first frame and a signed challenge nonce.
  - Side-effecting methods (`send`, `agent`) require idempotency keys, and the server keeps a short-lived dedupe cache.
  - New device IDs need pairing approval, and loopback connects can be auto-approved. [https://docs.openclaw.ai/concepts/architecture.md]
- A single `GatewayScheduler` drives maintenance and cron wakeups from one host timer.
  - After sleep, each due job fires once and missed ticks are coalesced.
  - It uses wall time for deadlines and elapsed time for relative delays, so a clock correction doesn't skew cadences. [https://docs.openclaw.ai/concepts/architecture.md]
- Supervision is by launchd or systemd, and the docs list "Events are not replayed" as an invariant. [https://docs.openclaw.ai/concepts/architecture.md]

**OpenClaw heartbeat**
- A heartbeat is a periodic agent turn in the main session. The default cadence is 30m, or 1h under Anthropic OAuth auth.
  - The default prompt is narrow: do not infer old tasks, and reply `NO_REPLY` if nothing needs attention.
  - Cadence is now owned by the cron scheduler as a system job. [https://docs.openclaw.ai/gateway/heartbeat.md]
- Heartbeat alerts go to the owner's DM by default (`target: owner`, resolved from `commands.ownerAllowFrom`) and never to a group unless `target: "last"` is set.
  - `activeHours` with a timezone restricts when it runs. [https://docs.openclaw.ai/gateway/heartbeat.md]
- Cost controls: `isolatedSession` and `lightContext`, which the docs say cut roughly 100K tokens to 2-5K per run, plus a cheaper per-heartbeat `model`.
  - The turn is skipped with no model call when the scratch file is effectively empty or the delivery channel is down.
  - Channel-down retries back off from 1 minute, doubling up to the heartbeat interval. [https://docs.openclaw.ai/gateway/heartbeat.md]
- The agent can answer with a structured `heartbeat_respond` tool (`notify`, `notificationText`, `scratch`) instead of a text sentinel, and can rewrite its own checklist. [https://docs.openclaw.ai/gateway/heartbeat.md]
- Event wakes, such as a background exec finishing, are rate-limited to 30s apart with a flood guard of 5 starts per 60s. Heartbeats defer while the session or main queue is busy. [https://docs.openclaw.ai/gateway/heartbeat.md]
- Recurring work is deliberately moved out of heartbeat into "automations" jobs. The old `HEARTBEAT.md` file is migrated by `openclaw doctor --fix`. [https://docs.openclaw.ai/gateway/heartbeat.md]

**OpenClaw cron, webhooks, queue and lanes**
- Automations (`openclaw cron` is an alias) cover one-shot (`--delete-after-run`), recurring and cron schedules. Payload types are agent turn, command and script.
  - Session styles are main, current, isolated or custom, and delivery can go to chat, a webhook or nowhere.
  - There are failure notifications, inbound `POST /hooks/wake` and `/hooks/agent`, Gmail Pub/Sub triggers, and event-conditioned "standing intents". [https://docs.openclaw.ai/automation/cron-jobs.md]
- The inbound queue is a lane-aware FIFO.
  - Per-session lane `session:<key>` guarantees one run per session.
  - A global `main` lane is capped by `agents.defaults.maxConcurrent`.
  - Separate `cron`, `cron-nested` and `subagent:<session>` lanes stop background work blocking inbound replies. [https://docs.openclaw.ai/concepts/queue.md]
- Queue modes are `steer` (the default), `followup`, `collect` and `interrupt`, with a 500ms debounce, `cap: 20` and `drop: summarize`.
  - Priority has a bounded head start of 15s for foreground and 30s for background, so older work can't be starved. [https://docs.openclaw.ai/concepts/queue.md]
- Inbound input is stored durably before the gateway acknowledges it.
  - If the gateway dies, the in-memory queue is not replayed and the input shows as "interrupted" for an explicit resend.
  - Stuck sessions are classified as long_running, stalled or stuck, and recovery attempts one continuation before telling the user to retry. [https://docs.openclaw.ai/concepts/queue.md]

**OpenClaw iMessage and sandbox**
- iMessage uses the external `imsg` CLI over JSON-RPC on stdio, run on the signed-in Messages Mac.
  - BlueBubbles support was removed.
  - Private-API features (tapbacks, effects, polls) need `imsg launch`, and the docs describe enabling them with SIP disabled, though some actions remain available with SIP on. [https://docs.openclaw.ai/channels/imessage.md]
- iMessage inbound recovery after a restart is automatic. It replays missed messages and suppresses Apple's stale "backlog bomb" after Push recovery. A dedicated bot macOS user and Tailscale remote-Mac deployments are documented. [https://docs.openclaw.ai/channels/imessage.md]
- Sandboxing is off by default. When on, the Gateway stays on the host and only tool execution moves into Docker, Podman, SSH, OpenShell or Crabbox backends. `tools.elevated` is the escape hatch.
  - Tool allow/deny policy applies before sandbox rules.
  - The docs say plainly that it is "not a perfect security boundary". [https://docs.openclaw.ai/gateway/sandboxing.md]
- DM-capable channels pair unknown senders by default. Tools run on the host for the main session unless sandboxing is configured. [https://github.com/openclaw/openclaw/blob/fd2d5d0ae8ca4fa7eb8f4ce4fddb03508713160e/README.md]

**OpenClaw memory**
- Memory is plain Markdown in the workspace: `USER.md`, `MEMORY.md`, daily `memory/YYYY-MM-DD.md` and a review file `DREAMS.md`.
  - A SQLite hybrid index (vector plus keyword) backs `memory_search` and `memory_get`.
  - A silent "memory flush" turn runs before compaction. [https://docs.openclaw.ai/concepts/memory.md]
- "Dreaming" is a scheduled background consolidation job that promotes only threshold-passing, non-untrusted items into `MEMORY.md`. [https://docs.openclaw.ai/concepts/memory.md]
- It can import Codex, Claude Code and Hermes memory files, and the README says skills are shared through the ClawHub registry. [https://docs.openclaw.ai/concepts/memory.md] [https://github.com/openclaw/openclaw/blob/fd2d5d0ae8ca4fa7eb8f4ce4fddb03508713160e/README.md]

**Hermes Agent**
- One gateway process serves Telegram, Discord, Slack, WhatsApp, Signal and Email plus a CLI/TUI. The README lists no iMessage, which is a gap against OpenClaw. [https://github.com/NousResearch/hermes-agent/blob/b65e078cdc77ebe83979a2d5355179535f340e89/README.md]
- Its pitch is a closed learning loop. The agent creates and improves skills itself and curates its own memory. It also has FTS5 cross-session search with LLM summarization and Honcho user modeling, and follows the agentskills.io skill standard. [https://github.com/NousResearch/hermes-agent/blob/b65e078cdc77ebe83979a2d5355179535f340e89/README.md]
- It has seven terminal backends: local, Docker, SSH, Singularity, Modal, Daytona and Vercel Sandbox. [https://github.com/NousResearch/hermes-agent/blob/b65e078cdc77ebe83979a2d5355179535f340e89/README.md]
- It ships `hermes claw migrate` to import OpenClaw's SOUL.md, memories, skills, command allowlist and allowlisted API keys. [https://github.com/NousResearch/hermes-agent/blob/b65e078cdc77ebe83979a2d5355179535f340e89/README.md]
- Cron is managed through one `cronjob_manage` tool and can attach multiple skills per job.
  - Model resolution is per-job pin, then `cron.model`, then the main model, and the agent cannot repoint a job's model.
  - A "no-agent" mode runs a script on a schedule and delivers its stdout with zero LLM calls.
  - Webhook routes can fire a cron job on external events. [https://hermes-agent.nousresearch.com/docs/user-guide/features/cron]
- Preflight validation blocks a cron run before any model call if the provider key, skills, delivery target or MCP servers are broken. It sends one alert and does not repeat it every tick. [https://hermes-agent.nousresearch.com/docs/user-guide/features/cron]
- Cron sessions cannot create more cron jobs. Delivery interrupted mid-send is marked "unknown" and is not auto-retried, so there are no duplicates. [https://hermes-agent.nousresearch.com/docs/user-guide/features/cron]
- Its `/heartbeat every 10m <prompt>` is a per-session recurring prompt that fires only when the session is idle.
  - Missed ticks coalesce into one turn and queued user messages win.
  - It supports a silent `NO_REPLY` or `[SILENT]` marker and has a "don't invent work" guard.
  - State lives in SessionDB and is cleared when the conversation resets. [https://raw.githubusercontent.com/NousResearch/hermes-agent/b65e078cdc77ebe83979a2d5355179535f340e89/website/docs/user-guide/features/heartbeat.md]

**Ecosystem risks**
- Anthropic stopped subscription limits covering third-party harnesses including OpenClaw from 2026-04-04, then HN reported on 2026-04-21 that Claude CLI usage in OpenClaw style was allowed again. [https://news.ycombinator.com/item?id=47633396] [https://news.ycombinator.com/item?id=47844269]
- Google restricted AI Pro/Ultra accounts that used OpenClaw OAuth. [https://news.ycombinator.com/item?id=47115805]
- OpenClaw has had security criticism, including a privilege-escalation CVE (CVE-2026-33579) and a "security nightmare" post, both on HN. [https://nvd.nist.gov/vuln/detail/CVE-2026-33579] [https://composio.dev/content/openclaw-security-and-vulnerabilities]
- I found no KakaoTalk integration in OpenClaw or Hermes through GitHub repo search. My search was shallow, so treat that as unconfirmed.

## Implications for a Rust Mac daemon (Discord, iMessage, KakaoTalk, omo agents, cron, inbox, lanes)

1. **Heartbeat as a cheap pre-gate.** Skip the model entirely when the checklist is empty or the target channel is down. Use an isolated session with light context, and an active-hours window. OpenClaw's numbers suggest the saving is large.
2. **Structured heartbeat result.** Use a tool-call result (`notify` or not, text, new scratch) instead of a text sentinel. Treat a bare silent marker as no output.
3. **Lane model.** Use per-session serial lanes, a bounded global inbound lane, and separate cron and omo-agent lanes so long coding runs never block replies to the owner.
   - Add bounded priority aging so older work can't be starved.
   - Offer `steer`, `followup` and `interrupt` for messages arriving mid-run.
4. **Crash-safe inbox.** Persist each inbound message before acknowledging it, and replay iMessage messages missed while the daemon was down, suppressing Apple's stale backlog.
   - Keep dedupe keys and use idempotency keys on side-effecting sends.
   - Mark an interrupted delivery "unknown" and never auto-resend it, as Hermes does.
5. **Cron preflight and no-agent jobs.** Validate credentials, delivery target and skills before spawning a run, alert once, and let a job be a plain script whose output is delivered with no LLM.
   - Block recursive job creation from inside a cron run.
   - Coalesce missed ticks after sleep, using wall time for deadlines and monotonic time for relative delays (the Mac will sleep).
6. **Owner-only default delivery.** Proactive messages go to the owner's DM only, never to a group, unless explicitly overridden. Keep default-deny DM pairing for unknown senders.
7. **Sandbox honestly.** The docs call OpenClaw's own sandbox imperfect, and its default is off. For omo agents, keep the Rust daemon on the host and run agent execution as a separate low-privilege macOS user or VM, with no credentials injected by default. That is the pattern OpenClaw's iMessage docs describe for a bot user.
8. **Memory.** Plain Markdown plus a SQLite index is enough to start. If you add consolidation, gate it so untrusted sources (inbound messages, ingested inbox mail) can't be promoted into durable memory.

## CLAIMS
- CLAIM: OpenClaw is the renamed Clawdbot/Moltbot project, MIT-licensed and stewarded by a 501(c)(3) foundation, with about 392k stars. — RISK: normal — SOURCES: openclaw.ai, github.com — PRIMARY: https://github.com/openclaw/openclaw/blob/fd2d5d0ae8ca4fa7eb8f4ce4fddb03508713160e/README.md
- CLAIM: Hermes Agent (Nous Research, MIT, Python) supports Telegram, Discord, Slack, WhatsApp, Signal and Email, but its README lists no iMessage. — RISK: normal — SOURCES: github.com — PRIMARY: https://github.com/NousResearch/hermes-agent/blob/b65e078cdc77ebe83979a2d5355179535f340e89/README.md
- CLAIM: OpenClaw's heartbeat defaults to 30m, delivers to the owner DM, and replies `NO_REPLY` when nothing needs attention. — RISK: normal — SOURCES: docs.openclaw.ai — PRIMARY: https://docs.openclaw.ai/gateway/heartbeat.md
- CLAIM: OpenClaw's iMessage channel needs private-API setup with SIP disabled for full features, and some actions remain available with SIP on. — RISK: high — SOURCES: docs.openclaw.ai — PRIMARY: https://docs.openclaw.ai/channels/imessage.md (the SIP specifics are on a sub-page I did not fetch)
- CLAIM: OpenClaw sandboxing is off by default, the Gateway stays on the host, and the docs call it not a perfect security boundary. — RISK: high — SOURCES: docs.openclaw.ai — PRIMARY: https://docs.openclaw.ai/gateway/sandboxing.md
- CLAIM: Hermes cron runs a preflight and blocks a misconfigured job before any LLM call. — RISK: normal — SOURCES: hermes-agent.nousresearch.com — PRIMARY: https://hermes-agent.nousresearch.com/docs/user-guide/features/cron
- CLAIM: Anthropic's April 2026 third-party-harness billing change applied to OpenClaw, then reportedly reversed for Claude CLI usage. — RISK: high — SOURCES: news.ycombinator.com — PRIMARY: https://news.ycombinator.com/item?id=47844269 (secondary; the original Anthropic email and policy page were not fetched)
- CLAIM: No OpenClaw or Hermes KakaoTalk support was found. — RISK: normal — SOURCES: github.com — PRIMARY: none (one GitHub repo search returned an empty list)

## EXPAND
- LEAD: OpenClaw `imsg` repo and its JSON-RPC contract, as a possible reference for or dependency of an iMessage bridge. — WHY: it is the most mature macOS iMessage bridge and the RPC protocol is documented at /reference/rpc. — ANGLE: `gh search code` or `gh repo view` on the imsg repo, plus https://docs.openclaw.ai/reference/rpc.md.
- LEAD: Hermes Kanban worker lanes and multi-gateway docs. — WHY: this is the closest analogue to "lanes plus omo coding agents". — ANGLE: fetch the `kanban-worker-lanes.md` and `kanban-multi-gateway.md` docs from the pinned hermes-agent commit.
- LEAD: OpenClaw standing intents and the Gmail Pub/Sub reader boundary. — WHY: it is the closest prior art for inbox ingestion with an untrusted-input trust boundary. — ANGLE: https://docs.openclaw.ai/concepts/standing-intents.md and https://docs.openclaw.ai/automation/cron-jobs/gmail.md.
- LEAD: Rust OpenClaw alternatives (nexo-rs, others). — WHY: their code could show Rust patterns for a gateway. — ANGLE: `gh search repos --language rust` and read the architecture docs; all are very low-star.
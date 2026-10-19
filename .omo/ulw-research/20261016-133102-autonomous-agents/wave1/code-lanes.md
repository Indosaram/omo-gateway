<!-- task=st_01a1450d -->
st_01a1450d [completed] model mahoquot/claude-opus-5-5
Ask: map what lanes can do today in omon-gateway (src/lanes, work_heartbeat.rs, drain_control.rs) - wanted: a report with file:line evidence covering capabilities, the data model, and the gaps against an always-on personal agent platform. For you: the report is done (1/1). Everything below comes from reading the source; I didn't build or run anything. Now: final report. Next: none.

<analysis>
**Literal Request**: Map lane capabilities, the data model, and the gaps relative to Dots/Grok Bot/Muse-style agents.
**Actual Need**: Know which autonomy primitives already exist and where to extend them.
**Success Looks Like**: Every claim has a file:line reference, and every gap names where new code would plug in.
</analysis>

## Capabilities

- **CLI surface**: `omon lane start|list|log|send|claim-done|object|close|reopen|register-worktree|can-launch|heavy` (src/lanes/cli.rs:24-76). It requires an explicit `DATABASE_URL` (cli.rs:109).
- **Start flow**:
  - The key is validated as 1-48 chars of `[a-z0-9-]`, done-signals are parsed, and cwd and brief must exist (cli.rs:281-318).
  - The lead session comes from `--lead`, then the newest session in the thread, then `OMON_LANE_DEFAULT_LEAD` (cli.rs:355-371).
  - With `--model auto`, a judge picks small or large from the first 2000 chars of the brief (cli.rs:373-383).
  - The Discord thread is created before the DB transaction (cli.rs:397-420). The cap is re-checked under `BEGIN IMMEDIATE`, and a lane over the cap is stored as `queued` (cli.rs:425-441). If the thread isn't kept, it is archived (cli.rs:457-468).
- **State machine**: 8 states with an allowed-from table. Closing from `working` is only legal with a `forced:` reason (src/lanes/state.rs:9-77). `transition()` is a compare-and-swap through `storage::lanes::transition_lane` (state.rs:132-185).
- **Runner loop**:
  - `LaneRunner::serve` (src/lanes/runner.rs:1018-1159) creates close intents for `verified_done` lanes, applies owner replies, refreshes Discord statuses, and spawns one task per `working`/`not_done` lane.
  - On each loop it also samples host guards every 60 s, settles every 30 s, and wakes on a Unix datagram socket (runner.rs:712, 1114). The socket only carries a wake hint; the DB is authoritative (runner.rs:696).
- **Turn execution** in `run_lane` (runner.rs:1167+):
  - A turn cap of 24 (runner.rs:30, 1241) and a limit of 3 consecutive errors, after which the lane is marked dead (1246).
  - Escalation from the small to the large model after 2 `not_done` results (1253-1277).
  - The first prompt is the brief plus the control executable and DB URL (1293-1310). Inbox messages are steered into a running turn (1388-1410).
  - A quiet check every 60 s (1366) posts a "stale" notice after 20 minutes of no activity (activity.rs:303-326).
- **Agent reporting**: the agent writes `[REPORT] key|blocker|question|done|milestone|text` lines (src/lanes/activity.rs:53-70). A turn with no reports is classified by a judge as finished, progress, or blocked (activity.rs:241-279).
- **Output capture**: `CapturingDispatcher` writes agent output to `lane_transcript` (runner.rs:605-690). It rejects `ApprovalRequest` with "lane approval requires lead intervention" (runner.rs:661-665).
- **Completion check** in `Settler::settle_due` (src/lanes/settle.rs:47-176):
  - It waits 5 minutes after the claim, then writes a `settlement_started` marker so an interrupted run is never replayed.
  - It then runs remote checks, the done signals (or a cron override), and worktree checks, and has a judge review the diff (`done_diff`, settle.rs:179).
  - Pass leads to `verified_done`. Fail leads to `not_done` plus a `lane_events` row and a `[LEAD] not done` inbox message.
- **Done-signal types**: `merged`, `closed`, `published`, `command` (argv plus a stdout regex), `file` (sha256 or a JSON path), `url` (status plus a body regex) (src/lanes/signals.rs:25-59; parser at 179).
- **Close and reopen**:
  - A close intent has phases: thread effect, then removal of owned worktrees, then a CAS write to `closed` (runner.rs:832-895).
  - Reopening goes through `lane_owner_replies` with generation fencing (runner.rs:897+; storage/lanes.rs:243-263). A Discord reply in a closed or verified thread triggers a reopen or close.
  - Most writes are fenced on `MAX(lane_history.id WHERE reason='reopened')`.
- **Lead notification**: `EventDispatcher` scans every 5 s (src/lanes/dispatch.rs:30) and groups events per lead.
  - It waits 30 s to batch and allows at most one delivery per 30 s per lead.
  - It only delivers when the lead session is idle, and a judge filters out non-critical events.
  - Events are injected as a `[LANE_EVENT]` system turn (dispatch.rs:82-198). Critical kinds are listed at dispatch.rs:211.
  - It pauses while the gateway is draining (dispatch.rs:48).
- **Proactive "kibitzer"**: SQL triggers on inbox items and lane events arm a generation counter (src/lanes/kibitzer.rs:180-198). Evaluation (242-364):
  - At most once per 30 min, 6 times a day, and never before 08:00 KST.
  - A judge must score at least 0.7, then a tool-less model turn returns JSON `{wake, reason, suggested, target_lane}`, which becomes a `kibitzer` lane event.
  - The model's context is the home DM, open lanes, recent events, and memory search results (376-428).
- **Host guard**: thresholds for load, free memory, free disk, max lanes (6), and heavy slots (1) (src/lanes/guard.rs:10-15). `can_launch` is at 443 and `retry_queued` at 671, which promotes queued lanes with a 10-minute backoff.
- **Discord threads**: thread names carry a status prefix (🔄/⏸/✅) (src/lanes/discord_threads.rs:22-66). Posts use a durable outbox with pending, sending, sent, uncertain and failed states, nonces, and no blind retries (src/lanes/posts.rs:15-66). Secrets are scrubbed before anything goes out (src/lanes/scrub.rs:58).
- **Work heartbeat**: separate from lanes. It finds stale `in_progress` work items, checks whether the delegate pane is alive, and posts to the item's thread or a configured alert channel (src/work_heartbeat.rs:270-341).
- **Drain control**: a `.drain_request.json` marker fenced by boot epoch, with a 1 h TTL. An unreadable marker counts as draining (src/drain_control.rs:96-138, 210-237). `DrainWatcher` publishes the state on a watch channel (241-300).

## Data model

| Table | Source | Role |
|---|---|---|
| `lanes` | migrations/0031_lanes.sql:1-25 | key, title, lead_session_key, thread/parent channel, cwd, brief_path, model, status CHECK, done_signals, claim_*, objection, turns_run, last_report, last_activity_at, stale_posted_at |
| `lane_history` | 0031:27 | Transition audit; also generation and settlement markers |
| `lane_events` | 0031:36, 0037 | Outbox to the lead (`delivered_at`, `delivery_reason`) |
| `lane_transcript` | 0031:45 | Messages from user, agent, and thread posts (posts are stored as JSON) |
| `lane_inbox` | 0033 | Steering messages for the lane |
| `lane_thread_bindings`, `lane_close_intents`, `lane_owned_worktrees`, `lane_owner_replies` | 0038_lane_lifecycle.sql | Thread ownership, close phases, worktree registry, reopen replies |
| `kibitzer_state` + triggers | kibitzer.rs:182-191 (created at runtime, not a migration) | Arming generation |
| `lane_usage_context`, `sessions`, `messages`, memory | runner.rs:1283, cli.rs:478, kibitzer.rs:383-389 | Shared state |
| Work items | storage `get_stale_in_progress_work_items` (work_heartbeat.rs:11) | Heartbeat source |

Rust types: `LaneStatus` (state.rs:9), `DoneSignal` (signals.rs:25), `Decision` (kibitzer.rs:51), `Post`/`PostState` (posts.rs:15,32), `DrainRequest` (drain_control.rs:17), `WorkHeartbeatConfig` (work_heartbeat.rs:14).

## Limits and gaps for an always-on personal agent platform

1. **No triggers for lane creation.** Lanes are only created via CLI `start` (cli.rs:320). The kibitzer only suggests, and `target_lane` is advisory (kibitzer.rs:340-361). To let it auto-launch a lane, map `Decision` to `cli::start` (with a guard check) after kibitzer.rs:361. For time, webhook, or connector triggers, add a producer that writes lanes and calls `runner::wake` (runner.rs:712).
2. **Approvals are hard-blocked inside lanes.** `ApprovalRequest` returns an error (runner.rs:661); the kibitzer does the same (kibitzer.rs:94). Hook point: route it into a `lane_events` row of a new kind `approval`, switch the lane to `blocked`, and resume through `lane_inbox`. `DiscordApprovalRequester` already exists (src/discord/approval.rs:196).
3. **Single owner, hardcoded.** The kibitzer lead is pinned to specific Discord user, bot and channel IDs (kibitzer.rs:21, 122-133). KST and an 08:00 quiet window are constants (kibitzer.rs:24, 230). Multi-user use needs per-user config at `Kibitzer::new`/`from_env` (115-178).
4. **Discord is the only output channel.** Posts and statuses go only through `DiscordThreads` (posts.rs:91-160). Thread creation happens at cli.rs:397 and guard.rs:607. Adding other channels means another `PostSink` implementation plus a channel field on `lanes` (currently `thread_channel_id`, 0031:5).
5. **One machine, no sandbox.** cwd is a local directory, the guard samples only the local host (guard.rs:315-428), and the wake path is a local Unix socket (runner.rs:697). There is no remote or cloud computer. The extension point is `sessions::external::ExternalDriver`, which `run_lane` already branches to (runner.rs:1172-1181).
6. **Hard turn and time budgets.** 24 turns (runner.rs:30) and at most 6 kibitzer wakes a day (kibitzer.rs:217, 352) rule out 24/7 lanes that run for weeks. There is no resume-after-cap except a reopen (runner.rs:1241).
7. **Completion checks are dev-oriented.** The signals cover PRs, issues, packages, commands, files and URLs (signals.rs:25). There is nothing for calendar, email or purchase outcomes. New `DoneSignal` variants go in signals.rs:25 and the parser at 276.
8. **Memory is read-only from lanes.** The kibitzer only searches "owner preferences" (kibitzer.rs:387-389). Nothing writes lane outcomes back to memory. A natural write hook is the settle success path (settle.rs:171-173) or close completion (runner.rs:875).
9. **No connector registry.** Lane turns use the generic `AgentBackend` without scoped tools. The kibitzer sets `enabled_toolsets = Some(vec![])` (kibitzer.rs:302), and lanes inherit whatever backend toolsets they get. Per-lane connector scoping would go in `run_lane` session setup (runner.rs:1182-1186).
10. **Heartbeat is separate from lanes.** `work_heartbeat` watches work items and tmux delegate panes, not lanes (work_heartbeat.rs:280-300). Lanes have their own stale detection. Unifying them would happen at `WorkHeartbeatRunner::step`.
11. **Drain doesn't stop lane runs.** The marker is file-based with a 1 h TTL (drain_control.rs:14). Only dispatch checks `is_draining` (dispatch.rs:48); the lane runner loop doesn't (runner.rs:1040-1110). A drain check should go before spawning tasks (runner.rs:1060).

## EXPAND
- src/sessions/external.rs (`ExternalDriver`, `remote_checks`): the existing path for remote or external executors.
- src/cron/remediate.rs (`handle_event`, `settle_lane`): coupling between cron and lanes.
- .omo/plans/gateway-autonomy-lanes.md: the design intent and any planned todos (runner.rs:1160 refers to "Todo 15").
- src/tracker/lanes.rs, src/profiles/lanes_block.rs, bot_profiles/blocks/lanes.md: how the lead agent is told to use lanes.

<results>
<files>
- /Users/indo/code/project/omon-gateway/src/lanes/runner.rs - service loop, turn execution, close/reopen, capture dispatcher, approval block (661)
- /Users/indo/code/project/omon-gateway/src/lanes/cli.rs - lane commands and start flow
- /Users/indo/code/project/omon-gateway/src/lanes/state.rs - state machine and CAS transition
- /Users/indo/code/project/omon-gateway/src/lanes/settle.rs - completion verification
- /Users/indo/code/project/omon-gateway/src/lanes/signals.rs - done-signal types and parser
- /Users/indo/code/project/omon-gateway/src/lanes/activity.rs - REPORT parsing, report classification, stale detection
- /Users/indo/code/project/omon-gateway/src/lanes/dispatch.rs - event batching and lead turn injection
- /Users/indo/code/project/omon-gateway/src/lanes/kibitzer.rs - proactive wake evaluator
- /Users/indo/code/project/omon-gateway/src/lanes/guard.rs - host resource guard and queued-lane promotion
- /Users/indo/code/project/omon-gateway/src/lanes/posts.rs - durable Discord post outbox
- /Users/indo/code/project/omon-gateway/src/lanes/discord_threads.rs - thread naming and creation
- /Users/indo/code/project/omon-gateway/src/work_heartbeat.rs - stale work-item heartbeat
- /Users/indo/code/project/omon-gateway/src/drain_control.rs - drain marker and watcher
- /Users/indo/code/project/omon-gateway/migrations/0031_lanes.sql - core lane tables
- /Users/indo/code/project/omon-gateway/migrations/0038_lane_lifecycle.sql - lifecycle tables
- /Users/indo/code/project/omon-gateway/src/storage/lanes.rs - storage helpers, owner-reply intake
</files>
<answer>Lanes are durable, SQLite-backed autonomous coding jobs. They run a fenced 8-state machine, verify completion with machine-checked done-signals plus a judge-reviewed diff, report through Discord threads with a durable outbox, and notify the lead session through judged, batched events. A rate-limited kibitzer suggests proactive actions. The main gaps for an always-on personal agent: no automatic lane creation, approvals blocked inside lanes, a hardcoded single owner, Discord-only output, a single host with no sandbox, a 24-turn cap, dev-only done signals, no memory write-back, and drain not reaching the lane runner. Each gap has its hook point listed above.</answer>
<next_steps>Use the file:line hook points in "Limits and gaps" to plan extensions. Read the EXPAND leads (external.rs, cron/remediate.rs, the autonomy plan) before designing remote execution or new triggers.</next_steps>
</results>
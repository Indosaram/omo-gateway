---
slug: discord-omo-session-control
status: approved
intent: unclear
review_required: true
plan_path: .omo/plans/discord-omo-session-control.md
plan_sha256: null
review_round_id: null
review_round_limit: 5
pending-action: write and review .omo/plans/discord-omo-session-control.md
review:
  plan_reviewer:
    status: pending
    workspace_root: /Users/indo/code/project/omon-gateway
    runtime_home: null
    target: .omo/plans/discord-omo-session-control.md
    round_id: null
    plan_sha256: null
    launch_id: null
    session: null
    result: null
approach: Fix the CLI control layer (all-scope, real interrupt, multiline remote), add a python3 socket probe run locally or over ssh for live state/questions/interrupt/answer, a pure overview formatter, a /sessions slash command family (owner-only, ephemeral in guilds), a no-model session watcher that DMs the owner on questions and on turn completion of watched sessions, an adopt fix for the real read shape, PATH fix so the lead session finds omo-gateway, and an updated lead prompt block.
---

# Draft: discord-omo-session-control

User pre-approved the plan in the request ("plan approval 은 이 자리에서 모두 허가하니 더이상 질문하지 말고 바로 진행하라"). Approval gate: approved 2026-10-16.

## Affected user and ideal state
Users: (U1) the owner in Discord DM / private owner thread; (U2) the lead OMO session (OmOMeow/wawabot) that calls `omo-gateway sessions ...` via bash; (U3) the target omo sessions on local and omarchy (TUI and daemon hosts) that must not be disrupted; (U4) other Discord users (must see nothing).
See plan `## Scope` for the IS/GAP table (single source).

## Findings (cited - path:lines)
- F1 src/sessions/control.rs read()/send() build `omo thread read|send <id>` without `--all-scope`; gateway cwd is the repo (launchd WorkingDirectory) so any session in another workspace fails `scope_denied` (reproduced: `omo thread read 01a13a7d-...` -> scope_denied; with `--all-scope` -> kind ok).
- F2 `omo thread` has no `interrupt` subcommand (omo-ai/bin/lib/thread.js COMMANDS; probe: `omo thread interrupt x` -> unknown subcommand). control.rs interrupt() therefore always fails.
- F3 TUI control socket protocol (senpi dist/modes/interactive/session-control-commands.js): connect ~/.omo/agent/rpc/tui/t-*.sock, write the bytes of `<sock>.secret`, then newline JSON `{type,id,...}`; reply has same id. Commands: get_state (isStreaming, isCompacting, blocking_question, sessionName, cwd, pendingMessageCount, turn_epoch), get_messages, interrupt, extension_ui_response {uiRequestId, answers:{<qid>:{selected:[],text}}, comment}. Daemon host ~/.omo/agent/rpc/rpc.sock: no secret, list_sessions then get_state {sessionId} (has pendingQuestions).
- F4 Async ask_user_question: assistant toolCall id = requestId; toolResult details.status "pending"; later user message "[Answer to question <id>]". Claude-variant question ids are q1,q2... (ask-user/schema.js parseQuestion). Blocking (waitForAnswer true) TUI overlay is not answerable remotely (pendingQuestion map holds only async).
- F5 ssh word quoting in transport_argv preserves newlines (verified: python3 echo of "line1\nline2 'q' $(x)" over ssh omarchy round-trips exactly). RemoteMultiline refusal is unnecessary.
- F6 omarchy: omo 5.1.29, python3 at /usr/bin/python3, same TUI socket commands; login shell zsh (only per-word quoting is safe).
- F7 Real list: 14 local + 4 omarchy sessions, all status "live" -> current status_reply shows "live 14" only.
- F8 Real `omo thread read --json`: {kind, thread_id, items:[{seq, role, content}], truncated, source, next_cursor only when truncated}; content may be a JSON-encoded string of blocks. external.rs expects nextCursor and items[].id -> adopt always fails ("read response has no cursor").
- F9 Lead session PATH (app-server pid env) = ~/.bun/install/global/node_modules/.bin:~/.bun/bin:/opt/homebrew/bin:... ; omo-gateway lives at ~/.omon/bin -> not resolvable. App-server spawned by gateway (src/agent/omo_daemon.rs:414 daemon_command).
- F10 Lead prompt block bot_profiles/blocks/lanes.md is identical to the DB copy for wawabot (diff rc 0); applied with `omo-gateway profile apply-block`.
- F11 Dirty worktree: src/main.rs, src/agent/omo_daemon.rs, src/agent/omo_backend.rs, src/cron/executor.rs, src/inbox/triage.rs, src/lanes/settle.rs, scripts/* have uncommitted owner changes -> edits must be additive and must not revert them.

## Decisions (with rationale)
- D1 Probe via embedded python3 helper (one process per machine per call) instead of N ssh calls: one round-trip per host, works identically local/remote, python3 present on both.
- D2 Notifications: no model calls; 60 s local/ssh probe tick (memory rule: timers allowed for cheap local checks). Questions always notify; turn completion only for watched sessions (auto-watch when owner sends via Discord) to avoid channel fatigue.
- D3 Discord surface: `/sessions` slash family, owner-only, ephemeral when used in a guild; natural language continues through the lead agent CLI.
- D4 No deploy / no production DB prompt apply in this run (user rule: state changes need explicit instruction); exact follow-up commands are reported.

## Approval gate
status: approved (user pre-authorized in request)

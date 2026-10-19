---
slug: autonomy-two-axis
status: reviewed-okay
intent: clear
review_required: true
plan_path: .omo/plans/autonomy-two-axis.md
plan_sha256: 8106ed580336749ebd44e17120f3fb92bb06b095b8ea02ef7aafc719bd69ffc5
review_round_id: null
review_round_limit: 5
pending-action: write and review .omo/plans/autonomy-two-axis.md
review:
  plan_reviewer:
    status: okay
    workspace_root: null
    runtime_home: null
    target: .omo/plans/autonomy-two-axis.md
    round_id: null
    plan_sha256: null
    launch_id: null
    session: null
    result: OKAY round 3 (st_01a14b76, opus)
approach: learned-autonomy (propose first, precedent ledger + engine memory) two-axis autonomy on the deployed slack-channel source - Axis1 (context pack, report claim ledger with re-measure + visible correction, lane pause/resume/nudge, per-lane worktree) then Axis2 (event-driven action loop replacing the dead kibitzer, deterministic effect table + taint gating auto/ask/ignore, GitHub PR/CI + mail source repair), tool-approval policy as the gate; voice/payments/MCP/vault after
---

# Draft: autonomy-two-axis

## Affected user and ideal state
Users: (1) owner (Indo) talking to the bot on Slack (home DM D0C9HNVUK5M) and Discord; (2) the lead omo session that consumes context packs, lane/loop events; (3) lane omo sessions; (4) other people whose messages reach the inbox (content only, never instructions).
IS-1 | owner never re-explains: each owner turn starts with the thread, related past conversation, open work and memory already in front of the lead | owner 2026-10-18 "기존 맥락을 전부 이해한 상태에서 작업"
IS-2 | every owner message gets an ack at once and a final; long work gets progress updates on one edited status message, never silence > the reporting rule | human.md reporting rule; dori writing.md
IS-3 | anything the bot reported as a fact (number, done, status) is re-measured; if it was wrong the bot posts a visible correction reply under the original message | owner screenshot 2026-10-18 (잡도리 정정); human.md 기계 판독
IS-4 | "done" is believed only from machine-verified signals (exists for lanes; extends to lead reports) | dori done-protocol; src/lanes/signals.rs:26-52
IS-5 | owner says wait -> the named lane stops in the same turn and stays stopped until resumed; quiet lanes get nudged | dori pause/freshness
IS-6 | parallel lanes on one repo never block each other via a shared working tree | prod lanes blocked by "uncommitted /Users/indo/code/project/mhc"
IS-7 | the bot itself checks what a human would check (Slack/Kakao/iMessage/mail inbox, GitHub PRs/CI, lane/cron events) as events arrive - no model polling | owner axis 2; human.md 이벤트 기반
IS-8 | for each found item the bot decides auto / ask / ignore; auto only for non-destructive and small work; ask shows a concrete proposal with a recommended option | owner axis 2
IS-9 | the auto/ask boundary is deterministic code, not model judgment; external-origin content can never trigger send/write/spend/delete without the owner | Instinct/Muse incidents; dori "only the owner's messages are requests"
IS-10 | no notifications KST 0-8; asks batch to 08:00; no duplicate pings | human.md
IS-11 | every new behavior has an env kill switch, default safe, and survives restart | gateway convention
GAP-1 | no context pack: Slack ingress adds only thread root (src/slack/ingress.rs:456-493); no related-history recall | -> todos Wave1
GAP-2 | no lead-report claim ledger / re-measure / correction reply | -> Wave1
GAP-3 | lead status/progress edit only for lane threads (src/lanes/posts.rs:409-471); lead turns have no single evolving status message | -> Wave1
GAP-4 | no lane pause/resume; quiet lanes are only posted stale, not nudged (src/lanes/activity.rs:289-304) | -> Wave1
GAP-5 | lanes share the repo working tree unless a worktree is manually registered (src/lanes/cli.rs:246-248) | -> Wave1
GAP-6 | kibitzer only proposes, cannot act, and in prod never delivered one event (89 judge calls, 59 turn failures, 0 kibitzer lane_events) | -> Wave2
GAP-7 | inbox bot items only become Repo/Lookup/Draft proposals; auto-exec limited to Lookup behind unset flag (src/inbox/proposals.rs:53-57, 501) | -> Wave2
GAP-8 | no GitHub PR/CI watcher (gh used only for done signals, src/lanes/signals.rs:988) | -> Wave2
GAP-9 | mail source dead: zele auth error loop, 0 mail items | -> Wave2
GAP-10 | tool approvals blanket auto-approved (src/agent/omo_backend.rs:1459-1471); lane approval requests fail the lane (src/lanes/runner.rs:693-696) | -> Wave2 (policy gate)
## Components (topology ledger)
C1 | context pack assembled before every owner turn | active
C2 | report claim ledger + re-measure + correction reply + single status message | active
C3 | lane control: pause/resume, nudge, auto worktree per repo lane | active
C4 | action loop: triggers -> item -> effect class -> auto/ask/ignore -> execute -> report (replaces kibitzer) | active
C5 | source coverage: GitHub PR/CI watcher, mail source repair, Slack own-thread watch | active
C6 | tool-approval policy gate (deterministic effect table, taint) | active
deferred: voice, Aside payments, MCP registration, credential vault, facts memory (owner wants voice+payments later; plan lists them as follow-up only)
## Open assumptions (announced defaults)
<!-- Record any default you adopt instead of asking, so the user can veto it at the gate. -->
<!-- assumption | adopted default | rationale | reversible? -->

## Findings (cited - path:lines)
- Deployed source is worktree omon-gateway-wt/slack @ e87390e +39 dirty paths (~/.omon/bin/DEPLOYED). Plan targets that tree; main lacks src/slack.
- Prod DB /Users/indo/code/project/omon-gateway/omon_gateway.db (lsof). lane events are durable and retried when lead busy (src/lanes/dispatch.rs:74-77), so the old "event queue drops events" claim was wrong for lane events.
- Persona already encodes dori routing/brief/done/onboarding rules (bot_profiles/omomeow.json omon:lanes block).
- See notepad for prod DB counts.

## Decisions (with rationale)
- Architect advisory st_01a14b46 (verified by lead): pick gateway-driven deterministic context assembly (1-A) and event reactor + deterministic policy table (2-A). Verified: message_search.search is single channel, root-only (src/storage/message_search.rs:148-176) -> context pack must not rely on blind FTS; use thread recent N + same-thread lane/inbox links + memory, FTS only as owner-scoped cross-channel helper added explicitly. Verified: approval denial aborts the turn after 5 (src/agent/omo_backend.rs:35,1475) -> policy 'ask' must suspend+resume, not deny. Architect said 3; actual 5.
- Claim ledger redesign (architect critique accepted): lead marks claims inline with a structured marker; gateway strips it, binds to the delivered platform message id from delivery_ledger, re-measures ONCE within 60s after delivery (no later re-check -> no false corrections from later external change); only measurable claims; no ban on unmeasured numbers.
- Silent-lane nudge only when no tool call is in flight and quiet >= threshold (architect risk 2).
- Worktree lifecycle owned by gateway with cleanup on close/fail and startup sweep (architect risk 4).

## Scope IN

## Scope OUT (Must NOT have)

## Owner decision 2026-10-18 (verbatim)
"그걸 제한 하면 안 된다는거임 사람이 제한하지말고 봇이 스스로 판단을 해야함 그래서 처음에는 사람에게 제안만 하고 그다음부터는 아 역까지는 해도되는구나 기억하면서 하는거지 이걸 메모리 기능이나 레저 같은걸로 최대한 자연스럽게 구현을 해야함. 게이트웨이 레벨에서 되는거랑 오모 엔진 자체에서 되는거랑 명확하게 구분해서 구현을 해야함."
-> No fixed human-set auto scope. Learned autonomy: start propose-only; each owner approve/decline becomes a precedent; next time the bot acts alone where precedent covers it. Split: ENGINE (omo agent: judgment, memory notes, proposing, reading precedents in its memory) vs GATEWAY (deterministic: approval ledger, precedent matching on approval requests, durable suspend/resume, triggers, delivery, kill switches).
Engine facts: per-bot omo memory repo exists (~/.omo/memory/agents/bot-1465631383862120451-7019e6d7/repo, system/boundaries.md + self-aware.md + notes/facts with auto fact extraction); gateway provisions .omo/omo.json memory.agent=slug (src/agent/omo_backend.rs:512-516); engine permission rules allow/ask/deny exist (~/.omo/agent/settings.json permission); engine approval requests arrive as JSON-RPC (src/agent/omo_protocol.rs:122-155).
Fixed rules kept only where the owner himself set them earlier: payment always confirmed (human.md 2026-10-17), KST 0-8 quiet.
## Open questions

## Approval gate
status: approved (owner 2026-10-18 '승인'; scope answer folded as learned autonomy)
next: on approval scaffold plan, plan-consultant, append todos, ref-check, plan-reviewer rounds
<!-- When exploration is exhausted and unknowns are answered, set status: awaiting-approval. -->
<!-- That durable record is the loop guard: on a later turn read it and resume at the gate instead of re-running exploration. -->

## Review log
- 2026-10-17T19:29:30.861Z plan written (14 todos, 4 waves); structural check OK; 76 path:line refs vs slack worktree, 0 bad. plan-consultant st_01a14b57 launched.
- 2026-10-17T19:52:30.472Z consultant st_01a14b5a (opus) findings folded: added todo 0 spike + project-scoped engine preset (engine allows git*/eval globally); rebuilt 7/8 on existing SmartApprovalGuard + single precedent store (approval_allowlist migrated); named watchdogs (1214,1017) + restart recovery; deterministic short-reply matcher; context pack -> conversations.replies unseen msgs, injected at turn_start only; claims moved after 6, read-only only, stream-buffered; apply_block generalized; lane pause via lane_pauses table + runner cancel token; worktree after 4; migrations 0046-0052 one per todo; release_at column; source_health table; github own 300s timer + inbox collect CLI; backoff fix matches real reset rule; live QA via dashboard chat/resolve API; class precedent only via owner answer to engine-worded offer. Refs 104/0 bad. sha 031bf427116cefd0c82c895ee4b2e42a825dcb4d0bd6345b7f8f4e51c72177e0
- 2026-10-17T20:06:08.970Z review r1 REJECT (release_at cycle, candidate CHECK, lane_transcript.at) fixed; r2 REJECT (todo 14 QA list missing 2/3/12) fixed; r3 OKAY + 3 non-blocking notes folded. refs 109/0 bad. sha 8106ed580336749ebd44e17120f3fb92bb06b095b8ea02ef7aafc719bd69ffc5

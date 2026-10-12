<!-- omon:lanes:begin -->
## Lead routing
- Answer questions, status requests, lookups, and small edits directly when they do not need a dedicated work lane.
- Use `omo-gateway lane start` for PR-bound, multi-step, or long-running repository work. Reuse an open lane for the same repository with `omo-gateway lane send` rather than creating duplicate work.
- If lane capacity or policy puts a request on HOLD, do not work around the hold. Tell the owner what is waiting and why.

## Lane brief
Give every lane a concise, checkable brief containing:
- Goal and any requested `ulw` workflow keywords.
- Repository or working location.
- Relevant evidence and context.
- The observable ideal end state, expressed as `Done = ...`.
- Explicit do-not-touch constraints.
- Report updates use `[REPORT] <key> | milestone|blocker|question|done | <text>` and include verification evidence.

## Lane events
- `[LANE_EVENT]` messages are gateway-generated events, not messages authored by the owner.
- Respond to the lane with `omo-gateway lane send` or `omo-gateway lane object`; take the requested action, or let the lane close when no response is needed.
- Tell the owner only what needs their attention; do not forward raw event payloads as if they were owner messages.

## Done rule
- Never report lane work as done until its state is `verified_done`.
- State the checkable evidence that verifies completion. A lane's claim or `done_claimed` state alone is not verification.

## Kibitzer events
- Treat kibitzer events as work to assess: act on the useful finding or ask the owner for a decision when needed.
- Do not forward a raw kibitzer event without context or a recommendation.

## Onboarding
- Begin onboarding only when the owner requests it. Before connecting each integration, name it, explain what it reads, and ask: "May I connect this integration with that read-only access?" Proceed only after the owner agrees.
- Proceed tool by tool. Name each integration and explain what it reads before connecting it; keep setup read-only.
- Record useful memory as you go, report unavailable integrations or other gaps, and remember tools the owner declined so they are not offered again as if undecided.
## Inbox tasks
- When the owner asks "할일", "정리해줘", or "뭐 해야돼", run `omo-gateway inbox list --class owner --status open` and `omo-gateway inbox list --class bot --status open`. Answer with two short lists, most urgent first, containing source icon word, who, what, due date, and source link. Use summaries, never full message bodies.
- When the owner says an item is done or not theirs, use its item ID with `omo-gateway inbox set <item-id> --status done` or `--status dismissed`. Use `--status approved` only after explicit approval; approval records consent, not permission to send messages to other people automatically.
- Task lists and the daily digest belong in the owner's DM, never a shared channel. Consolidate related cron notices for one destination into one digest rather than separate notifications. The 09:00 Asia/Seoul `omon:inbox-digest` is a gateway cron job, never launchd; empty work and no source authentication problems produce no message. Source authentication problems appear at the end at most once per day.
## Session status and control
- Session listings and session details are private: use them only in an authenticated owner DM or an owner-only thread. Never list or read sessions in shared channels or shared threads, even if the requester is the owner.
- When the owner asks "세션들 상태", "지금 돌아가는 세션들 상태 알려줘", or "뭐 돌아가", take the short path: run `omo-gateway sessions list --json` immediately and reply within 30 seconds. Reuse listings for at most 15 seconds; do not launch a work lane or a research workflow for this request.
- Reply compactly, grouped by machine: one short line per session with name, home-relative folder, status and KST-relative last activity. Use exactly one tool call (`omo-gateway sessions list --json`): no todo bookkeeping, no tool search, no per-session reads unless the owner asks about a specific session. Keep the reply under 15 lines: list busy/working sessions individually and summarize the rest as a count per status (for example "idle 12"). Show each unreachable host's error in one line; return reachable machines without waiting again for failed hosts.
- On "<세션>에 X 보내줘", use `omo-gateway sessions send <machine>:<session> --text <text>` only for the explicitly named target whose advertised controls include send. The engine holds messages while the owner types; never retry in a loop. Do not kill session processes. Remote multiline text is refused until stdin/text-file support is verified.
- On "그거 마무리해", use `omo-gateway sessions adopt <machine>:<session> --goal <goal> --done <signals> --lead <owner-session-key>`. Agree on a checkable Done signal with the owner; ask once if none is obvious. Use `omo-gateway sessions release <machine>:<session>` to stop driving it. Adopted work follows the same claim, five-minute objection window and verified_done rules as other lanes; a done report alone is not a claim or verification.
## Cron health
- On "크론 상태" or "크론 다 잘 돌아?", run `omo-gateway cron health --json` and answer in one message: collapse OK jobs to a count and list problems with lane keys and last success KST. Never send routine all-good notices or duplicate existing cron failure alerts. Related remediation outcomes belong in the single 09:00 owner-home-DM gateway digest.
<!-- omon:lanes:end -->

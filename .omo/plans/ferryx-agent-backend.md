# Plan: Ferryx agent backend for omon-gateway (+ tracker integration)

Status: proposed 2026-10-16, authored from the Ferryx session. Implemented by the gateway's own lead session.
Commits/deploy: follow the gateway's own rules (this file does not authorize either).
Counterpart (contract owner): Ferryx `/Volumes/T9-Mac/project/ferryx-wt/agent-control-20261016/.omo/plans/agent-control-v1.md`,
branch `feat/agent-control-20261016`. Research: Ferryx `docs/research/in-app-supervisor-agent-feasibility-2026-10-16.md` §8-9.

## Why
The gateway is already the supervisor brain for omo sessions (sessions/probe.py + control.rs + watch.rs + external.rs, lanes
settle/judge). It sees only sessions with an omo RPC socket. Ferryx owns every PTY and now exposes an agent-neutral contract
(`ferryx-cli agents ...`), so adding ONE backend extends /sessions, watch notices, external lanes and verified completion to
claude, codex, opencode, cline and every other TUI agent. Supervisor logic is not rewritten.

## Ferryx contract the gateway consumes (frozen v1)
Run locally or over the existing ssh transport (`sessions::control::transport_argv`): `ferryx-cli agents <op> ... --json`.
One JSON object on stdout, `{ "ok": true, ... }` or `{ "ok": false, "code", "message" }`.
Exit codes = gateway's: 0 ok, 2 invalid, 3 not found / not running / no daemon, 4 refused, 5 other / outcome unknown.
- `agents list` -> `{ sessions: [AgentSession] }`
  AgentSession = { sessionId, running, cols, rows, workspaceId?, cwd?, agent?, tier: "native"|"screen",
  state: "working"|"blocked"|"idle"|"unknown", stateSource: "provider"|"screen"|"none", detail?, pending?: Prompt,
  screenRevision?, screenReadable }
  Prompt = { id, agent, kind: "question"|"approval"|"plan"|"menu", title, question, body?, options:[{label,...}],
  multiSelect, customOptionIndex?, steps? }
- `agents read <id>` -> `{ session: AgentSession & { screen: { text, revision, gap, truncated } } }`
- `agents send <id> <text> [--request-id <id>]` -> `{ receipt }`. Typed into the pane through the same ordered queue the
  phone client uses; refused (exit 4, REQUEST_CONFLICT/UNSUPPORTED) when the pane holds a detected prompt.
  The same request id is deduplicated (a retried delivered send returns its receipt) -> pass the gateway's idempotency key.
- `agents answer <id> --prompt <promptId> --revision <screenRevision> (--option N | --options N,M | --text T)`
  -> screen-validated: refused (exit 4) if the screen changed since the revision.
- `agents stop <id>` -> provider interrupt key (Escape). Never a signal.
Exit 5 with code OPERATION_OUTCOME_UNKNOWN means bytes may have been typed: NEVER auto-retry with a new request id.

## Work items (gateway repo)
1. `src/sessions/ferryx.rs` (new): `FerryxBackend<R: CommandRunner>` mapping:
   - list -> `agents list`; map each AgentSession to registry `Session` with `session_id = "fx-" + sessionId` (namespaced so
     omo and ferryx ids never collide on one machine), name = agent or cwd basename, status = state, `controls` =
     ["send","interrupt","answer"] when running.
   - snapshot -> `LiveSession`: streaming = state=="working"; blocking_question = state=="blocked" && pending==null;
     questions = pending ? [PendingQuestion{ request_id: pending.id, questions:[{id,header:title,question,options:labels}] }] : [];
     last_assistant = last non-empty lines of `agents read` screen text (bounded 300 chars); keep `screenRevision` beside it.
   - send(target, text, mode, key) -> `agents send <id> <text> --request-id <key>`. Mode: Ferryx v1 has one behavior (types
     into the pane; the agent queues or takes it) so Auto/Steer/FollowUp all map to send; record mode in the transcript only.
   - answer(target, text, request_id) -> resolve option by label/number exactly like `probe::answers_for`, then
     `agents answer --prompt <request_id> --revision <rev from the same snapshot> --option N` (or --text).
   - interrupt -> `agents stop`.
2. `registry.rs`: per machine, list omo sessions (existing) AND ferryx sessions; ferryx failure (exit 3 = no daemon) is
   "no ferryx on this host", never an error that hides omo sessions (same rule as `unreachable_host_does_not_hide_local_notices`).
3. `control.rs`: route `Target` by session prefix: `fx-` -> FerryxBackend, else existing omo path. No behavior change for omo.
4. `watch.rs`: no logic change; question notices dedupe on `q:{target}:{pending.id}`; completion edge works because
   streaming flips working->idle. Blocked-without-card notice text: "Ferryx pane is waiting; answer in the terminal".
5. `external.rs` lanes: `adopt` on an `fx-` target works unchanged except `read_new`: Ferryx v1 has no transcript cursor,
   so read the screen, diff against the last stored screen text per lane (store in external_session_items as item_id =
   screen revision) and scan new lines for `[CLAIM_DONE] <key> | evidence`. Native transcripts arrive in Ferryx v2.
6. Policy: auto-answer (judge) only when `stateSource == "provider"` or prompt kind is question; approvals on
   `tier == "screen"` escalate to the owner. Never answer when `screenReadable == false`.
7. Tests (gateway style, fake CommandRunner like registry tests): list merge with ferryx exit 3; snapshot mapping of each
   state; send passes --request-id; answer refuses on missing revision; outcome-unknown is not retried.

## Acceptance (gateway side)
- G1 `omo-gateway sessions list` on a host running Ferryx shows omo sessions AND `fx-*` sessions with agent + state.
- G2 `/sessions answer target:<machine>:fx-<id> text:1` answers a claude/codex approval card in a Ferryx pane (verified by
  the pane leaving the prompt in `agents read`).
- G3 a watched `fx-` session emits one question notice and one completion notice (dedupe across restart holds).
- G4 an external lane adopted on an `fx-` codex session reaches verified_done through the existing settle path.

## Tracker integration (Linear-identical tracker, .omo/plans/linear-tracker.md)
Decision: the tracker stays the gateway's planning and review surface and becomes the human-facing view of the SAME
supervisor; Ferryx is not a second tracker and the tracker does not talk to PTYs.
Layering: Issue (tracker, human intent) -> Lane (gateway, execution + verification) -> Session (omo or Ferryx, where the
agent runs). The tracker already mirrors lanes into team AGT and can delegate an issue to a lane; extend that, do not fork it.
1. Delegation target: "Delegate to agent" gains a target picker = `sessions list` entries (omo + fx-). Choosing an existing
   session creates an external lane via `ExternalDriver::adopt(target, goal = issue title+description, done = issue DoD)`;
   choosing "new" keeps today's app-server lane. Issue.lane_key links as today.
2. Live state on the issue: `lane_detail` adds `session: {target, agent, tier, state, stateSource, pending}` from the
   latest watch tick (no new polling path). Board cards in AGT show a "Needs input" badge when state == blocked.
3. Pending prompts as inbox items: a watch question notice also writes a `tracker_notifications` row
   (kind `lane_blocked`, body = prompt question + options) linked to the issue; answering from the issue page calls the
   same `control.answer` path as Discord. One answer path, two UIs.
4. Activity: lane status transitions already write `tracker_activity` kind `lane_status`; add kind `agent_input` for every
   gateway-sent text/answer (actor = agent user or me), so the issue shows what the supervisor typed.
5. Ferryx side (later, not v1): a desktop "Attention" panel can deep-link a pane to its tracker issue via `lane_key` stored
   in pane metadata; Ferryx never stores issues.
Order: finish the tracker plan as written (no change to its current workers' ownership), then items 1-4 here as a
follow-up wave after the Ferryx backend (work items 1-3) lands.

# Todo12 owner integrated source receipt

Status: SOURCE-SETTLED for coordinated compilation and QA. This is not a passing
test result, runtime approval or completion of the authoritative plan checkbox.
Restored after coordinator evidence moved to task-12-shared-wiring.md. This file
records the implementation itself, not a reference to its own prior contents.

## Source ownership and implemented behavior

Todo12 owns lanes/activity.rs, posts.rs, runner.rs, mod.rs and
tests/test_lane_events.rs. Authorized narrow changes also touch the lane-only
completed-item seam in agent/omo_backend.rs, the corrected active close/reopen
regression in tests/test_lane_reopen.rs, and blocked-object handling plus tests
in lanes/cli.rs and tests/test_lane_cli.rs. Shared main/entry/lib/Cargo wiring
and heartbeat removal belong to the coordinator, not this implementation receipt.

LaneActivity::captured_item(key, message, generation, item_id) persists accepted
source identity and report effects transactionally. Generation is the latest
lane_history id with reason='reopened', or zero before first reopen. Duplicate
item identity in one generation does not enqueue duplicate effects. Questions
and blockers transition working to blocked and enqueue one selective event;
milestones enqueue durable posts without a lead event. Done prose produces
idle_without_claim, never verified_done/closed. Claim state suppresses late
report effects. Plain captured activity resets the stale latch.

unreported_turn_generation(key, last_message, judge, generation) uses the existing
Judge ReportIntent choice with a scrubbed 2,000-character excerpt, only after a
successful turn with no accepted report. Progress continues; finished persists
idle_without_claim with its event; blocked_or_question blocks. Status/generation
are checked transactionally after Judge returns, so a newer claim/reopen wins.

quiet_generation(key, turn_running, Some(generation)) is a cheap local check:
20 minutes of captured-message silence while working produces a stale event,
stale latch and post intent in one transaction. Restart honors the stored latch.
No timer invokes Judge or an agent to inspect routine activity.

## Durable posts and actual transport

posts::enqueue inserts typed JSON under lane_transcript role thread_post inside
the caller's activity transaction. Post stores source, destination, scrubbed
bounded body, nonce, state, message_id and error. No migration was introduced.
PostSink::send returns a real message receipt. The DiscordThreads implementation
uses its existing HTTP handle, safe_allowed_mentions, Nonce::String and
enforce_nonce(true). Installed Serenity 0.12.5 source and official Discord docs
were checked; nonce deduplication is only documented for the past few minutes:
https://github.com/discord/discord-api-docs/blob/main/developers/resources/message.mdx

posts::drain conditionally claims pending records as sending, bounds transport
to 30 seconds, and marks sent only after receiving the message ID. Transport
error/timeout is uncertain, never delivered or blindly retried. Missing/invalid
destination stays pending with an explicit error and no transport call.
posts::recover changes interrupted sending to uncertain. No indefinite nonce,
automatic reconciliation or exactly-once guarantee is claimed.

LaneRunner::from_env installs DiscordThreads as PostSink and shares a Notify
with CapturingDispatcher. serve takes the existing service lock, recovers once,
drains startup pending posts, and drains on capture/quiet/outcome notifications.
with_post_sink supplies the fake sink seam. Returned ActivityDecision post
strings are compatibility/informational data, not delivery receipts or a second
enqueue instruction. Runner no longer appends raw post strings after the turn.

## Capture and runner integration

CAPTURE_GENERATION scopes the awaited backend future to its immutable generation.
The backend's lane-only completed-item seam dispatches EditMessage with identity
lane-item:<turn-id>:<item-id>. Its bounded interrupt-drain branch also captures
completed lane items. Missing item IDs use a stable text hash within the turn;
distinct identical ID-less items cannot be distinguished by that fallback.
Non-lane dispatch remains on the original paths.

StreamCapture accepts increasing sequences, replaces cumulative snapshots and
finalizes once. Nonfinal snapshots only refresh activity; final fallback content
is processed as a complete message. Backend final aggregate emission is suppressed
when completed items were already captured. Production stream buffers are cleared
at turn end. Runner does not reprocess completed reports at turn completion.

Outcome writes preserve idle_without_claim, which is cleared only for a new
guided turn. Question/blocker permits the current turn to finish but prevents
another chained turn. Capture, quiet, Judge and relevant runner lifecycle writes
use the reopened-history generation fence.

## Grouped todo9 lifecycle corrections

- fail takes the originating generation. Status/history/dead event share a
  transaction entered through a generation-conditioned write. Service task
  results carry the starting generation into their failure handler.
- clear_failed_resume returns false on zero changed rows before clearing durable
  session metadata or adding fallback history. Its predicate uses reopened
  history only. In-memory binding is cleared only after successful guarded work.
- Outcome transcript insertion uses the existing SQLite transaction, not a
  nested pool write. The writer is acquired before generation/status inspection;
  outcome and turn counter commit together.
- Active close/reopen regression now starts a runnable old turn, subscribes
  before spawning, uses bounded waits, observes a separate owner-text turn and
  ends that new turn with a claim. The backend implements the required run API.

## B12-1 blocked-object correction

The full main todo12 independent audit was read. Its sole blocker is corrected:
change_claim selects existing blocked -> working for object, while claimed
object still selects done_claimed -> not_done. No state table was broadened.
The transaction acquires the writer, records generation, conditionally updates
status, clears idle state, writes objection/history and queues the exact supplied
answer. Existing CLI wake runs after commit. CLI JSON reports resulting status.
Other states still reject with exit 2 and no queued guidance. No todo13 settling
logic was added. Additional capture/transport audit notes were nonblockers.

## Tests and verification limits

Source tests cover report parsing, blocked/question, milestone, misleading done,
claim precedence, MockClock stale/restart/rearm, Judge bounds/options/races,
transaction rollback, sink acknowledgement/recovery/uncertainty, item replay,
snapshot assembly, missing destinations, old-generation capture, local WebSocket
completed-item identity and bounded runner question pause/outcome completion.
Runner unit regressions cover old third-error failure and zero-row stale fallback
preserving newer bindings. Reopen regression covers active generation replacement.
CLI regressions cover captured question -> actual object command -> exact next
owner-text prompt -> claim, plus claimed objection and all invalid object states.

All current-tree tests remain UNRUN. No Cargo compile, clippy, full gate or manual
QA was performed by this worker. Rustfmt parsing/output exited 0 on scoped owned
files during implementation; that is not type checking or a whole-tree fmt pass.
LSP repeatedly timed out. Parent must run the coordinated frozen-source batch:
lane_events, lane_runner, lane_reopen, lane_cli, runner unit regressions and backend
regressions, plus required global gates. Earlier historical passes are not current
tree evidence.

Named scratch QA remains pending: actual post receipt, question DB row and paused
chaining, send/object answer resume, stale restart behavior, three-error dead
event, and recorded-thread close/reopen generation behavior. No live Discord or
live DB action was performed. Coordinator owns shared startup/heartbeat changes.

Nine adversarial classes: malformed input and misleading success have source
tests; prompt boundary is scrubbed but model resistance unverified; cancel/resume
and stale generation have source regressions; dirty worktree is N/A to DB/post
operations; transport hangs are bounded with uncertainty; tests avoid sleeps but
remain unrun; repeated interruption recovery is source-covered, not live-proven.

Cleanup: no persistent process, external message, scratch DB, service mutation,
schema/config/model change or git commit was created by this work. Formatter
subprocesses exited. Coordinator receipt was inspected but not modified.

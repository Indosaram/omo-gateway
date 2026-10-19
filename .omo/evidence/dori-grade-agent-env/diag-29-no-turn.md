# Todo 29: a running lead turn was canceled at the reply deadline

## Conclusion

The premise "no lead turn at all" is false. The dashboard routed the message,
the actor persisted it, the backend created an OMO thread, and that thread ran
several read-only tools. The helper's 30000 ms final-reply observer rejected;
`runQA` immediately entered its `finally { await stop(); }`, stopping the
gateway and canceling the still-running turn. This explains the missing final
assistant row and delivery obligation. The 30-second final-reply requirement
really failed; this diagnosis does not convert it into a pass.

No product source was changed. The harness correction captures deadline
evidence before cleanup instead of leaving only an empty final delivery ledger
from which to infer that no turn ran. It does not extend the deadline, retry,
change the model, or run live QA.

## Authoritative evidence

Scratch: `/private/tmp/omon-task29-live-final-1lkxge`.
Binary SHA-256: `0924e30d9a73ebc57092b07d501b26da898841b200a2daba7dcb346b2d769e00`.
The original `task29-live-last.md` and scratch files were not modified.

Read-only SQLite queries showed one user row and no delivery obligations, but
also a durable `sessions.state_json.metadata.omo_thread_id` of
`01a12e40-dc9e-7de8-8303-eab09533e6b6`, persisted at
`2026-10-12T03:56:07.179Z`. This binding alone disproves failure to reach the
backend's thread creation stage.

The original OMO session is:

`/Users/indo/.omo/agent/sessions/--private-tmp-omon-task29-live-final-1lkxge-workspace-agents-bot-1465631383862120451--/2026-10-12T03-56-05-150Z_01a12e40-dc9e-7de8-8303-eab09533e6b6.jsonl`

Its exact event timestamps, rather than the shutdown-flushed memory mirror's
`captured_at` timestamps, establish this sequence (UTC):

| Time | Evidence |
| --- | --- |
| 03:56:05.058499 | Gateway log: dashboard listening. |
| 03:56:05.142234 | SQLite: actor persisted owner user message. |
| 03:56:05.150 | OMO session header: new lead thread created. |
| 03:56:07.267 | OMO user message: Korean status request accepted. |
| 03:56:12.640 | Assistant text and todo/tool_search calls, stopReason `toolUse`. |
| 03:56:16.680 | Assistant invokes `tool.thread_list({})` through eval. |
| 03:56:18.941 | Tool result: workspace scope, zero threads. |
| 03:56:22.632 | Assistant invokes `tool.thread_list({all_scope:true})`. |
| 03:56:24.540 | Tool result: local-machine thread listing. |
| 03:56:27.769 | Assistant invokes eval to extract fields from that listing. |
| 03:56:27.786 | Tool result: 25 sessions. |
| 03:56:35.144907 | Gateway log: lane service stopped by cleanup. |
| 03:56:35.165 | OMO assistant message: stopReason `aborted`, empty text. |

The read-only mirror also preserves the tool calls and abort:

`home/.omo/memory/agents/bot-1465631383862120451-7019e6d7/runtime/transcripts/01a12e40-dc9e-7de8-8303-eab09533e6b6/transcript.jsonl`

The agent used built-in thread tools, not the new `bin/omo` wrapper. Therefore
the wrapper's permitted/rejected command behavior did not cause this failure.
No final response after 30 seconds can be inferred: the turn was canceled.

## Comparison with todos 20 and 32

The descriptor env-key arrays for task 29, task 20 (`yqT9LF`), and the replying
task 32 (`RiFI5E`) are identical. Common defaults are LANES=true, INBOX=false,
DISCORD=false, KIBITZER=false, and AUTOSPAWN=off. All use an owned app-server;
task 29's dashboard logs explicitly identify `ws://127.0.0.1:55820` for both
interactive and cron backends. The restart retained those environment values.
All three sessions use the same lead storage key.

The concrete control-flow difference is the observer lifetime:

- Task 29: `qa-last-readonly-extra.mjs` awaits a 30000 ms final-delivery observer;
  its rejection escapes the scenario and triggers gateway cleanup immediately.
- Task 20: `qa-final-scenarios.mjs:profileChat` uses 240000 ms observers. Its
  first user row at 03:06:00.964920 received an assistant row at
  03:06:47.363780 (46.399 seconds), already longer than task 29's cutoff.
  Its gateway INFO log likewise has no routine turn-start entry.
- Task 32 (`RiFI5E`): the same `extra` helper, without the readonly wrapper,
  uses a 240000 ms observer. User row 03:41:36.313479, assistant row
  03:41:59.212726 (22.899 seconds), delivered obligation present. Its gateway
  INFO log also has no routine turn-start entry. The response failed to obtain
  the desired cron data; it proves chat delivery, not successful cron reporting.
- Task 32 (`YhA5QX`) never brought up the dashboard in its retained log and
  cannot serve as evidence of a successful chat in that particular run.

The different elapsed times are observed, not evidence of a fixed latency
guarantee. Neither environment routing nor an actor pickup delay explains the
task 29 result: the actor had processed the message within milliseconds.

## Product path inspected

- `src/dashboard.rs:1176`: `post_chat` resolves the session key, constructs a
  message event, and calls `mux.route`; 202 acknowledges queue admission.
- `src/dashboard_runtime.rs:129`: standalone dashboard constructs its own
  OmoBackend and SessionMultiplexer with the owned endpoint configuration.
- `src/multiplexer/router.rs:338`: `route` admits the event to its session actor.
- `src/multiplexer/actor.rs:322`: actor persists inbound before running the
  backend; shutdown cancellation cancels the active runner future.
- `src/agent/omo_backend.rs:666`: setup connects, initializes, resolves the
  thread; `persist_session_binding` stores its ID. Normal startup does not
  emit an INFO "turn start" line.
- `src/agent/omo_backend.rs:586`: `deliver_rendered_result` creates the final
  delivery obligation and assistant transcript row. Intermediate tool-use
  messages in the OMO session do not establish final gateway delivery.

## Change and verification

`qa-last-readonly-extra.mjs` now saves
`owner-session-status-deadline.json` before rethrowing an observation failure.
It includes elapsed time, the session binding, user/assistant rows, delivery
rows, and gateway log. A failed deadline remains failed and normal cleanup
still runs.

`task29-deadline.test.mjs` is an offline helper regression: it substitutes only
the common runner and dashboard boundary, executes the actual task 29 branch,
and checks subscription before POST, the unchanged 30000 ms bound, preservation
of the original rejection, and evidence saving before cleanup. It spawns no
gateway or app-server and sends no message.

Verification: `bun test ./.omo/evidence/dori-grade-agent-env/task29-deadline.test.mjs`
passed with 1 pass, 0 fail, exit code 0 (14 ms). The initial invocation without
the `./` prefix matched no files because Bun's discovery excluded the hidden
directory; no tests executed in that invocation. The explicit file invocation
ran the regression once successfully.

Workspace `git diff --check` also found an existing unrelated
`Cargo.toml:123: new blank line at EOF.` This file was not changed or repaired
by this diagnosis.

Rustfmt, cargo clippy, and Rust test targets are not applicable because product
source was not changed. The JS language server could not initialize because
this workspace has no valid TypeScript installation; no dependency was added
for this helper-only change. Live QA was not run, and compliance with the
30-second final-reply requirement remains unverified after this diagnostic fix.

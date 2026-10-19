# Task 35 QA2 usage source investigation

## Verdict

The installed omo 5.1.30 / senpi 2026.10.11 app-server does not emit token usage notifications. This is an upstream projection omission, not a gateway method-name/JSON-path mismatch or a missing lane pool. No product fix is possible within the four authorized Rust files by parsing real notification frames: those frames contain no counters. Product source and assertions were left unchanged rather than fabricating usage or labeling a synthetic fixture as a real emitter frame.

## Installed source evidence

All Senpi references below are relative to `/Users/indo/.bun/install/global/node_modules/@code-yeongyu/senpi/dist/`.

- `modes/app-server/protocol/methods.js:198` declares `thread/tokenUsage/updated`; `modes/app-server/server/notifications.js:16` merely allows it for broadcast. Neither is an emitter.
- `modes/app-server/threads/turns.js:214-260` subscribes to session events and passes them through `EventProjector`, then emits the projected notifications. `threads/turns.js:309` forwards those notifications.
- `modes/app-server/threads/projection.js:34-43` handles assistant `message_end` by completing dangling text and projecting provider-native content. It never reads `message.usage`.
- `modes/app-server/threads/projection-message-items.js:22-36` emits the completed text item containing type, id, text, phase and memoryCitation, not usage. `threads/projection.js:205-217` wraps it as `item/completed` with `params.threadId`, `params.turnId`, `params.item`, and `params.completedAtMs`.
- `modes/app-server/threads/turn-terminal.js:13` emits `turn/completed` with `{threadId, turn}`; `threads/turn-runtime.js:48-64` builds the turn with items/status/timestamps/error, without usage.
- `core/session-manager.js:1214-1225` shows the actual internal assistant counters: `message.usage.input`, `.output`, `.cacheRead`, `.cacheWrite`. These are persisted/internal counters, not app-server notification fields.
- The entire installed runtime chunk `bundle/chunks/app-server-command-NHDOL4HQ.js` was read and searched, not just the grep tool's first 4 MiB. `thread/tokenUsage/updated` and `tokenUsage` each occur exactly twice, both on line 6 in method allowlists; `token_usage` occurs zero times. There is no usage emitter hidden later in the bundle.
- `/Users/indo/.bun/install/global/node_modules/omo-ai/package.json:3,20` identifies version 5.1.30 and the Senpi 2026.10.11 dependency. Searches of omo-ai found no alternate token-usage emitter; its launcher/postinstall delegates to/prepares the installed engine.

## Direct installed-code reproduction

Imported the installed `EventProjector` directly, without starting a service or calling a model. Projected an assistant `message_end` with usage `{input:100, output:20, cacheRead:10, cacheWrite:0, totalTokens:130}`. With fixed `nowMs: () => 123`, the result was byte-for-byte identical after removing usage from the input message:

```json
{"method":"item/completed","params":{"threadId":"thread","turnId":"turn","item":{"type":"agentMessage","id":"message-1:0","text":"done","phase":null,"memoryCitation":null},"completedAtMs":123}}
```

This is the actual projection shape, not a guessed token-usage fixture. There is no real token-usage frame to cite or substitute into `tests/test_usage_done_gate.rs:27`.

## Gateway comparison and lane wiring

- `src/usage.rs:63-80` requires `thread/tokenUsage/updated`, exact `/params/threadId` and `/params/turnId`, and `/params/tokenUsage/last` containing camelCase `inputTokens`, `outputTokens`, `cachedInputTokens`. The synthetic fixture matches this contract; it does not prove installed-server emission.
- `src/agent/omo_backend.rs:1309-1328` records only after a start acknowledgement, with `self.pool = Some` and an active turn ID, and a resolved usage purpose. This handler is in the shared execution loop, not a chat-only branch.
- `src/lanes/runner.rs:892-901` constructs `OmoBackend::new(...).with_pool(pool.clone())`. `runner.rs:542-550` creates a `lane` session key, and `runner.rs:1253-1255` invokes the shared backend's `run_cancelable`.
- `src/usage.rs:267-280` resolves lane purpose from `lane_usage_context` by the session channel ID.
- Read-only SQLite checks of both retained QA DBs reproduced `model_usage` count 0, lane purpose `lane_large`, `last_turn_outcome = completed`, and non-null real thread IDs. No scratch database was modified.

## Scope and required next change

To make real usage recording work, the owning Senpi app-server must expose counters first, either via a correlated notification or a supported authoritative per-turn usage API. An upstream emitter should include every model call within a tool-loop turn, not just its final assistant message, and define cached-input semantics explicitly. Then the gateway parser and exact emitter fixtures can be aligned and live QA repeated. Editing the installed dependency, adding a side-channel session-file reader, or inventing counters is outside this request's allowed scope. No services, models, environment files or git commands were used.

## Verification

Fresh LSP diagnostics for both product files timed out after 3000 ms each; these are unavailable diagnostics, not clean results. No Rust source was changed.

Requested warm-target command was launched once, preserving normal Cargo lock waiting:

```sh
RUSTC_WRAPPER= CARGO_TARGET_DIR=$PWD/target-w4 cargo test -j4 --offline --test test_usage_done_gate --test test_omo_backend
```

The command waited for the build-directory lock, compiled successfully in 9.61 s, then exited 101: backend tests had 45 passed, 3 failed, 1 ignored in 30.88 s. The exact failure names match the three canonical names specified in the request and present in `/Users/indo/code/project/omon-gateway-wt/dori-grade-agent-env/.omo/evidence/dori-grade-agent-env/task-2-failing.txt`; zero new failure names:

- `test_omo_backend_e2e_thread_lifecycle_and_streaming`: `tests/test_omo_backend.rs:1784`, Some(true) versus Some(false).
- `test_omo_backend_aborts_turn_on_repeated_approval_denials`: `tests/test_omo_backend.rs:2587`, timeout during turn streaming: no activity from server.
- `test_omo_backend_deadline_interrupts_looping_turn`: `tests/test_omo_backend.rs:2143`, deadline took 9.815892208 s.

This is exact-name baseline matching, not an independently established equivalence of historical panic causes. Cargo stopped after the failing backend target, so `test_usage_done_gate` did not execute in that combined command. The omitted target was launched separately with the same warm target/offline/-j4 settings: compilation finished in 51.50 s, 9 passed / 0 failed / 0 ignored in 0.53 s, exit 0. Those passing tests still exercise the synthetic notification contract, not installed app-server token emission.

Overall: source cause settled; requested tests executed with zero new backend failure names. Real usage recording remains blocked on an upstream emitter/API change outside the authorized files. No live QA or successful product fix is claimed.

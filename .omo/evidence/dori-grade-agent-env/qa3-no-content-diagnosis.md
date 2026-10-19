# QA3 no-content diagnosis

Date: 2026-10-12. Investigation only; no product source, existing helper, env file, owner database, shared daemon, or Discord changes. One owned app-server reproduction, one thread, one turn. The released gateway binary was not needed or executed.

## Conclusion

The failed initial turns actually executed **openai/gpt-6-astra**, not the gateway's requested DeepSeek or Opus models. Every one of the seven retained OMO sessions records an extension-origin model switch from mahoquot/gpt-6.1-sol to openai/gpt-6-astra, followed by an assistant message with `stopReason: error`, `content: []`, and **OpenAI API error (401), code invalid_api_key**. Credential material is deliberately omitted here.

The extension actor is `/Users/indo/.bun/install/global/node_modules/omo-ai/plugin/extensions/omo.js`. The gateway requested custom-provider models, but the installed app-server does not honor those requests through the paths the gateway normally uses. Its thread/start model lookup checks builtin providers only; its turn/start adapter discards model and effort. This leaves startup/default/extension model selection in control. The extension then selected OpenAI Astra, whose request failed authentication. The app-server's message_end projection does not inspect an assistant message's stopReason/errorMessage, and agent_end completes the turn as successful when no earlier projected failure was emitted. That explains why the gateway receives a completed turn without useful items and reports the generic no-content error instead of the underlying 401.

This is a model-selection contract mismatch plus an upstream error-projection gap, not plain availability of the four intended models. A fresh PONG turn was successful but ran **mahoquot/gpt-6.1-sol**, despite explicitly sending DeepSeek in both requests. It is evidence that the WebSocket path is alive and model parameters are ineffective, not proof that a DeepSeek WebSocket turn succeeded.

## Models: requested versus executed

The four retained gateway logs use INFO verbosity and contain no raw turn/start parameter frames. Requested models below are reconstructed from readonly qa.db session state, lane.model, the profile export in task20-qa3.json, and the request construction in src/agent/omo_backend.rs:698-702 and src/agent/omo_protocol.rs:69-97. They are not claimed to be captured historical wire parameters. Executed models are directly recorded in OMO assistant message records.

| Scratch / session | Gateway active_model / lane.model | Actual failing provider/model | Retry behavior |
| --- | --- | --- | --- |
| task20 Bg8qIM / cc1a | inferhub/cb/deepseek-v4.1-flash | openai/gpt-6-astra | No backend fallback: requested primary already equals fallback. |
| task26 fhq8vX / d67c lead | mahoquot/claude-opus-5-5 | openai/gpt-6-astra | 00:42:54.900 fallback warning; settings update switches to inferhub DeepSeek at 00:42:54.910; QA26_NO_APPROVAL delivered. |
| task26 fhq8vX / 181b lane | inferhub/cb/deepseek-v4.1-flash | openai/gpt-6-astra | Three lane attempts fail 401; primary equals fallback, so no backend fallback update. |
| task35 6s6dvB / ce42 lane | inferhub/cb/deepseek-v4.1-flash | openai/gpt-6-astra | Initial three attempts fail 401; later session activity also records 401. |
| task35 6s6dvB / fe23 lead | mahoquot/claude-opus-5-5 | openai/gpt-6-astra | 00:43:00.829 fallback warning; DeepSeek settings update at 00:43:00.961; tool activity continues until QA shutdown. |
| task35 v5b6tM / d00c lane | inferhub/cb/deepseek-v4.1-flash | openai/gpt-6-astra | Three attempts fail 401; no backend fallback update. |
| task35 v5b6tM / fe3e lead | mahoquot/claude-opus-5-5 | openai/gpt-6-astra | 00:43:00.845 fallback warning; DeepSeek settings update at 00:43:00.987; tool activity continues until QA shutdown. |

The task26 lead's next turn explicitly restores mahoquot/claude-opus-5-5 at 00:43:18.699 and receives a normal text response at 00:43:26.501. This is an internal positive control: thread/settings/update works with custom providers, while the first-turn request path does not. The task35 fallback turns did not end in the original no-content failure; they were interrupted when the QA actor shut down.

`DEFAULT_MODEL=gemini-3.8-flash-high` was inherited according to the supplied QA context and remains present in this investigation shell. None of the seven failing assistant records used Gemini. Production's OMON_DEFAULT_MODEL and lane purpose routing also do not establish the remote session's actual model.

## Authoritative retained evidence

All scratch qa.db files were opened readonly. Selected columns were sessions.state_json.active_model, metadata.omo_thread_id, metadata.omo_fallback_model_active, and lanes.model/status/last_turn_outcome. No owner DB was opened. Model usage rows were absent; no token accounting was performed.

OMO JSONL session files were read at these exact paths under `/Users/indo/.omo/agent/sessions/`:

- `--private-tmp-omon-task20-qa3-Bg8qIM-workspace-agents-bot-1465631383862120451--/2026-10-12T00-42-41-050Z_01a12d8f-cc1a-70f6-8e9f-89be841594e6.jsonl`: model_change ff884512 at 00:42:51.118, error assistant 5662986e at 00:42:52.124.
- `--private-tmp-omon-task26-qa3-fhq8vX-workspace-agents-bot-1465631383862120451--/2026-10-12T00-42-43-708Z_01a12d8f-d67c-7527-bb86-738ece0abeb2.jsonl`: model_change 0451913e at 00:42:53.733, error 4c9fa288 at 00:42:54.878, successful fallback model_change 01e69690.
- `--private-tmp-omon-task26-qa3-fhq8vX-workspace-qa26--/2026-10-12T00-43-00-507Z_01a12d90-181b-75df-94d1-9e5ddca0f8d8.jsonl`: model_change d78aa2ee at 00:43:07.189, errors 938152e2 / 891a43c6 / 137ea8ac.
- `--private-tmp-omon-task35-qa3-6s6dvB-workspace-qa35--/2026-10-12T00-42-41-602Z_01a12d8f-ce42-75d5-abd1-e78970788885.jsonl`: model_change 445da459 at 00:42:50.709, initial errors b5051a0a / fcaf26ea / 12361b8e.
- `--private-tmp-omon-task35-qa3-v5b6tM-workspace-qa35--/2026-10-12T00-42-42-060Z_01a12d8f-d00c-794e-965f-89a339c7e7d4.jsonl`: model_change 8ff6fc56 at 00:42:50.909, errors 1cf0310f / 95d9b4de / 60318dd3.
- `--private-tmp-omon-task35-qa3-6s6dvB-workspace-agents-bot-1465631383862120451--/2026-10-12T00-42-53-859Z_01a12d8f-fe23-75e5-b1c0-a4735b51647f.jsonl`: model_change 9c4208b8, error 0ff6c05b, fallback 2ef7ebca.
- `--private-tmp-omon-task35-qa3-v5b6tM-workspace-agents-bot-1465631383862120451--/2026-10-12T00-42-53-886Z_01a12d8f-fe3e-7d5e-8218-ceef72c5468d.jsonl`: model_change 0c8e0ab4, error 07e02e93, fallback 49a1c955.

Scratch memory transcripts independently preserve the same 401 error messages under `home/.omo/memory/agents/<agent>/runtime/transcripts/<thread-id>/transcript.jsonl`. The senpi-rpc-host-internal directories retain sockets and ownership files, not server stderr logs. The initial broad owner-session search timed out with partial coverage; the subsequent exact scratch-cwd directory reads above supplied the authoritative evidence.

## App-server and helper findings

`qa-owned-appserver.mjs:53-58,120-135` records argv but deliberately consumes and discards both stdout and stderr after recognizing readiness. There is no persisted server error capture. Existing gateway logs cannot answer whether the server printed a 401; the session JSONL can and does show the provider's 401 response.

The helper's recorded argv has no explicit --extension. That is not a missing-extension failure: the installed public launcher at `omo-ai/bin/lib/launcher.js:312-315` appends `--extension <omo-ai/plugin>` to app-server launches. Historical model_change actor records and qa3-final-integrity.json RPC-host process argv confirm that the extension loaded.

The original request described HOME=/Users/indo, but all four retained receipts record appserver.home as the scratch `home` directory. `qa2-common.mjs:218` passes the entire gateway env into withOwnedAppServer; `qa-owned-appserver.mjs:55,113` merges it over process.env. Its HOME assertion checks the parent, not the merged child env. Therefore the receipts contradict the asserted child HOME. This alone did not remove the provider registry: sessions were actually saved in the owner's canonical agent directory, and fallback custom models worked. The parent carries explicit coding-agent-dir overrides in this session; historical descriptor.envKeys does not record those inherited server keys. Do not infer missing credentials solely from the scratch HOME.

Installed runtime inspected: `/Users/indo/.omo/agent/runtime/dd11217900179eb7-6e4044c3d021/dist/`.

- `modes/app-server/threads/start-options.js:4-14,52-59`: requestedStartModel resolves only getProviders()/getModels() builtin entries; custom mahoquot/inferhub models fall through to undefined.
- `modes/app-server/turn-adapter.js:9-17`: turnStartParams retains threadId, clientUserMessageId, input and unknownCommandAsText only. model and effort are discarded.
- `modes/app-server/threads/settings-handlers.js:70-88`: settings update resolves against the actual session.modelRegistry, including custom providers.
- `modes/app-server/threads/projection.js:35-44`: message_end projects text/native items, not stopReason/errorMessage. The separate message_update error branch can fail a turn, but historical failures did not reach the gateway as failed terminal turns.
- `modes/app-server/threads/turns.js:217-224`: agent_end calls completeTurn with default completed status. finalizeProjection precedes terminal notification emission, reducing the plausibility of a terminal-before-item race.

## What no-content means here

`src/agent/omo_backend.rs:1506-1513` rejects a completed turn when filtered text is empty **and total_tool_calls is zero**. A tool-only turn with item/started tool items is accepted, not labeled no-content. The failed assistant records contain no text, reasoning, or tool calls at all; they contain provider errors. The initial failures are not an approval wait or a valid tool-only result.

The default app-server approval policy is never; the fresh response advertised never and dangerFullAccess. Gateway approval requests are answered automatically unless APPROVAL_MODE/POLICY is always, and neither key appears in the retained gateway descriptor.envKeys. No approval request appears in the single reproduction. Historical raw notification sequences were not retained, so their exact ordering cannot be reconstructed; zero-content provider errors are directly observed and are sufficient to explain the symptom without inventing a race.

## Single minimal reproduction

Executed once at 00:55:48 UTC through the unchanged withOwnedAppServer helper, HOME=/Users/indo, authenticated private ephemeral WebSocket, cwd `.omo/evidence/dori-grade-agent-env`. Requests:

1. initialize with experimentalApi capability.
2. thread/start with model `inferhub/cb/deepseek-v4.1-flash` and cwd.
3. turn/start with the same model, effort high, input `Reply with exactly: PONG`.

thread/start returned model `gpt-6.1-sol`, modelProvider `mahoquot`, approvalPolicy `never`, sandbox `dangerFullAccess`. The exact ordered wire sequence, method names and item types only:

```text
response (initialize)
extension_event
extension_event
extension_event
response (thread/start)
thread/started
extension_event
response (turn/start)
thread/status/changed
turn/started
item/started userMessage
item/completed userMessage
extension_event
item/started dynamicToolCall
item/completed dynamicToolCall
item/started agentMessage
item/agentMessage/delta
item/agentMessage/delta
item/completed agentMessage
thread/status/changed
turn/completed
```

Terminal status completed; agent text was exactly PONG. No approval request or error notification. The actual reproduction session file is `--Users-indo-code-project-omon-gateway-wt-dori-work-.omo-evidence-dori-grade-agent-env--/2026-10-12T00-55-48-385Z_01a12d9b-cfa1-7e33-aa04-37e059ab3500.jsonl` under the owner agent sessions directory. It records mahoquot/gpt-6.1-sol for both assistant messages, with no switch to the requested DeepSeek model. Later shutdown records an aborted assistant entry; the observed requested turn had already returned PONG and completed. No second model turn was submitted by this diagnostic client.

The fresh run did not repeat the historical extension switch to OpenAI Astra. Its absence is reported, not hidden: this is a minimal transport/model-contract reproduction rather than a controlled recreation of the full historical workspace/profile startup. Historical actor records directly prove that switch for all failed initial sessions.

Process exit code 0 for the diagnostic runner. Helper receipt: ready 430 ms; total 36762 ms; cleanup 15066 ms. Owned child required SIGKILL after its 15-second graceful deadline. pidAbsent=true, remainingGroupPids=[], portRebindSucceeded=true, privateScratchRemoved=true. The temporary diagnostic script was removed after execution. The ordinary OMO session transcript is retained as evidence; the helper's private token directory is removed. Shared port 19742 was not contacted.

## Exact proposed changes (not implemented)

### Gateway model contract fix

In `src/agent/omo_backend.rs:705-715`, remove the gate that sends thread/settings/update only for model_override or omo_fallback_model_active. Whenever the effective model is Some, send and await the existing settings update **before every turn/start**, after thread creation/resume and extension binding. Reuse the current custom-provider-aware update RPC and existing rejection handling. Keep turn/start.model for future compatibility, but do not rely on it. This also fixes the no-fallback lane case: a lane already requesting the fallback model must still enforce that model remotely, rather than assuming it was applied.

Add a neighboring fake WebSocket integration regression: thread/start responds with a different startup model; turn/start.model has no effect on the server; server returns PONG only if a preceding acknowledged thread/settings/update set the requested custom model. Assert the first normal turn (no fallback metadata) performs settings update then turn/start. Cover active_model change on a resumed thread and primary-equals-fallback. Register expectations before triggering, use bounded event awaits, no sleeps. A rejection of settings update must not silently continue on the wrong model. This test fails the current gate and passes the proposed change.

### Upstream app-server error projection fix

On assistant message_end, recognize stopReason error/aborted and carry errorMessage into failed/interrupted terminal state before agent_end can mark completed. Add a runtime regression using message_start + message_end(error, content empty) + agent_end without a message_update error event; expect error notification plus turn/completed failed with the provider error, not completed with only userMessage. This fix belongs in OMO runtime, not an invented parser in the gateway. Current gateway cannot recover a provider error that the server never transmits. Do not suppress genuine no-content errors or mark them successful.

### QA helper corrections

In qa-owned-appserver.mjs, build `childEnv = { ...process.env, ...toolEnv, HOME: homedir() }`, assert childEnv.HOME, and record that exact value in receipt.home. Keep the QA gateway's own HOME isolated; do not pass its HOME through to the app-server. The existing public launcher already supplies the extension, so adding duplicate --extension arguments is unnecessary.

Retain a sanitized server log/event summary rather than discarding every line. At minimum record stream byte counts, error classifications, selected provider/model, thread/settings/updated and terminal status/error codes; never bearer tokens, API keys, prompts, tool arguments or raw extension_event payloads. The diagnostic instrumentation's attempted spawn interception reported zero stdout/stderr bytes; that counter is not reliable because the helper's already-bound named spawn import was not intercepted. It is not evidence of a silent server.

For QA model fidelity, resolve expected models explicitly per purpose and verify the **remote** settings after binding, rather than copying DEFAULT_MODEL from the parent and trusting lane.model. A harness-only workaround must actually set and verify thread/settings/update before the turn; merely changing DEFAULT_MODEL or HOME cannot repair the ignored wire-model contract. Implement that in a QA-only adapter or adopt the gateway fix above. No environment files need modification.

## Limits and validation

The historical gateway wire frames and app-server stdout/stderr were not retained. The report distinguishes reconstructed gateway requests from directly observed execution. No product fix or regression test was run, because source edits were expressly excluded. The temporary JS diagnostic executed successfully, but its editor diagnostics were unavailable because this workspace has no valid TypeScript installation for that LSP. No dependency was installed. No cargo build, released-binary execution, commit, deployment, shared daemon operation, or live Discord action was performed.

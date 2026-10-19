<!-- task=st_01a14524 -->
st_01a14524 [completed] model mahoquot/claude-sonnet-5-5
I read this as a comprehensive research request: pinned-SHA source reads on OpenClaw and Hermes-agent, compared against the local Rust gateway. I'll stop when the report has sources for every claim.

Ask: Deep-dive OpenClaw and Hermes-agent gateways, then say what our Rust gateway lacks - wanted: a sourced report of at most 1500 words. For you: the research is done. Every source below was PRIMARY (read at a pinned SHA), and the local-gap list comes from grep and is inference. Now: done. Next: none.

**Pinned SHAs:** openclaw `88ef99e66fc9b698c2350774b5ef5dbef3c6c9a4`, hermes-agent `e6899c8090da3bd48c829893e4103ce989d3482d`.

**Routes:** `gh api` for SHAs and trees worked. `raw.githubusercontent.com` through curl worked for all files, with no Cloudflare challenge. I didn't need the web.archive.org, jina, HN or Reddit fallbacks. The `gh api` raw Accept header was rejected by the broker.

Short version: OpenClaw has dropped `HEARTBEAT.md` as a runtime file. Heartbeat is now a system-owned cron job with a DB-stored scratch field. Hermes has a much simpler per-session heartbeat. Our Rust gateway appears to have the cron, silence-token and approval basics. It appears to lack the layers around them: wake queue, node and device identity, sandbox, and scoped approval grants.

## Findings

- **OpenClaw HEARTBEAT.md is gone at runtime.** `openclaw doctor --fix` imports it into DB "monitor scratch" (256 KiB cap, compare-and-swap writes, `--expected-revision`). It archives the file and says the runtime never reads it. [https://github.com/openclaw/openclaw/blob/88ef99e66fc9b698c2350774b5ef5dbef3c6c9a4/docs/gateway/heartbeat.md] PRIMARY, curl raw
- **Heartbeat is one system-owned cron job per heartbeat-enabled agent.** Config is the desired state and the persisted schedule owns the tick. Default cadence is 30m, or 1h with Anthropic OAuth. [same heartbeat.md] PRIMARY
- **The default prompt is deliberately narrow.** It tells the agent not to infer tasks from prior chats and to reply `NO_REPLY` when nothing needs attention. Recurring work belongs in cron jobs. [same] PRIMARY
- **Response contract.** The agent can call a `heartbeat_respond` tool with `notify` and `notificationText`, plus an optional `scratch` that replaces its own checklist. Legacy `HEARTBEAT_OK` is stripped at the start or end of a reply if at most 300 characters remain. [same] PRIMARY
- **Skip rules save model calls.**
  - Skip on effectively empty scratch.
  - Skip when the main queue or target session is busy.
  - Skip outside `activeHours`.
  - Skip with `no-route` when no owner DM resolves.
  - Skip when all of `showOk`, `showAlerts` and `useIndicator` are false.
  - Skip with exponential readiness backoff (1 min doubling up to the interval) when the channel is down. [same] PRIMARY
- **Event wakes have their own rate limits.** There is a 30-second minimum between event turns and a flood guard after five starts in 60 seconds. This applies even when the recurring cadence is `0m`. [same] PRIMARY
- **Completion events keep their captured delivery route.** A completion is delivered to the original account, conversation and thread. The heartbeat's configured `target` cannot redirect it. Resetting the session cancels retained events instead of moving them to the replacement. [same] PRIMARY
- **Cost levers:** `isolatedSession` (about 100K tokens down to 2-5K), `lightContext`, and a cheaper heartbeat `model`. [same] PRIMARY
- **Hermes heartbeat is session-scoped and simpler.** `HeartbeatState` has `prompt`, `interval_seconds`, `status` (active, paused or cleared), `created_at`, `last_fired_at` and `fire_count`. Minimum interval is 60s and it fires only into an idle session. [https://github.com/NousResearch/hermes-agent/blob/e6899c8090da3bd48c829893e4103ce989d3482d/hermes_cli/heartbeat.py] PRIMARY, curl raw
- **Hermes fire accounting.** `due_prompt()` records the fire before the turn runs. Missed ticks coalesce by resetting the anchor to now. `abandon_fire()` rewinds the claim only if the persisted state is unchanged. [same] PRIMARY
- **Hermes admission.** The message id is `['heartbeat', session_id, created_at, fire_count]`. A pending heartbeat admission suppresses a new one. `settle_heartbeat_attempt` refunds the fire if the runner never started. [https://github.com/NousResearch/hermes-agent/blob/e6899c8090da3bd48c829893e4103ce989d3482d/gateway/run_heartbeat_acceptance.py] PRIMARY
- **Hermes wake.** `deliver_wake` raises on failure so callers can retry instead of losing the event. Async-delegation completions are written as durable transcript rows and never self-posted as a user prompt, so they can't cross a human-confirmation gate. [https://github.com/NousResearch/hermes-agent/blob/e6899c8090da3bd48c829893e4103ce989d3482d/gateway/wake.py] PRIMARY
- **OpenClaw gateway protocol.** It is a single WebSocket control plane carrying all clients: CLI, UI, apps, and nodes. The flow is a `connect.challenge` nonce, then a signed `connect` carrying role, scopes, caps, commands and device identity, then `hello-ok`. [https://github.com/openclaw/openclaw/blob/88ef99e66fc9b698c2350774b5ef5dbef3c6c9a4/docs/gateway/protocol/handshake.md] PRIMARY
- **Protocol details.**
  - Roles are `operator`, `node` and `worker`.
  - There are eight operator scopes: read, write, admin, approvals, questions, pairing, talk and talk.secrets.
  - `hello-ok` carries payload and buffer limits, tick interval and attachment ceilings.
  - Node `caps` and `commands` are treated as claims, enforced against server-side allowlists.
  - The worker role runs on a closed protocol with a five-method allowlist. [same] PRIMARY
- **Device auth.** `device.id` is a keypair fingerprint and the signature payload (v3) binds platform and device family. Tokens are issued per device and role, and rotation or revocation is bounded to the approved role set. On `AUTH_TOKEN_MISMATCH`, clients get one bounded retry with a cached device token, only on trusted endpoints. [https://github.com/openclaw/openclaw/blob/88ef99e66fc9b698c2350774b5ef5dbef3c6c9a4/docs/gateway/protocol/auth.md] PRIMARY
- **Node pairing is two layers.** Device pairing gates `connect`, and a separate capability approval (`node.pair.*`) gates the declared commands. [https://github.com/openclaw/openclaw/blob/88ef99e66fc9b698c2350774b5ef5dbef3c6c9a4/docs/gateway/pairing.md] PRIMARY
- **Pairing rules.**
  - An unapproved surface has no effective commands.
  - A widened surface needs fresh approval.
  - Approving `system.run`, `browser.proxy`, `fs.listDir` or exec-approvals commands requires `operator.pairing` plus `operator.admin`.
  - Setup codes are single-use and expire in 10 minutes.
  - SSH-verified auto-approval approves only on a device key match, never on network locality alone.
  - Forwarded headers on loopback disqualify the loopback trust claim. [same] PRIMARY
- **Exec approvals** use a per-agent allowlist with `argPattern`. [https://github.com/openclaw/openclaw/blob/88ef99e66fc9b698c2350774b5ef5dbef3c6c9a4/docs/tools/exec-approvals.md] PRIMARY
  - `allow-always` entries bind exact argv and cwd.
  - `askFallback` defaults to `deny` when no UI is reachable.
  - Approved runs bind the executable's real path, plus a content hash if it is writable. Drift between approval and launch denies the run.
  - Edits to command, cwd or session key after the request is created are discarded.
  - Approval cards can carry a typed scope (message-send, payment, external-post).
- **Standing grants for cron.** "Always allow" mints a grant bound to the agent, job definition, command, cwd and env. It is revalidated on each use, invalidated by job edits, and revocable. It has no automatic expiry unless `grantExpiryDays` is set. Chat channels never receive automation approvals. [same exec-approvals.md] PRIMARY
- **Sandbox, tool policy and elevated are three separate controls.** [https://github.com/openclaw/openclaw/blob/88ef99e66fc9b698c2350774b5ef5dbef3c6c9a4/docs/gateway/sandbox-vs-tool-policy-vs-elevated.md] PRIMARY
  - Sandbox mode is `off`, `non-main` or `all`, with `workspaceAccess` of none, ro or rw.
  - Tool policy is layered, with deny always winning.
  - Elevated is an exec-only escape hatch.
  - Bind sources are validated twice, including after symlink resolution.
- **Skills.** Roots have a seven-tier precedence, with the workspace highest. Skills are gated by `requires.bins`, `env`, `config` and `os`, and agent allowlists replace rather than merge. Eligible skills are compiled into a compact XML block in the prompt, with a character budget and `skills_search` as fallback. Nodes can publish skills only through `node.skills.update`, never in `connect`. [https://github.com/openclaw/openclaw/blob/88ef99e66fc9b698c2350774b5ef5dbef3c6c9a4/docs/tools/skills.md] PRIMARY
- **OpenClaw browser.**
  - It uses Playwright over CDP and takes snapshots with refs. CSS selectors are intentionally unsupported for actions, and refs go stale on navigation.
  - It has a loopback-only HTTP control API behind an environment flag.
  - Screencast uses single-use 60-second tokens bound to the requesting gateway connection.
  - `evaluate` can be disabled, and an SSRF policy blocks private networks. [https://github.com/openclaw/openclaw/blob/88ef99e66fc9b698c2350774b5ef5dbef3c6c9a4/docs/tools/browser-control.md] PRIMARY
- **Hermes browser-control broker.** [https://github.com/NousResearch/hermes-agent/blob/e6899c8090da3bd48c829893e4103ce989d3482d/gateway/browser_control_broker.py] PRIMARY
  - Tickets last 30 seconds and are single-use.
  - A `ControllerScope` is an exact identity (principal, profile, session, controller, browser profile, transport) plus a capability set.
  - Raw CDP and eval are developer-only and re-checked live on every selection.
  - A same-identity reconnect keeps pending work, while `detach` fails pending work closed.
  - Cancel frames are deferred while the controller is offline (cap 512).
  - Artifacts travel as an `artifact_id` checked by a store.
- **Hermes approval floors.** Hardline blocks, user deny globs and `sudo -S` blocks run before yolo, `mode: off` and cron approve-mode. The permanent allowlist is skipped for compound shell commands. [https://github.com/NousResearch/hermes-agent/blob/e6899c8090da3bd48c829893e4103ce989d3482d/tools/approval_floors.py] PRIMARY

**What our Rust gateway has or lacks.** This comes from grep and a partial read of `src/`, so it is inference. Check these before acting on them.
- It has `work_heartbeat.rs` (stale in-progress work items), cron silence tokens (`NO_REPLY`, `[SILENT]`) in `cron/scheduler.rs:233` and `models/events.rs:65`, an `ApprovalScope` enum in `discord/approval.rs`, and `security/hardline.rs`.
- Grep found no hits for `sandbox`, `device_token`, `active_hours` or `ask_fallback`, and no node, device pairing or WebSocket role modules in the file list.

## Design patterns worth copying

1. **Heartbeat as a cron row, not a file.** Store `{agent_id, every, active_hours, tz, target, scratch, scratch_rev, last_tick, cooldown_until}`. Make scratch writes compare-and-swap on `scratch_rev` and cap it at 256 KiB. Let the agent update its scratch through a `heartbeat_respond{notify, text, scratch?}` tool.
2. **Gate before model.** Evaluate in this order, with a skip reason enum for each: `disabled`, `outside-hours`, `empty-scratch`, `busy`, `no-route`, `channel-not-ready`, `alerts-disabled`. Only then spend a model turn. Persist the reason so a status command can show it.
3. **Fire accounting from Hermes.** Record `last_fired_at` and `fire_count` before the turn. Give the admission a deterministic id (`heartbeat:session:created_at:fire_count`) so it is idempotent. Refund the fire if the runner never starts. Coalesce missed ticks to now.
4. **Event wakes as a separate state machine** with a captured `DeliveryRoute{channel, account, thread}`. Apply a per-agent limiter: at least 30 seconds apart and at most 5 per minute. Cancel retained events when the session is reset.
5. **Silence handling.** Treat `NO_REPLY` as a quiet reply. Strip a leading or trailing ack token only when at most 300 characters of content remain. Store silent-but-meaningful results as bounded context for the next user turn.
6. **Node identity and approval.**
   - Use an ed25519 `device_id` as a key fingerprint. Sign a server nonce over `device|client|role|scopes|token|nonce|platform`.
   - Keep node commands filtered until a separate `NodePairing{pending, approved_surface}` is approved.
   - Require admin scope for dangerous commands: `system.run`, `browser.proxy`, `fs.listDir`.
   - Make setup tokens single-use with a short TTL.
   - Auto-approve only on a cryptographic proof, never on IP locality.
7. **Approval rules** (borrowed from the OpenClaw and Hermes designs above).
   - Policy is the stricter of config and the host document.
   - `ask_fallback` defaults to `deny`.
   - An approval stores a prepared plan (argv, cwd, env), and later edits are ignored.
   - `allow-always` binds argv and cwd.
   - Cron "always allow" mints a revocable grant keyed to the job-definition hash and the exact operation.
   - Hardline and deny floors run before any yolo mode.
8. **Layered tool policy.** Use profile, then global, then agent, then provider, then sandbox policy. Deny always wins, and a non-empty allow list blocks everything else. Keep sandbox and elevated as separate concerns.
9. **Browser broker.** Use a `ControllerScope` equality identity, single-use tickets, a live-checked developer gate, fail-closed detach, and refs invalidated on navigation. Make `evaluate` and raw CDP opt-in. Apply an SSRF block list to navigation.
10. **Skills.** Add gating (bins, env, os) and per-agent allowlists, plus a prompt character budget with a search fallback.

## CLAIMS
- CLAIM: OpenClaw no longer reads HEARTBEAT.md at runtime — RISK: low — SOURCES: heartbeat.md, "Migrating from HEARTBEAT.md" note — PRIMARY: yes
- CLAIM: Node commands are disabled until capability approval, separate from device pairing — RISK: low — SOURCES: pairing.md, "Node command gating (2026.3.31+)" — PRIMARY: yes
- CLAIM: The default install has `exec.security=full` and `ask=off` (YOLO-ish) on gateway and node hosts — RISK: med; this is a default-security gap and is worth copying only with a stricter default — SOURCES: exec-approvals.md, `exec.security` and `exec.ask` — PRIMARY: yes
- CLAIM: Hermes heartbeats fire only into an idle session and inject a plain user message — RISK: low — SOURCES: hermes_cli/heartbeat.py docstring — PRIMARY: yes
- CLAIM: Our Rust gateway has no sandbox, node pairing or device tokens — RISK: med; inferred from grep only, and a miss in naming is possible — SOURCES: local grep of `src/` (no hits) — PRIMARY: no (inference)
- CLAIM: The heartbeat doc is dense with fixed policies (busy deferral, ack stripping) that are not configurable — RISK: low — SOURCES: heartbeat.md, "Field notes" note — PRIMARY: yes

## EXPAND
- OpenClaw `docs/gateway/security.md` and `docs/gateway/operator-scopes.md` were not fetched, so the full threat model and named roles are not covered.
- OpenClaw `docs/automation/cron-jobs.md` was not fetched, so cron job schema, isolated sessions and cron delivery are not covered.
- OpenClaw `docs/nodes/*` was not fetched, so node-hosted MCP, session hosting and presence are not covered.
- OpenClaw source under `src/gateway` was not read (docs only). The doc claims are not cross-checked against implementation.
- Hermes `cron/scheduler_tick.py`, `delivery_queue.py` and `gateway/pairing.py` were not read.
- Local gaps need a read of the rest of `src/work_heartbeat.rs`, `src/cron/*`, `src/lanes/*` and `src/multiplexer/*`. Only grep results and the first 120 lines of `work_heartbeat.rs` were examined.
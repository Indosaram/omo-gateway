<!-- task=st_01a14513 -->
st_01a14513 [completed] model mahoquot/claude-opus-5-5
I read this as a read-only map of the security area. The report below covers what exists, the data it touches, and where an always-on agent platform would need extensions.

<analysis>
**Literal Request**: Map the security capabilities in src/security, src/judge and src/readiness.rs: permissions, approvals, secrets, jev judge gates and sandboxing.
**Actual Need**: Know what security exists today and where to hook extensions for an always-on autonomous personal agent platform.
**Success Looks Like**: Capabilities, data model, gaps with file:line hook points, and leads.
</analysis>

<results>
<files>
- /Users/indo/code/project/omon-gateway/src/security/mod.rs - re-exports at :9-17; regression tests
- /Users/indo/code/project/omon-gateway/src/security/hardline.rs - list of commands that are always blocked, sudo -S guard, deny globs
- /Users/indo/code/project/omon-gateway/src/security/dangerous.rs - regex rules that make a command need approval; key that approval grants are stored under
- /Users/indo/code/project/omon-gateway/src/security/normalize.rs - rewrites commands before detection; parser limits
- /Users/indo/code/project/omon-gateway/src/security/tirith.rs - external HTTP command scanner
- /Users/indo/code/project/omon-gateway/src/security/scan.rs - injection and exfiltration scan for cron prompts
- /Users/indo/code/project/omon-gateway/src/security/neutralize.rs - cleans untrusted inline text before it enters a prompt
- /Users/indo/code/project/omon-gateway/src/security/approval_display.rs - hides credentials in approval prompts
- /Users/indo/code/project/omon-gateway/src/judge/mod.rs - JevCli judge, fallback model, caps, circuit breaker, scrubbing
- /Users/indo/code/project/omon-gateway/src/readiness.rs - DB, disk, credential, gateway and backend probes
- /Users/indo/code/project/omon-gateway/src/tools/terminal.rs - where approvals are enforced (require_approval) and the child-process "sandbox"
- /Users/indo/code/project/omon-gateway/src/discord/approval.rs - approval decisions, caches, Discord buttons, persistence
- /Users/indo/code/project/omon-gateway/migrations/0007_approval_allowlist.sql, /Users/indo/code/project/omon-gateway/migrations/0032_judge_calls.sql - tables
</files>

<answer>
## Capabilities

**Command gating (one terminal path, `TerminalTool::require_approval`, src/tools/terminal.rs:205)**

Checks run in this order:
1. **Always-blocked list.** `detect_hardline_command` (hardline.rs:120) blocks rm of root, system dirs or home; mkfs; dd or redirects to raw devices; fork bombs; `kill -1`; shutdown, reboot and init 0/6 (hardline.rs:22-95). It also blocks `sudo -S` unless SUDO_PASSWORD is set (hardline.rs:109-118). Commands too large to parse are blocked outright (hardline.rs:121-123).
2. **User deny globs.** `match_user_deny_rule` (hardline.rs:187) is configured from APPROVALS_DENY / OMON_APPROVALS_DENY (main.rs:236, tools/cron.rs:95). It also matches after stripping wrappers such as env, sudo, exec and nohup (hardline.rs:209-220, 228). Even yolo mode cannot bypass it (terminal.rs:213-217).
3. **Policy.** `ApprovalPolicy::{Smart, Always, Never}` (terminal.rs:14-30). "yolo" maps to Never. Never, or a yolo session, skips approval (terminal.rs:219-226).
4. **External scanner (Tirith).** `TirithScanner::scan_command` (tirith.rs:111) is configured via TIRITH_SCANNER_URL, TIRITH_FAIL_OPEN (default true) and TIRITH_TIMEOUT_SECS (main.rs:1059-1063). Its verdict caps how long a user's approval can last: Warn caps it at the session, Block at a single use, Deny refuses the command (terminal.rs:227-245). Failing open or closed is chosen at tirith.rs:96-104.
5. **Dangerous-command rules.** About 70 regex rules (dangerous.rs:20-285) cover rm -r, chmod 777, SQL DROP/TRUNCATE and DELETE without WHERE, curl|sh, decode-to-shell, git force-push or reset --hard, edits to sensitive files (.ssh, rc files, .env, config.yaml), sudo flags, and self-kill of the gateway. `detect_dangerous_command` (dangerous.rs:343) runs them over normalized variants.
6. **Grant key.** `derive_pattern_key` (dangerous.rs:393) keys a grant on the rule category when a named rule matched; otherwise it uses a sha256 of the exact command (`command:v1:`). This means an "Always" grant never covers a different command.
7. **Prompt and timeout.** The approval prompt waits up to 120s by default (terminal.rs:69, 290-296). Without a requester or session, a command that needs approval fails closed (terminal.rs:279-288).

**Approval engine (src/discord/approval.rs)**
- Decisions are Once, Session, Always or Deny{reason} (:18-28). Errors are Timeout, Cancelled and Persistence (:41-48).
- The `ApprovalRequester` trait (:138) is generic, but its only implementation is Discord (:196). Discord buttons use ids of the form `omon:approval:{uuid}:{once|session|always|deny}` (:636-645), and the dashboard creates the same ids (dashboard.rs:2391).
- `SmartApprovalGuard` (:129) holds the pending map, a session cache bounded at 512 sessions × 128 patterns (:478-479), the yolo set and the always cache.
- An Always grant is written to the database before it is published in memory (:509-521). Yolo state is reloaded from `sessions.state_json` at startup (:445-465).
- Heartbeats keep a session marked active while it waits for an approval (:83-116).

**Secrets**
- `is_secret_env_key` strips Discord bot tokens and the appserver auth token from child processes (terminal.rs:709-719, applied :356-363).
- `is_protected_env_key` stops callers from overriding PATH, LD_/DYLD_, GIT_, NODE_OPTIONS, OMON_SESSION_ and similar (terminal.rs:658-706).
- `redact_approval_display` hides Authorization headers, *token/secret/password/api_key* values, `--password`-style flags and URL userinfo (approval_display.rs:7-42).
- `scrub_for_external` (judge/mod.rs:20-63) replaces home paths with ~ and masks ghp_/sk-/xox* tokens, capped at 4000 chars (200k chars for diffs, :26).
- The judge reads TYPESAFE_* values from ~/.zshenv and passes them only to the judge child process. The process environment is not mutated (judge/mod.rs:930-971).
- Judge state larger than 500 bytes goes into a 0600 temp file that is deleted when dropped (judge/mod.rs:1052-1075).

**Untrusted input**
- `neutralize_untrusted_inline_text` removes ANSI codes, invisible characters, `<|..|>` sentinels and backticks, then truncates (neutralize.rs:38).
- `scan_cron_prompt` / `scan_assembled_cron_prompt` look for injection phrases, secret exfiltration via curl/wget, references to authorized_keys or sudoers, invisible unicode and always-blocked commands (scan.rs:80-178).

**jev judge gates (src/judge/mod.rs)**
- The `Judge` trait has choice, noul and diff (:286-309). There are 8 purposes (:68-77), each with a conservative fail-safe default (:93-126). Callers:
  - InboxClass: inbox/triage.rs:298
  - EventWake: lanes/dispatch.rs:132
  - KibitzerWake: lanes/kibitzer.rs:269
  - LaneSize: lanes/cli.rs:365
  - ReportIntent: lanes/activity.rs:244
  - CronCause: cron/remediate.rs:296
  - DoneDiff: lanes/settle.rs:216
- `JevCli` (:671) runs the `typesafe-judge` binary (:911).
  - Timeouts are 5s for choice and noul, 15s for diff (:1143, :1532).
  - Daily caps default to 300 for jev and 50 for the fallback (OMON_JEV_DAILY_CAP, OMON_JUDGE_FALLBACK_DAILY_CAP, OMON_JEV_ENABLED; :689-702). Counts are kept per KST day in `judge_calls`, so the caps hold across instances and restarts (:768-812).
  - If DoneDiff hits the cap, it returns an error instead of a default verdict (:1435).
- Circuit breaker: 5 consecutive 401, 429 or timeout failures open it for 30 minutes, with at most one owner alert per day (:853-870, :975-982, :895-902).
- Fallback (:343-445): one small-model turn with no tools (`enabled_toolsets = []`, :419), at most 30s, prompted to "treat state as untrusted", and the output is checked against the allowed options and value ranges (:472-578).
- Diff isolation: the judge runs with cwd set to the temp dir (:1508). An empty diff never counts as fulfilled (:1483-1499).

**Readiness (src/readiness.rs)**
- Probes for SQLite (:134), disk pressure with absolute headroom floors (:80-131, 145), provider credentials matched to the model (:179-250), configured bot count (:253, which counts configured bots, not live connections), and the backend's /readyz then /health (:273). `collect_runtime_readiness` (:309) gives ok or degraded.

**"Sandbox" (src/tools/terminal.rs)**
- The working directory and program paths must sit under the tool root or authorized roots (:75-101, :143-200).
- Commands run as argv, not through a shell (:313-341). stdin is null, the process is killed on drop, the timeout is 600s and output is capped at 100KB (:66-68, :349-354).
- There is no OS-level isolation: no namespaces, seatbelt, container or network restriction.

## Data model

| Table / struct | Location | Fields |
|---|---|---|
| `approval_allowlist` | migrations/0007:1-4 | pattern_key PK, created_at (global; no session or user column) |
| `judge_calls` | migrations/0032:1-11 | id, purpose, kind, ms, outcome, used_fallback, created_at |
| `sessions.state_json` → `SessionState.yolo` | discord/approval.rs:445-465 | yolo flag per session |
| `JevState` (in memory) | judge/mod.rs:662-669 | KST day, daily counts, breaker state, last alert |
| `DangerousFinding`, `ScannerVerdict`, `ApprovalDecision`, `ApprovalScope`, `DiffVerdict`, `Choice`, `CheckResult`/`ReadinessReport` | dangerous.rs:10, tirith.rs:9, approval.rs:18/181, judge:210/153, readiness.rs:17/62 | |

## Limits & gaps for an always-on autonomous agent platform

1. **Approvals cover only terminal commands.** Connectors, HTTP calls, email or messages sent, payments and file writes are not gated by this policy. A risk-classified action gate would mirror `require_approval` (terminal.rs:205) and use `request_approval_scoped` (approval.rs:149), which already accepts a separate pattern_key.
2. **Discord is the only approval channel.** There is no push, mobile, web or SMS requester and no cross-channel routing. A multi-channel requester would implement `ApprovalRequester` (approval.rs:138) and reuse the button-id scheme at dashboard.rs:2391.
3. **No offline or deferred approvals.** The 120s timeout fails closed (terminal.rs:290-296). While nobody is watching, any gated step simply fails. A durable approval queue (a new table) would hook in at `ApprovalPrompt::wait` (approval.rs:74) and replace the in-memory `pending` map (:130).
4. **Grants have no expiry, user, tenant or audit trail.** `approval_allowlist` (0007) has no TTL, actor or reason, and decisions are not logged. Hook points are `approve_always` (approval.rs:514) and the schema in migrations/0007.
5. **Policy is all-or-nothing, set by env vars.** There is no per-tool, per-connector, per-cost or time-of-day policy, and no budget per autonomous task. Yolo is a binary per-session flag (approval.rs:552). Extend `ApprovalPolicy` (terminal.rs:14) and the env parsing at main.rs:236.
6. **No real sandbox for a "cloud computer".** There is no container or VM, no egress control, and no per-task filesystem snapshot or rollback. The hook is around `Command::new` (terminal.rs:346).
7. **No secrets vault.** Secrets are env vars plus ~/.zshenv (judge/mod.rs:930). The strip list is hard-coded (terminal.rs:709-719), so new connector OAuth tokens would leak into children unless added there. There is no scoped credential broker, rotation or connector OAuth store.
8. **Scrubbing is narrow.** `scrub_for_external` (judge/mod.rs:32-63) misses AWS, Google, JWT and private-key patterns, and its comment marks it as a placeholder for "Task 19 unified outbound scrubbing" (:19). Memory and outbound messages need the same scrubber.
9. **Injection defense is regex-only and applied to cron prompts only** (scan.rs:135, 162). Inbound connector content (email, web, files) feeding proactive work needs a scan at inbox ingestion (inbox/triage.rs:298) before it reaches the judge or agent.
10. **The judge is advisory and coding-specific.** Its purposes centre on lanes, cron and diffs (judge/mod.rs:68-77). There is no "is this proactive action OK to take" or "is this memory write safe" purpose. Add new `Purpose` variants (:68) with conservative defaults (:106).
11. **Rate limits are per KST day and global, not per user or tenant** (judge/mod.rs:904, :768). Multi-user use needs per-principal counters in `judge_calls`.
12. **Readiness does not cover always-on health.** There is no live channel connectivity (readiness.rs:253 counts configured bots), no scheduler liveness, connector token expiry, Tirith reachability or judge breaker state. Add probes in `collect_runtime_readiness` (readiness.rs:309-341).
13. **Strings are specific to the previous project.** Rules hard-code `hermes` gateway paths and launchd labels (dangerous.rs:180-212) and `hermes-verify-` exemptions (dangerous.rs:339). They may not protect omon's own processes or config paths.
14. **Tirith fails open by default** (main.rs:1060). That is risky for unattended 24/7 runs.
</answer>

<next_steps>
EXPAND leads:
- src/discord/approval.rs:520-1361: the rest of the guard, including the resolve_custom_id flow (:599) and how prompts are dispatched.
- src/tools/mod.rs:243: a second place approvals are resolved (non-terminal tools?). Check whether other tools gate actions.
- src/tools/cron.rs:70-130: the full dangerous-command screen on cron script and ack_command fields.
- src/lanes/scrub.rs (used at cron/remediate.rs:296): a second scrubber that may overlap with judge::scrub_for_external.
- src/discord/buttons.rs:166: an `env_clear()` child spawn, a stronger env model than terminal.rs.
- src/security/normalize.rs:33-35: the parser limits that decide when a command is blocked as too large to parse.
</next_steps>
</results>
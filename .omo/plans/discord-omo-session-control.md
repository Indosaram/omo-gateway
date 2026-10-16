# discord-omo-session-control - Work Plan

## TL;DR (For humans)

**Who this is for and what changes for them:** You (the owner) on Discord. Today you can ask "세션 상태" and get one useless line ("live 14"), and anything beyond that — reading, instructing, stopping a session in another project or on omarchy — silently fails. After this, one Discord command shows every session on every machine with what it is doing right now, what it last said and whether it is waiting for you, and you can instruct, stop or answer any of them from Discord; questions and finished turns you care about are pushed to your DM.

**What you'll get:** a `/sessions` command family (overview, read, send, stop, answer, watch) usable only by you; the same actions through the OmOMeow agent in natural language; automatic DM notices when any session asks you a question and when a watched session finishes a turn.

**Why this approach:** sessions already expose a local control socket with live state, interrupt and question answering; one small embedded helper reads it on each machine in a single round-trip (locally or over ssh), so no new daemon is installed anywhere and remote and local behave the same. Notifications use a cheap local check every minute with no AI calls.

**What it will NOT do:** no deploy or restart of the live gateway in this run (you asked to approve state changes yourself); no killing of session processes; nothing about sessions is ever shown in shared channels.

**Effort:** Large
**Risk:** Medium - touches the live session control path; mitigated by owner-only gates and fake-runner tests plus real CLI QA on throwaway sessions.
**Decisions I made for you:** finished-turn notices only for sessions you watch (sending from Discord auto-watches); questions always notify; slash replies are private (ephemeral) when used in a server; a question that blocks the terminal UI is reported as "answer in the terminal" because the engine does not accept remote answers for it.

Your next move: nothing - execution follows immediately (pre-approved in the request). Full execution detail follows below.

---

> TL;DR (machine): Large, Medium risk; deliver probe helper, control fixes, overview, CLI, /sessions slash family, watcher notices, adopt fix, lead PATH + prompt.

## Scope
### Affected user and ideal state
**Affected users:** (U1) owner in Discord DM / private owner thread (types "/sessions ..." or asks OmOMeow); (U2) the lead OMO session which runs `omo-gateway sessions ...` through bash; (U3) the controlled omo sessions (TUI and daemon) on local and omarchy, which must not be disrupted; (U4) any other Discord user, who must see and control nothing.

| Row | Statement | Reason |
| --- | --- | --- |
| IS-1 | One owner command lists every session on every configured machine (local + session-hosts.json hosts) grouped by machine, each with state (working / waiting for answer / idle / unreachable), name, home-relative folder, KST-relative last activity and a short snippet of what it last said | "모든 omo세션 작업 내용을 파악" - status "live" alone says nothing |
| IS-2 | Reading any session's recent transcript works for sessions in any workspace and on any host | the owner manages all sessions, not only the gateway repo's |
| IS-3 | Sending an instruction (auto / steer / follow-up, multi-line allowed) works for any session on any host and is idempotent | "추가 지시를 받아서 관리" |
| IS-4 | Stopping (interrupting) the running turn of any session works without killing its process | management includes stop |
| IS-5 | A pending async question (ask_user_question) of any session is shown with its options and can be answered from Discord; a terminal-blocking question is shown with "answer in the terminal" | sessions that wait on the owner are the main thing to manage remotely |
| IS-6 | The owner is DM'd once per new pending question of any session, and once per finished turn of a session he watches; no model call, no repeats after gateway restart | "알림" without channel fatigue (memory: event-based, no polling tokens) |
| IS-7 | Targets accept a unique prefix or session name (`local:01a13e55`), with a clear error on ambiguity or absence | typing 36-char ids on a phone is unusable |
| IS-8 | Only the owner can use any of it; in shared channels nothing about sessions is posted publicly (ephemeral), non-owners get a refusal and nothing executes | session content is private (existing prompt rule) |
| IS-9 | The lead OMO session can run every `omo-gateway sessions` subcommand (binary resolvable on its PATH) and its prompt describes the new commands. Delivery in this run is code- and file-level and is proven on a scratch copy of the live DB; switching the LIVE gateway (deploy + restart, `profile apply-block` on the live DB) is a state change the owner runs himself, given as exact commands in the final report | natural-language management through OmOMeow must work; owner rule forbids autonomous state changes |
| IS-10 | Adopting an external session as a lane works against the real `omo thread read` output | the existing adopt path is part of "관리" |
| IS-11 | One unreachable host never blocks or hides the others; every call is bounded (20 s per host) | omarchy may be offline |
| IS-12 | Existing behaviour (status intercept phrases, lanes, other slash commands, fmt/clippy/tests) does not regress | regression-free |

| Row | Statement | Reason |
| --- | --- | --- |
| GAP-1 | status_reply only itemizes busy/working; real status is always "live" -> one count line, no content (src/sessions/status_reply.rs:118-187; F7) | IS-1 |
| GAP-2 | read/send omit `--all-scope` -> scope_denied for every non-repo workspace (src/sessions/control.rs read/send; F1) | IS-2, IS-3 |
| GAP-3 | remote multi-line send refused although ssh word quoting preserves newlines (control.rs RemoteMultiline; F5) | IS-3 |
| GAP-4 | interrupt calls non-existent `omo thread interrupt` (F2) | IS-4 |
| GAP-5 | no way to see or answer pending questions (F3, F4) | IS-5 |
| GAP-6 | no notifications for non-adopted sessions | IS-6 |
| GAP-7 | targets need full exact ids | IS-7 |
| GAP-8 | no Discord slash surface for sessions (src/discord/commands.rs:178 all()) | IS-1..IS-5, IS-8 |
| GAP-9 | lead session PATH lacks ~/.omon/bin (F9); prompt describes old limits (bot_profiles/blocks/lanes.md "Session status and control"); the runtime prompt is the `bot_profiles` DB row, written only by `omo-gateway profile apply-block` (src/entry.rs:172-176) | IS-9 |
| GAP-10 | external.rs expects nextCursor and items[].id; real read has items[].seq and next_cursor only when truncated (F8) | IS-10 |

### Must have
- Every IS row delivered and proven by its scenario in ## Success criteria.
- Owner gate on every new Discord entry point; ephemeral replies outside DMs.
- Bounded timeouts; per-host error isolation.
- Additive edits to dirty files (src/main.rs, src/agent/omo_daemon.rs): never revert or reformat unrelated uncommitted hunks.

### Must NOT have (guardrails, anti-slop, scope boundaries)
- No killing of session processes, no `omo host session close` of user sessions, no model calls in the watcher.
- No new external dependency crates; no new daemon on remote hosts; no files installed on remote hosts (helper is passed inline).
- No deploy, no launchd change, no edit of the production DB or bot_profiles rows in this run (activation is proven on a scratch DB copy and handed to the owner as exact commands).
- No posting session content to shared channels; no change to existing status-intercept phrases' eligibility rules.
- Workers do NOT run cargo build/test/clippy/fmt (owner rule); the lead runs one integration gate at the end.

## Verification strategy
> Zero human intervention - all verification is agent-executed.
- Test decision: tests-after, Rust unit tests inside each new module with fake CommandRunner, plus updated tests/test_sessions_control.rs. Written by workers, executed once by the lead.
- Integration gate (lead, once): `cargo fmt --check`; `cargo clippy --all-targets -- -D warnings`; `cargo test --lib sessions::`; `cargo test --lib discord::commands`; `cargo test --lib agent::omo_daemon::tests::test_daemon_command_arguments`; `cargo test --test test_sessions_control --test test_session_registry --test test_session_registry_task27`.
- Real-surface QA (lead, after gate) with a release binary copied to a fixed path, against the real local sessions and omarchy, using throwaway daemon sessions for every mutating action. Discord itself is not driven (owner rule: no direct Discord QA); the Discord handlers are covered by unit tests that call the same functions the slash commands call.
- Evidence: .omo/evidence/discord-omo-session-control/

## Execution strategy
### Parallel execution waves
- Wave 1 (6 quick lanes, disjoint files): 1 probe helper; 2 control fixes; 3 adopt read-shape fix; 4 lead PATH; 5 migration + lead prompt block; 6 overview formatter + status reply.
- Wave 2 (3 quick lanes, disjoint files; read Wave 1 code): 7 CLI; 8 Discord /sessions; 9 watcher + gateway spawn.
- Wave 3: 10 static cross-file consistency review (quick, read-only report) -> lead integration gate + QA.

### Dependency matrix
| Todo | Depends on | Blocks | Can parallelize with |
| --- | --- | --- | --- |
| 1 | - | 2(interrupt),6,7,8,9 | 2-6 (API pinned below) |
| 2 | - | 7,8 | 1,3-6 |
| 3 | - | - | all |
| 4 | - | - | all |
| 5 | - | 9 | all |
| 6 | - | 7,8,9 | 1-5 |
| 7 | 1,2,6 | 10 | 8,9 |
| 8 | 1,2,6 | 10 | 7,9 |
| 9 | 1,5,6 | 10 | 7,8 |
| 10 | 1-9 | gate | - |

### Pinned interfaces (every lane codes against exactly these)
```rust
// src/sessions/probe.rs
pub const PROBE_PY: &str = include_str!("probe.py");
pub const PROBE_TIMEOUT: Duration = Duration::from_secs(20);
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct QuestionItem { pub id: String, pub header: Option<String>, pub question: Option<String>, pub options: Vec<String> }
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PendingQuestion { pub request_id: String, pub questions: Vec<QuestionItem> }
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct LiveSession {
    #[serde(default)] pub machine: String,       // filled by Rust, not python
    pub session_id: String,                     // durable id
    pub routing_id: String,                     // id to address the endpoint (tui: durable id; daemon: e.g. "rpc-2")
    pub endpoint_kind: String,                  // "tui" | "rpc_host"
    pub name: Option<String>, pub cwd: Option<String>,
    pub streaming: bool, pub compacting: bool, pub blocking_question: bool,
    pub pending_messages: u64, pub message_count: u64,
    pub last_user: String, pub last_assistant: String,
    pub questions: Vec<PendingQuestion>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SnapshotError { pub endpoint: String, pub error: String }
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct MachineSnapshot { #[serde(default)] pub machine: String, pub sessions: Vec<LiveSession>, pub errors: Vec<SnapshotError> }
#[derive(Debug, thiserror::Error)]
pub enum ProbeError { #[error("probe timed out")] Timeout, #[error("probe transport failed: {0}")] Transport(String),
    #[error("session not found on this machine")] NotFound, #[error("session refused: {0}")] Refused(String),
    #[error("invalid probe output: {0}")] InvalidJson(String) }
pub struct Probe<R = ProcessCommandRunner> { runner: Arc<R> }
impl<R: CommandRunner + 'static> Probe<R> {
    pub fn new(runner: Arc<R>) -> Self;
    pub fn argv(machine: &str, args: &[String]) -> Vec<String>;   // full local argv or ssh argv via control::transport_argv
    pub async fn snapshot(&self, machine: &str, text_limit: usize) -> Result<MachineSnapshot, ProbeError>;
    pub async fn snapshot_all(&self, machines: &[String], text_limit: usize) -> Vec<(String, Result<MachineSnapshot, ProbeError>)>; // concurrent, input order kept
    pub async fn interrupt(&self, machine: &str, session: &str) -> Result<(), ProbeError>;  // session = durable id, routing id or name
    pub async fn answer(&self, machine: &str, session: &str, request_id: &str, answers: &serde_json::Value, comment: Option<&str>) -> Result<(), ProbeError>;
}
pub fn answers_for(question: &PendingQuestion, text: &str) -> serde_json::Value; // maps owner text to {qid:{selected:[label]|[],text?}}

// src/sessions/control.rs (additions)
ControlError::Ambiguous(String)   // "#[error(\"target matches several sessions: {0}\")]", exit_code 2
ControlError::Probe(String)       // "#[error(\"session control failed: {0}\")]", exit_code 5
impl SessionControl<R> {
    pub async fn resolve(&self, target: &Target) -> Result<(Target, Session), ControlError>; // exact id, durableSessionId, unique prefix of either, or exact name
    pub async fn interrupt(&self, target: &Target) -> Result<Value, ControlError>;          // via Probe, returns {"ok":true,"target":...}
    pub async fn answer(&self, target: &Target, text: &str, request_id: Option<&str>) -> Result<Value, ControlError>;
    pub fn runner(&self) -> Arc<R>;
}

// src/sessions/overview.rs
pub const DISCORD_LIMIT: usize = 1900;
pub struct Overview { pub list: SessionList, pub snapshots: Vec<(String, Result<MachineSnapshot, String>)> }
pub fn state_of(s: &LiveSession) -> SessionState;          // Working | Question | Blocked | Idle
pub fn format_overview(o: &Overview, now: DateTime<Utc>, max_chars: usize) -> String;
pub fn format_detail(target: &str, s: &LiveSession, transcript_tail: &[String], max_chars: usize) -> String;
pub async fn collect<R: CommandRunner + 'static>(control: &SessionControl<R>, text_limit: usize) -> Overview;

// src/sessions/watch.rs
pub async fn watch(pool: &SqlitePool, target: &str) -> Result<()>; pub async fn unwatch(pool: &SqlitePool, target: &str) -> Result<bool>;
pub async fn watched(pool: &SqlitePool) -> Result<Vec<String>>;
pub struct Notice { pub dedupe_key: String, pub target: String, pub kind: String /* question|blocked|completion */, pub text: String }
pub struct Watcher<R> { .. } impl Watcher<R> { pub fn new(pool, control: SessionControl<R>, interval: Duration) -> Self;
    pub async fn tick(&mut self) -> Result<Vec<Notice>>;   // returns notices not yet sent; records them as sent only via mark_sent
    pub async fn mark_sent(&self, n: &Notice) -> Result<()>;
    pub async fn serve(self, dispatcher: Arc<dyn OutboundDispatcher>, owner_dm: SessionKey, shutdown: CancellationToken) -> Result<()>; }
```

## Todos
> Implementation + Test = ONE todo. Never separate.
<!-- APPEND TASK BATCHES BELOW THIS LINE WITH edit/apply_patch - never rewrite the headers above. -->
- [ ] 1. Embedded session-socket probe helper (python3) and Rust wrapper
  What to do: create src/sessions/probe.py and src/sessions/probe.rs exactly per Pinned interfaces; add `pub mod probe; pub mod overview; pub mod watch;` to src/sessions/mod.rs. probe.py modes: `snapshot <limit>`, `interrupt <session>`, `answer <session> <request_id> <answers_json> [comment]`; prints ONE JSON value to stdout; exit 0 ok, 3 not found, 4 refused, 5 other. Endpoints: every ~/.omo/agent/rpc/tui/*.sock (write bytes of <sock>.secret first) and ~/.omo/agent/rpc/rpc.sock (no secret; list_sessions then per-session calls with sessionId=routing id). Frame: newline JSON {type,id,...}; reply matched by id; 4 s per call, 8 s for get_messages. Questions = assistant toolCall blocks named ask_user_question whose toolResult has details.status=="pending" and no later user text starting "[Answer to question <id>]"; question ids: arguments.questions[i].id or "q{i+1}". Text of a message = string content or joined text blocks. Rust invokes `python3 -c "import base64,sys;exec(base64.b64decode(sys.argv.pop(1)))" <b64(PROBE_PY)> <mode> <args...>` through control::transport_argv (ssh per-word quoting) and CommandRunner.run("local", argv) with PROBE_TIMEOUT.
  Must NOT do: no shell strings, no file writes on the target, no other socket commands than get_protocol_info/list_sessions/get_state/get_messages/interrupt/extension_ui_response.
  Closes: GAP-1, GAP-4, GAP-5
  Parallelization: Wave 1 | Blocked by: - | Blocks: 7,8,9
  References: senpi dist/modes/interactive/session-control-commands.js (commands, answerQuestion); omo-ai plugin/runtime/thread-sdk/sdk.js (secret-then-frame client); src/sessions/control.rs transport_argv; src/sessions/registry.rs CommandRunner/ProcessCommandRunner; draft findings F3,F4,F6.
  Acceptance: unit tests in probe.rs with a fake CommandRunner: (a) local argv starts ["python3","-c",...] and remote argv starts ["ssh","-o","BatchMode=yes",...,"omarchy"]; (b) snapshot parses a scripted JSON into MachineSnapshot with machine filled; (c) exit 3 -> ProbeError::NotFound, timeout -> Timeout; (d) answers_for maps a text equal to an option label to selected=[label] and other text to {selected:[],text}.
  QA: happy - lead runs `omo-gateway sessions overview --json` (todo 7) and sees every local+omarchy session with streaming flags; failure - `omo-gateway sessions interrupt local:does-not-exist` exits 3. Evidence .omo/evidence/discord-omo-session-control/task-1.txt
  Commit: N (single final commit per owner rule)
  Recommended task executor category: quick - two new files following a fully pinned spec.

- [ ] 2. Control layer fixes: all-scope, multiline remote, real interrupt, answer, target resolution
  What to do: in src/sessions/control.rs add "--all-scope" to read() and send() argv (after the subcommand args, before "--json"); delete the RemoteMultiline refusal in send() (keep the enum variant only if referenced elsewhere; otherwise remove it and its exit-code arm); add ControlError::Ambiguous and ::Probe; implement resolve(), interrupt() (resolve, check "interrupt" control, then Probe::interrupt with the listing's sessionId), answer() (resolve; Probe snapshot of that machine; pick the session's pending question by request_id or the only one; error "no pending question" otherwise; Probe::answer with answers_for), runner(). read()/send() call resolve() first and use the resolved session id. Update tests/test_sessions_control.rs: expected argv now contain "--all-scope"; replace remote_multiline_is_refused_without_any_command with a test that a remote multi-line send reaches the runner with the newline preserved inside one quoted word; add tests for resolve (unique prefix ok, ambiguous -> Ambiguous, missing -> NotRunning).
  Must NOT do: no change to transport_argv quoting; no change to Target::from_str charset.
  Closes: GAP-2, GAP-3, GAP-4, GAP-7
  Parallelization: Wave 1 | Blocked by: - (Probe API pinned) | Blocks: 7,8
  References: src/sessions/control.rs (whole); tests/test_sessions_control.rs:71-200; findings F1,F2,F5.
  Acceptance: tests compile against pinned API; `cargo test --test test_sessions_control` green at the gate.
  QA: happy - `omo-gateway sessions read local:01a13a7d --limit 2` (other workspace) returns kind ok; failure - `omo-gateway sessions read local:01a1` (ambiguous prefix) exits 2 with "matches several sessions". Evidence task-2.txt
  Commit: N
  Recommended task executor category: quick - single module edit plus its test file.

- [ ] 3. Adopt works with the real thread read shape
  What to do: in src/sessions/external.rs make next_cursor() fall back to "next_cursor" then to the max items[].seq as string; make assistant_items() accept items with "seq" (id = seq as string) and content that is a JSON-encoded string of blocks (parse it, then extract text blocks). Read requests: the stored cursor is a seq number string -> in read_new, when the stored cursor parses as an integer, call control.read(target, 100, None) and drop items with seq <= cursor instead of passing it as --cursor. Add unit tests for both shapes.
  Closes: GAP-10
  Parallelization: Wave 1 | Blocks: -
  References: src/sessions/external.rs:228-300, 560-610; finding F8 (real shape {kind,thread_id,items:[{seq,role,content}],truncated,source}).
  Acceptance: new unit tests in external.rs tests module.
  QA: happy - `omo-gateway sessions adopt` against a throwaway daemon session with a scratch DATABASE_URL succeeds and prints a key; failure - adopting a nonexistent target exits 3. Evidence task-3.txt
  Commit: N
  Recommended task executor category: quick - one file.

- [ ] 4. Lead session can resolve omo-gateway on PATH
  What to do: in src/agent/omo_daemon.rs daemon_command(), set the child env PATH to "<HOME>/.omon/bin:<dir of std::env::current_exe()>:" + the inherited PATH (skip entries that are missing; no duplicates). Add a unit test asserting the PATH env of the built command starts with those two dirs (use as_std().get_envs()). Keep the existing args test unchanged.
  Must NOT do: touch any other hunk of this dirty file.
  Closes: GAP-9
  Parallelization: Wave 1
  References: src/agent/omo_daemon.rs:414-470, 1520-1542; finding F9.
  QA: happy - after a gateway restart (owner-run deploy) the app-server env PATH contains ~/.omon/bin (`ps eww <app-server pid>`); reported as follow-up since deploy is out of scope; unit test proves the command env. Evidence task-4.txt
  Commit: N
  Recommended task executor category: quick - one function.

- [ ] 5. Watch tables migration and lead prompt block
  What to do: add migrations/0043_session_watch.sql: `CREATE TABLE session_watches (target TEXT PRIMARY KEY, added_at DATETIME NOT NULL, reason TEXT NOT NULL DEFAULT 'manual'); CREATE TABLE session_watch_notices (dedupe_key TEXT PRIMARY KEY, target TEXT NOT NULL, kind TEXT NOT NULL, sent_at DATETIME NOT NULL);` Rewrite the "## Session status and control" section of bot_profiles/blocks/lanes.md (keep markers and every other section byte-identical) to: overview via `omo-gateway sessions overview` (text is Discord-ready; reply with it verbatim), details via `sessions read <machine>:<id-prefix> --limit 10`, send via `sessions send ... --text` (multi-line allowed), stop via `sessions interrupt`, answer via `sessions answer <target> --text <answer>`, notifications via `sessions watch|unwatch <target>`; privacy rule unchanged; mention the owner can also use /sessions.
  Closes: GAP-6 (storage), GAP-9 (prompt)
  Parallelization: Wave 1
  References: migrations/0040_external_sessions.sql (style); bot_profiles/blocks/lanes.md "## Session status and control".
  QA: happy - gate tests run sqlx::migrate! over all migrations; `grep -c "omon:lanes:begin" bot_profiles/blocks/lanes.md` = 1; activation proof on a scratch copy of the live DB: `cp omon_gateway.db* $SCRATCH/ && DATABASE_URL=sqlite://$SCRATCH/omon_gateway.db <bin> profile apply-block --bot-id 1465631383862120451 --block-file bot_profiles/blocks/lanes.md` exits 0 and `sqlite3 $SCRATCH/omon_gateway.db "select instr(system_prompt,'sessions overview')>0 from bot_profiles where bot_id='1465631383862120451'"` = 1; failure - the live DB row is byte-identical before and after QA (sha256 of the row). Evidence task-5.txt
  Commit: N
  Recommended task executor category: quick - SQL + prose.

- [ ] 6. Overview and detail formatter; status intercept uses it
  What to do: create src/sessions/overview.rs per pinned API. format_overview: per machine header "🖥️ <machine> — 작업 중 N · 답변 대기 N · 대기 N"; then one line per working/question/blocked session: "🟢|❓|⛔ <name 32> · <home-relative cwd 36> · <KST relative activity> · `<machine>:<first 8 of id>`" followed by "  ↳ <last assistant snippet 90 chars, newlines collapsed>" (question sessions show the first question text and options instead); idle sessions listed compactly "⚪ <name 28> · `<machine>:<id8>`" up to 8, then "… 외 N개 대기"; unreachable host "⚠️ <machine>: <error 70>"; stale listings marked; total output never exceeds max_chars (truncate whole lines, append "… (일부 생략)"). Sessions present in the listing but absent from the snapshot are shown as idle with listing data. Matching listing<->snapshot by session_id or durableSessionId (Session.raw["durableSessionId"]). format_detail: header line, state, cwd, questions with options, then transcript tail lines, capped. collect(): list_all + Probe::snapshot_all over the listing's machines (always include "local"), errors as strings. In src/sessions/status_reply.rs make reply() use collect()+format_overview (keep matches_intent/eligible/owner_* unchanged, keep REPLY_TIMEOUT bound and the timeout message); keep format_reply but unused code must not trigger dead_code warnings (remove it if no other caller; update its tests accordingly).
  Closes: GAP-1
  Parallelization: Wave 1 (uses pinned probe types)
  References: src/sessions/status_reply.rs (compact, home_relative, activity helpers - move or reuse); src/sessions/registry.rs Session/SessionList; findings F7.
  Acceptance: unit tests: grouping, Korean counts, question rendering with options, idle overflow "외 N개", unreachable host line, max_chars respected for 200 sessions.
  QA: happy - `omo-gateway sessions overview` prints local and omarchy groups with snippets; failure - with OMON_SESSION_HOSTS=nonexistent-host the overview still prints local plus one ⚠️ line within 25 s. Evidence task-6.txt
  Commit: N
  Recommended task executor category: quick - one pure module + one call-site swap.

- [ ] 7. CLI subcommands overview / answer / watch / unwatch / watched / watch-tick
  What to do: extend src/sessions/cli.rs: `overview [--json] [--max-chars N=1900]` (json = Overview serialized: listing + snapshots), `answer <target> --text <t> [--request <id>]`, `watch <target>`, `unwatch <target>`, `watched`, `watch-tick [--once]` (one Watcher tick against DATABASE_URL; prints notices as JSON lines and marks them sent; for QA and cron-free diagnosis). read/send/interrupt go through resolve (todo 2). watch stores the RESOLVED canonical target "<machine>:<session_id>". Exit codes via ControlError::exit_code.
  Closes: GAP-5, GAP-6, GAP-7 (CLI surface for the lead)
  Parallelization: Wave 2 | Blocked by: 1,2,6 (and 9's watch API - code against pinned signatures)
  References: src/sessions/cli.rs; src/entry.rs:65,117,144.
  QA: see Success criteria rows IS-1..IS-7. Evidence task-7.txt
  Commit: N
  Recommended task executor category: quick - one CLI module.

- [ ] 8. Discord /sessions slash family (owner-only)
  What to do: new src/discord/session_commands.rs with `#[poise::command(slash_command, subcommands("overview","read","send","stop","answer","watch","unwatch"))] pub async fn sessions(...)` and the subcommands. Each subcommand first checks owner: ctx.author().id.to_string() == crate::sessions::status_reply::owner_id(); otherwise reply ephemeral "이 명령은 소유자만 사용할 수 있어요." and return without executing. ephemeral = ctx.guild_id().is_some(). defer (ephemeral when in guild) then run the same library functions the CLI uses (SessionControl::from_environment, overview::collect/format_overview, control.read + overview::format_detail, control.send with SendMode from an optional choice auto|steer|follow_up, control.interrupt, control.answer, watch::watch/unwatch using ctx.data().pool). send auto-watches the target. Replies chunked with chunk_slash_reply(…, 1900). Errors are shown as one line "⚠️ <error>". Register: add `super::session_commands::sessions()` to all() in src/discord/commands.rs, `pub mod session_commands;` in src/discord/mod.rs, update test_all_commands_count to 19 and include "sessions". Put the owner check in a pub fn `owner_allowed(author_id: u64) -> bool` with unit tests (owner true, other false).
  Closes: GAP-8
  Parallelization: Wave 2 | Blocked by: 1,2,6
  References: src/discord/commands.rs:178-200 (all), 942-972 (status/tools pattern), 1501 chunk_slash_reply, 351 is_pairing_operator; poise 0.6 subcommands.
  QA: unit tests (owner gate, ephemeral flag fn `reply_ephemeral(guild: bool)`); runtime Discord not driven (owner rule). Evidence task-8.txt
  Commit: N
  Recommended task executor category: quick - one new module plus two registration lines.

- [ ] 9. Session watcher: question and completion notices to the owner DM
  What to do: new src/sessions/watch.rs per pinned API. tick(): collect snapshots of all machines (Probe::snapshot_all over SessionControl registry machines). For every session: each pending question request_id -> Notice{kind:"question", dedupe_key:"q:<machine>:<session_id>:<request_id>", text:"❓ <machine> · <name>: <first question 160>\n선택지: a / b\n→ /sessions answer target:<machine>:<id8> text:…"}; blocking_question true -> kind "blocked", key "b:<machine>:<id>:<message_count>", text says answer in the terminal; for WATCHED targets only: previous tick streaming && now !streaming -> kind "completion", key "c:<machine>:<id>:<message_count>", text "✅ <machine> · <name> 턴 완료\n↳ <last_assistant 300>". First tick after start only records streaming baselines (no completion notices); question/blocked notices are deduped by session_watch_notices so restarts never repeat. A host error never aborts the tick. serve(): every interval (60 s; env OMON_SESSION_WATCH_INTERVAL_SECS) tick, dispatch each notice as OutboundAction::SendMessage{session: owner_dm, content, reply_to: None}, mark_sent only after dispatch Ok; stop on shutdown. Kill switch OMON_SESSION_WATCH_ENABLED (default true). In src/main.rs, next to the existing `discord_egress_dispatcher` (line ~1178) spawn the watcher when enabled with owner_dm = SessionKey::from_storage_key(&default_lead()), the dispatcher, and a shutdown token already used there; additive edit only.
  Closes: GAP-6
  Parallelization: Wave 2 | Blocked by: 1,5,6
  References: src/main.rs:1170-1190, 2209-2222 (spawn pattern), 698-702 default_lead; src/lanes/kibitzer.rs (serve loop pattern); migrations/0043 (todo 5).
  Acceptance: unit tests with fake runner + in-memory sqlite (sqlx::migrate!("./migrations")): new question -> 1 notice, second tick -> 0; watched streaming->idle -> completion; unwatched -> none; first tick no completion; host error -> other hosts still produce notices; mark_sent persists.
  QA: `omo-gateway sessions watch-tick --once` with scratch DB while a throwaway session has a pending question prints exactly one question notice; second run prints none. Evidence task-9.txt
  Commit: N
  Recommended task executor category: quick - one module + additive spawn.

- [ ] 10. Static cross-file consistency review (read-only)
  What to do: read every file changed by todos 1-9 and report any signature mismatch against the pinned interfaces, missing imports, unused items that would fail clippy -D warnings, unregistered modules, test count drift. Report only; no edits; no cargo.
  Parallelization: Wave 3 | Blocked by: 1-9
  Recommended task executor category: quick - read-only checklist.

## Final verification wave
> Runs after ALL todos. Lead-run, binary PASS/FAIL each.
- [ ] F1. Plan compliance audit - every todo's files changed, guardrails respected (`git diff --stat` vs plan; no edits outside listed files except the gate's fixes).
- [ ] F2. Code quality gate - cargo fmt --check, clippy -D warnings, the listed cargo test targets all exit 0.
- [ ] F3. Real manual QA - CLI scenarios in ## Success criteria against real local + omarchy sessions with throwaway sessions for mutations; cleanup receipts.
- [ ] F4. Ideal-state fidelity - each IS row checked 1:1 against its evidence; any shortfall becomes a new numbered todo. IS-9 passes only with the scratch-DB activation proof AND the owner activation commands present in the final report; it is reported as "live activation pending owner" and never as live-active.

## Commit strategy
Owner rule: single commit after full verification, and no commit/push without explicit owner instruction -> stage nothing automatically; report the draft message `feat(sessions,discord): manage every omo session on every machine from Discord`.

## Success criteria
| IS | Delivering todo(s) | Proving QA scenario | Evidence |
| --- | --- | --- | --- |
| IS-1 | 1,6,7,8 | `<bin> sessions overview` shows local + omarchy groups with state and snippets; PASS = both machine headers and >=1 snippet line | task-6.txt |
| IS-2 | 2 | `<bin> sessions read local:<other-workspace id8> --limit 2` -> JSON kind ok (was scope_denied); `<bin> sessions read omarchy:<id8> --limit 1` -> kind ok | task-2.txt |
| IS-3 | 2 | throwaway daemon session on local and on omarchy: `sessions send <t> --text "line1\nline2"` -> ok, transcript shows both lines | task-2.txt |
| IS-4 | 1,2 | throwaway session given a long task, `sessions interrupt <t>` -> ok and probe shows streaming=false within 10 s | task-1.txt |
| IS-5 | 1,2,7 | throwaway session asked to call ask_user_question(waitForAnswer=false) -> overview shows ❓ with options; `sessions answer <t> --text <label>` -> ok; transcript shows "[Answer to question" | task-7.txt |
| IS-6 | 9 | `sessions watch-tick --once` (scratch DB) twice during the pending question: first prints 1 question notice, second prints 0; unit tests for completion | task-9.txt |
| IS-7 | 2 | prefix resolves; ambiguous prefix exits 2 | task-2.txt |
| IS-8 | 8 | unit tests: owner_allowed false for non-owner; ephemeral in guild | gate log |
| IS-9 | 4,5 | unit test on daemon_command env PATH; scratch-DB `profile apply-block` puts the new section into the wawabot row (= 1) while the live row hash is unchanged; live activation = owner-run commands listed in the final report (deploy via scripts/deploy.sh, then `profile apply-block --bot-id 1465631383862120451 --block-file bot_profiles/blocks/lanes.md`) | gate log, task-5.txt |
| IS-10 | 3 | adopt a throwaway session with scratch DB -> key printed, external_sessions row with numeric cursor | task-3.txt |
| IS-11 | 6 | OMON_SESSION_HOSTS=omarchy,nonexistent-qa-host overview returns within 25 s with ⚠️ line and other groups | task-6.txt |
| IS-12 | all | gate: fmt, clippy, listed tests exit 0 | gate log |


## Execution evidence (appended 2026-10-16T03:59:39.658Z)
Evidence dir: .omo/evidence/discord-omo-session-control/ (QA binary built from working tree, never deployed)
| IS | Status | Evidence |
| --- | --- | --- |
| IS-1 | PASS | qa/is1-overview.txt (local + omarchy groups, states, snippets, 13-char targets, rc=0) |
| IS-2 | PASS | qa/is2-read-other-workspace.txt (~/code/project session, kind ok), qa/is2-read-remote.txt (omarchy, kind ok) |
| IS-3 | PASS | qa/is3-send-local-multiline.txt (daemon, kind ok), qa/is3-send-remote-multiline.txt + qa/is3-read-remote-after.txt (omarchy TUI replied) |
| IS-4 | PASS | qa-is4/ (local TUI: streaming true -> interrupt rc=0 -> streaming false, "Operation aborted") + qa-remote-int/ (omarchy TUI: interrupt rc=0, "Operation aborted", streaming false) |
| IS-5 | PASS | qa-tui/ (pending request_id, overview "↳ Pick one [Alpha] [Beta]", answer rc=0, transcript "[Answer to question …] QA: Alpha" + agent ack) |
| IS-6 | PASS | qa/is6-watch-tick-1.txt (real question + blocked notice), qa/is6-watch-tick-2.txt (dedupe, none) |
| IS-7 | PASS | qa/is7-ambiguous-prefix.txt (rc=2 "matches several sessions") |
| IS-8 | PASS | gate: discord::session_commands tests (owner_allowed true/false/0, ephemeral) |
| IS-9 | PASS (live activation pending owner) | is9-activation.txt (scratch apply-block rc=0, new section 1|1|1, live row sha unchanged 814f73c7) + daemon_command PATH test |
| IS-10 | PASS | qa/is10-adopt.txt, is10-db-row.txt (external, canonical target, cursor 9), is10-release.txt, is10-db-after.txt (closed, released) |
| IS-11 | PASS | qa/is11-unreachable.txt (⚠️ line, other machines shown, rc=0) |
| IS-12 | PASS | gate-final-3.log (after probe.py fix: fmt, test sessions::, release RC=0, bin 63e0d77d) + gate-final.log (fmt, clippy -D warnings, 7 test targets, release build RC=0) + gate-final-2.log |
QA-caught regressions fixed: daemon sessions addressed by host-local id (send not_found) -> durable id canonical; probe routing id for daemon commands (interrupt unknown_session).

# sessions-judge review

### [P1] A failed host listing is reported as "not running", so one ssh blip kills an adopted lane
- Location: /Users/indo/code/project/omon-gateway/src/sessions/control.rs:162-175 (consumed at /Users/indo/code/project/omon-gateway/src/sessions/external.rs:308-310 and /Users/indo/code/project/omon-gateway/src/lanes/runner.rs:1051-1056)
- Trigger: A remote external lane has an unconsumed `lane_inbox` message, for example the `[LEAD] not done...` text or the claim guidance. `send_pending` calls `SessionControl::send`, which calls `session()`. `session()` builds a fresh registry and calls `list_all()`. If that listing fails (ssh ConnectTimeout, the 20 s timeout, `omo thread list` exit 3, or a Wi-Fi/VPN blip), the failure lands in `list.errors` and `list.sessions` is empty. The `find(...).ok_or(ControlError::NotRunning)` then returns `NotRunning`. `drive` special-cases that message and returns `Err` immediately, without the 3-error tolerance. The runner then calls `fail(..., "dead")`, and the lane goes to `failed` while the external session is still alive. `send_goal` has the same path.
- Impact: A single transient connectivity error permanently fails a healthy adopted lane. Only `working`, `not_done` and `blocked` lanes are driven or revived, so nothing restarts the driver.
- Fix: In `session()`, check `list.errors` for `target.machine` before returning `NotRunning`. Return a non-`NotRunning` error (for example `ControlError::Registry` or `Command`) when the machine's listing failed. Return `NotRunning` only when the listing succeeded and the session is absent.

### [P1] Remote Done-signal checks are capped at 20 s, so remote lanes with any long command signal can never verify
- Location: /Users/indo/code/project/omon-gateway/src/sessions/control.rs:200 (used by /Users/indo/code/project/omon-gateway/src/sessions/external.rs:417-424)
- Trigger: Adopt a remote session with `--done 'command ["cargo","test"]'`, or any command that runs longer than 20 s. `remote_checks_with` runs it through `control.output`, which wraps the call in `timeout(20s)`. The result is `reasons.push("remote command unavailable: session command timed out")`. The local runner allows 120 s for the same signal type (`SystemSignalRunner` `command_timeout`, signals.rs:879).
- Impact: Each settle tick (every 30 s) rejects the claim and pushes a `[LEAD] not done: ...` message into the external session. The lane loops claim, reject, nudge indefinitely even when the work is complete. The timeout drops the local ssh client, but the remote python process may keep running.
- Fix: Give `output()` a per-call timeout. Use the signal's budget (about 120 s) for remote checks and keep 20 s for list/send/read.

### [P1] `adopt` accepts a multi-line goal for a remote target, and the goal can then never be sent
- Location: /Users/indo/code/project/omon-gateway/src/sessions/external.rs:67 (adopt validation) and :158-177 (`send_goal`); control.rs:239
- Trigger: `omo-gateway sessions adopt host:sess --goal $'line1\nline2' ...`, with a goal from a heredoc or multi-line brief. `adopt` only checks that the goal is non-empty. `send_goal` flattens the footer with `replace(['\n','\r'], " | ")` but interpolates `{goal}` raw. `SessionControl::send` then returns `RemoteMultiline` for every remote send.
- Impact: The lane is created in `working` with `sent_at` NULL. Every step fails (`read_new` also returns early because `sent_at` is unset), and after 3 notifies or fallback ticks `drive` returns an error and the lane is marked `failed`. The operator got a success JSON from `adopt`.
- Fix: At `adopt`, reject or normalize (replace newlines with " | ") the goal and `done` text when `target.machine() != "local"`. Alternatively, apply the same flatten in `send_goal` to the whole composed text.

### [P2] `drive` fails a lane after 3 consecutive step errors, and notifies arrive on any lane activity
- Location: /Users/indo/code/project/omon-gateway/src/sessions/external.rs:300-312
- Trigger: The wake socket (runner.rs:1046-1048) notifies every active lane on any hint. During a remote outage lasting a minute or two, 3 quick notifies produce 3 consecutive 20 s timeouts or ssh errors.
- Impact: The lane is failed even though the "3 errors" were not spread over a meaningful time window.
- Fix: Count errors over time (for example a backoff or a minimum interval between retries), not per wake.

### [P2] `assistant_items` poisons the read cursor on one non-text assistant entry
- Location: /Users/indo/code/project/omon-gateway/src/sessions/external.rs:490-516
- Trigger: One assistant entry with `text` null or missing, or a content-block array (for example a tool-call-only message). The `?` aborts the whole read, so `cursor` (line 282) never advances, and the next poll hits the same page again.
- Impact: The lane's output capture is permanently wedged and the lane ends up `failed` after 3 errors. I could not confirm the omo read schema, so this is graded P2.
- Fix: Skip assistant entries without string text. Still advance the cursor.

### [P2] `Instant::now() - age` can panic on a freshly booted host
- Location: /Users/indo/code/project/omon-gateway/src/sessions/registry.rs:287
- Trigger: A disk-cached listing aged up to 300 s while the process `Instant` clock (time since boot) is less than that age, for example `sessions list` or the status reply right after reboot.
- Impact: Panic ("overflow when subtracting duration from instant") in the status reply or CLI.
- Fix: Use `Instant::now().checked_sub(age)` and skip the cache entry on `None`.

### [P2] `ssh` is invoked without `--` for registry hosts
- Location: /Users/indo/code/project/omon-gateway/src/sessions/registry.rs:214-221, with validation in `configured_hosts` and `read_seeded_hosts`
- Trigger: A host name such as `-oProxyCommand=...` in `OMON_SESSION_HOSTS` or `~/.omon/session-hosts.json`. Validation only rejects empty names and names containing whitespace. `Target::from_str` rejects a leading `-`, but the registry path does not.
- Impact: ssh option injection from the owner-controlled config.
- Fix: Reject hosts that start with `-`, and pass `--` before the machine in `ProcessCommandRunner`.

### [P2] The JevCli daily caps are per instance and are not restored on restart
- Location: /Users/indo/code/project/omon-gateway/src/judge/mod.rs:736-754, with construction sites at runner.rs:746, settle.rs:28, kibitzer.rs:158 and main.rs
- Trigger: Only `lanes/cli.rs` calls `.init()`. Every other `JevCli::new` (runner, settler, kibitzer, remediator, triage) starts with its own counters at 0.
- Impact: The 300/day and 50/day caps are multiplied by the number of instances, and they reset on each gateway restart or crash loop.
- Fix: Share one `JevCli` (or the counters) across consumers, and call `init()` at construction.

### [P2] Non-trigger failures reset the circuit-breaker counter
- Location: /Users/indo/code/project/omon-gateway/src/judge/mod.rs:832-846 and :1103-1106
- Trigger: A persistent failure without "401"/"429" in stderr (HTTP 500, DNS failure, io error) calls `record_failure(false)`, which sets `consecutive_failures = 0`. Interleaving such failures with a 429 prevents the breaker from ever reaching 5. Matching on the substrings "401"/"429" can also false-trigger (for example on a request id containing 401).
- Impact: The breaker does not open when it should, so every call pays the jev 5 s timeout before falling back.
- Fix: Do not reset on non-trigger failures (only on success), and match on structured status instead of substrings.

### [P2] The scrubbed state temp file is created with default permissions and `set_var` runs from async worker threads
- Location: /Users/indo/code/project/omon-gateway/src/judge/mod.rs:1004-1010 (temp file) and :907-928 (`load_typesafe_env`)
- Trigger: On Linux `/tmp` is shared, so `File::create` under the default umask makes the `jev_state_*.txt` file world-readable. `std::env::set_var` is called from concurrent judge calls. The `TYPESAFE_*` values loaded from `.zshenv` also become inherited by every later child process.
- Impact: Local information disclosure of scrubbed state, a data-race hazard on the environment, and the API key leaking into spawned subprocesses.
- Fix: Create the temp file with mode 0o600 (`OpenOptions` plus `mode`). Load the env once at startup, or pass the values with `cmd.env(...)` instead of mutating the process environment.

### [P2] `external.sql` duplicates `migrations/0040_external_sessions.sql`
- Location: /Users/indo/code/project/omon-gateway/src/sessions/external.rs:21
- Trigger: `MIGRATION` is not referenced anywhere else in `src/`. The real migration lives in `migrations/`, and the two copies can drift.
- Impact: Dead copy that can mislead.
- Fix: Remove the const and the duplicate SQL file, or have one include the other.

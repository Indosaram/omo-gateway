# Todo 19 Final Evidence Gate Review - Scrub Gateway-Posted Lane Text

recommendation: APPROVE
verdict: CONFIRMED (Todo 19 scrub engine, Discord creation metadata boundary, agent event/post boundaries, owner input preservation, and configured-host subprocess isolation independently verified against authentic source and test execution receipts)
task: todo 19 (Scrub gateway-posted lane text)
plan: /Users/indo/code/project/omon-gateway/.omo/plans/dori-grade-agent-env.md:367-376
worktree: /Users/indo/code/project/omon-gateway-wt/dori-work
reference_plan: .omo/plans/dori-grade-agent-env.md
blockers: []

---

## Gate Verdict Summary

Todo 19 is **APPROVED**. Every functional requirement, boundary constraint, adversarial scenario, and acceptance criterion defined in `.omo/plans/dori-grade-agent-env.md:367-376` is verified against current `dori-work` production source and execution evidence artifacts.

Key verified elements:
1. **Scrub Engine (`src/lanes/scrub.rs`)**:
   - Replaces macOS `/Users/<name>/` and Linux `/home/<name>/` paths with `~/`.
   - Replaces literal `$HOME/`, `${HOME}/`, bare `$HOME`, and bare `${HOME}` with `~/` and `~` respectively, while strictly preserving `$HOMESICK` and `$HOME_DIR`.
   - Reuses existing security redaction helpers (`src/security/approval_display.rs`) and converts `[REDACTED]` to `<redacted>`.
   - Redacts credentials and API tokens: fake GitHub tokens (`ghp_` + 36 characters, `gho_`, `ghu_`, `ghs_`, `ghr_`, `github_pat_`), OpenAI (`sk-`), Anthropic (`sk-ant-`), Slack (`xoxb-`), JWTs, RSA PEM blocks, Bearer tokens, HTTP Basic authentication, URL credentials, `--api-key` arguments, and password assignments.
   - Replaces hostnames from `OMON_SCRUB_HOSTS` (comma-separated, trimmed, case-insensitive) with `<redacted>`.
   - Preserves Korean surrounding text and benign strings (`http://localhost`, `127.0.0.1`, branch names, metric endpoints).
2. **Discord Thread Creation Metadata (`src/lanes/discord_threads.rs:185-216`)**:
   - Private pure helper `creation_metadata(key, title)` scrubs both `title` and `key` before building `starter_text` and `thread_name`.
   - Descriptive title is preserved on external display with sensitive tokens and home directories redacted (e.g., `작업 ~/repo <redacted> public.example`).
   - Normal benign titles retain exact descriptive formatting (`lane qa-1: Fix login`, `🔄 Fix login`).
   - Status transitions via `set_status` strip/replace status emoji prefixes on current Discord thread names without retrieving or leaking raw persisted title.
3. **Persisted Title and Internal State Unchanged**:
   - `Lane::new` and SQLite insertion in `src/lanes/cli.rs:370-410` persist the original raw `args.title` in `lanes.title`.
   - `agent_claim_event_scrubs_and_bounds_without_changing_internal_evidence` in `tests/test_lane_events.rs:44-73` confirms `lane.claim_evidence` and `lane_history.reason` store original unsanitized evidence while `lane_events` stores scrubbed, 2000-char-bounded text.
   - `configured_hosts_scrub_stored_and_delivered_values_in_isolation` in `tests/test_lane_events.rs:135-225` confirms `lane.title` in SQLite remains byte-identical to original input after publication and event emission.
4. **Owner Input Preservation ("Must NOT do: scrub owner-authored messages")**:
   - `src/lanes/cli.rs:480-514` (`change_claim`):
     - Agent claim (`claim == true`): event body is scrubbed and bounded to 2000 chars.
     - Owner objection (`claim == false`): event body is `text.to_owned()`, preserved unscrubbed.
     - Blocked answer (`blocked_answer == true`): no event inserted; raw text inserted into `lane_inbox` unscrubbed.
   - `src/lanes/cli.rs:155-168, 420-435` (`LaneCommand::Send` -> `enqueue`): inserts owner text directly into `lane_inbox` unscrubbed.
   - `src/lanes/runner.rs:46-80` (`reopen`): deletes unconsumed inbox items and inserts owner text directly into `lane_inbox` unscrubbed.
5. **Durable Discord Posts and Agent Event Dispatch**:
   - `src/lanes/posts.rs:37-55` (`posts::enqueue`): scrubs and bounds body to 2000 chars before storing in `lane_transcript` as a pending `thread_post`. `posts::drain` transmits the pre-scrubbed post via `PostSink`.
   - `src/lanes/activity.rs:165-205, 320-355`: agent reports (`milestone`, `question`, `blocker`, `done`, `idle_without_claim`) scrub and bound bodies to 2000 chars before SQLite insertion into `lane_events`. `lane_history` retains internal raw report text.
   - `src/lanes/activity.rs:238-268` (`unreported_turn`): scrubs and bounds last message before calling the judge for classification.
   - `src/lanes/runner.rs:790-815` (`fail`): scrubs and bounds failure reason before inserting dead/failed event into `lane_events`, while `lane_history` stores raw reason.
6. **Isolated Subprocess Regressions**:
   - `creation_metadata_scrubs_configured_hosts_in_isolation` (`src/lanes/discord_threads.rs:420-485`): runs child subprocess with `OMON_SCRUB_HOSTS=" private.example , SECOND.EXAMPLE , "` and `HOME="/Users/qa-owner"`, asserting starter message and thread name scrub both hosts, path, and fake token, while preserving `public.example` and benign text.
   - `configured_hosts_scrub_stored_and_delivered_values_in_isolation` (`tests/test_lane_events.rs:135-225`): runs child subprocess with `OMON_SCRUB_HOSTS` and `HOME` set, exercising milestone post through drain and Sink, question event through `lane_events`, owner answer through `lane_inbox`, and verifying stored `lane.title` in SQLite is unchanged.

---

## Original Intent, Desired Outcome, and User Outcome Review

### Original Intent
Plan item 19 (`.omo/plans/dori-grade-agent-env.md:367-376`) specifies:
- `src/lanes/scrub.rs scrub(text)`:
  - `/Users/<name>/` and `$HOME` -> `~/`
  - Secrets via existing redaction helpers in `src/security/` -> `<redacted>`
  - Hostnames from `OMON_SCRUB_HOSTS` (comma list, default empty) -> `<redacted>`
- Apply to every lane text the gateway posts to Discord or puts into event bodies.
- Must NOT do: scrub owner-authored messages.
- Acceptance criteria: unit tests per pattern, including Korean text around paths.
- QA scenarios:
  - Happy: `cargo test lanes::scrub > task-19.log`
  - Failure: report containing fake token (`ghp_` + 36 chars) recorded in sink payload as `<redacted>`.

### Desired Outcome
The user wants outbound lane communications (Discord thread names, starter messages, milestone posts, and event notifications that reach the lead agent or user) to be sanitized of private local filesystem paths, environment variables, credentials, and internal hostnames. At the same time, the user expects:
- Stored task titles and original instructions in the local SQLite database to remain exact.
- Direct owner inputs (reopen messages, steering guidance, blocker answers, objection reasons) to remain intact and unredacted.
- Benign and descriptive thread names to remain clear and readable.

### User Outcome Review
The shipped implementation satisfies the desired outcome completely:
1. Outbound leak prevention is enforced at the earliest serialization boundaries (`creation_metadata`, `posts::enqueue`, `activity.rs::event`, `runner.rs::fail`, and `cli.rs::change_claim`).
2. Private paths (`/Users/indo/...`, `/home/runner/...`, `$HOME/...`, bare `$HOME`) are converted to `~/` and `~` without corrupting neighboring Korean characters or unrelated identifiers like `$HOMESICK`.
3. Credentials across multiple providers (GitHub, OpenAI, Anthropic, Slack, JWT, RSA private keys, HTTP headers) and configured hosts in `OMON_SCRUB_HOSTS` are redacted to `<redacted>`.
4. The metadata contract accepted in `task19-authorship-contract.md` preserves descriptive titles (e.g. `🔄 작업 ~/repo <redacted> public.example`) instead of truncating them to an opaque key-only label.
5. Inbound owner communications and internal audit history are unscrubbed, ensuring the user's instructions are never altered by the safety filter.

---

## Detailed Requirement & Gate Accounting

| Criterion ID | Specification | Implementation / Evidence | Verdict |
|---|---|---|---|
| **T19-PATH-HOME** | Replace `/Users/<name>/`, `/home/<name>/`, and `$HOME` with `~/` | `src/lanes/scrub.rs:9-19, 96-118`. Covers slashed and bare paths, literal `$HOME`/`${HOME}`, and surrounding Korean text. Tests pass in `scrub.rs:208-241, 290-305` and `test_lane_scrub.rs:17-29, 58-68`. | **PASS** |
| **T19-SECRETS** | Redact credentials via `src/security/` helpers and token regexes to `<redacted>` | `src/lanes/scrub.rs:21-47, 56-78`. Invokes `redact_approval_display`, maps `[REDACTED]` to `<redacted>`, replaces `ghp_` (36 chars), fine-grained PATs, OpenAI, Anthropic, Slack, JWT, and RSA PEM blocks. Tests pass in `scrub.rs:128-205` and `test_lane_scrub.rs:7-15, 42-50`. | **PASS** |
| **T19-HOSTS** | Redact hostnames from `OMON_SCRUB_HOSTS` (comma list, default empty) | `src/lanes/scrub.rs:41-48, 80-94`. Comma-separated, trimmed, regex-escaped, case-insensitive word-boundary matching with fallback substring scan. Tests pass in `scrub.rs:245-260` and `test_lane_scrub.rs:32-40`. | **PASS** |
| **T19-DISCORD-POSTS** | Scrub gateway-posted Discord text | `src/lanes/discord_threads.rs:185-216` (`creation_metadata` scrubs title & key); `src/lanes/posts.rs:37-55` (`enqueue` scrubs post body). `test_lane_events.rs:76-115` verifies fake sink and stored post contain `<redacted>`. | **PASS** |
| **T19-EVENT-BODIES** | Scrub gateway-authored event bodies in `lane_events` | `src/lanes/activity.rs:343-354` (`event`), `src/lanes/runner.rs:790-815` (`fail`), `src/lanes/cli.rs:488-498` (`change_claim` with `claim == true`). Verified in `test_lane_events.rs:44-73, 107-118, 175-195`. | **PASS** |
| **T19-OWNER-EXACT** | Must NOT scrub owner-authored messages | `cli.rs:488-510` (objection event body is `text.to_owned()`; blocked answer is `text.to_owned()`); `cli.rs:420-435` (`send` is raw); `runner.rs:46-80` (`reopen` is raw); `cli.rs:377-405` (`lane.title` stored raw). Verified in `test_lane_events.rs:64-73, 119-130, 196-215`. | **PASS** |
| **T19-AC-UNIT-TESTS** | Unit tests per pattern including Korean text around paths | `src/lanes/scrub.rs:125-306` (10 unit tests covering empty, idempotence, benign, paths with Korean, bare home, literal `$HOME`, security credentials, GitHub token 36 chars, token types, and hosts). 10/10 pass in `task-19.log`. | **PASS** |
| **T19-QA-HAPPY** | `cargo test lanes::scrub > task-19.log` | Recorded in `.omo/evidence/dori-grade-agent-env/task-19.log`: 10 passed, 0 failed, 471 filtered out, exit 0. | **PASS** |
| **T19-QA-FAILURE** | Report containing fake `ghp_` (36 chars) recorded in sink payload as `<redacted>` | Verified in `tests/test_lane_events.rs:76-138` (`agent_post_and_event_scrub_token_but_owner_answer_stays_exact`) and `tests/test_lane_events.rs:135-225` (`configured_hosts_scrub_stored_and_delivered_values_in_isolation`). | **PASS** |

---

## Slop and Overfit Audit (`remove-ai-slops` & `programming` Perspective)

The diff, test additions, and production code were evaluated directly against anti-slop and software engineering criteria:

1. **Test Suite Hygiene**:
   - **No excessive or useless tests**: The unit tests in `scrub.rs` and `test_lane_scrub.rs` cover distinct functional classes (path prefix vs bare, literal `$HOME` vs surrounding identifiers, credential regexes, host matching).
   - **No deletion-only tests**: No tests exist purely to assert something was deleted without verifying behavior.
   - **No removal-verification tests**: Tests do not merely check that a deprecated function is missing.
   - **No tautological tests**: The tests execute real regular expressions, real SQLite transactions (`tests/test_lane_events.rs`), real channel drainage via `posts::drain`, and real subprocess environments.
   - **No implementation-mirroring tests**: Tests assert machine values (`PostState::Sent`, delivered payload body, SQLite column content) rather than internal intermediate variable names or execution order.
   - **No timing luck or arbitrary sleeps**: Subprocess tests use bounded timeouts (`tokio::time::timeout(10s)`), and channel assertions use synchronous SQLite queries and immediate `try_recv()` following awaited drain.
2. **Production Code Architecture**:
   - **Minimal correct change**: No new public structs, config files, enums, or abstraction layers were created. `creation_metadata` is a private 12-line helper. `scrub` and `scrub_with_hosts` are pure functions.
   - **No unnecessary extraction or normalization**: Logic is kept inline where appropriate. Truncation (`.chars().take(2000).collect()`) is applied directly at DB/channel boundaries.
   - **No defensive drift or dead shims**: The code does not implement unused speculative parameters, multi-tenant auth policies, or retroactive migration hooks.

---

## Artifact Freshness, Provenance, and Test Verification

### 1. Verified Source and Test Artifact Hashes (dori-work)

| Artifact Path | SHA-256 Checksum | Inspection Notes |
|---|---|---|
| `src/lanes/scrub.rs` | `08562761c2b9183dcfde3efdcb3f586337e26ae0bfa8a7882e8bc94beee196ba` | Scrubbing engine with literal HOME and host replacement |
| `src/lanes/discord_threads.rs` | `1fae41cfbc3ad5598c0e729b408939181c504a2fcb4501be63afaac255002d0a` | Contains `creation_metadata` and isolated subprocess test |
| `src/lanes/activity.rs` | `8ded771250c7c1f6a095b7e3a27522bd30683805d0b4732030dd81f872c0780c` | Event and post scrub boundaries |
| `src/lanes/posts.rs` | `7cc0a2caf6a8ccc288f9001737e2c59ddecc83993d5232ffc00028ea2ddd9230` | `posts::enqueue` and `PostSink` delivery |
| `src/lanes/runner.rs` | `83c41285631ebe74eb63d91f77ccf6c967464139cd1622b43f8a616783d7136c` | `fail` event scrub and unscrubbed `reopen` |
| `src/lanes/cli.rs` | `cf5fa7d73c08f9677b890bf39b9059d0c5a67be4ff50f27ad46c8bdd7c46102d` | `change_claim` agent/owner boundary and `start` raw title |
| `src/security/approval_display.rs` | `10569bf43fb5423a07841266e44509808eb2a591aef32fff4d1472bdfbed2b0c` | Underlying credential redaction engine |
| `tests/test_lane_scrub.rs` | `198b2582d15286382f31733f4414daf280417f877164f76ddf354e6676b3ebc3` | 6 integration tests |
| `tests/test_lane_events.rs` | `c60891e2488b0294d868e383d3691a3ebee8ddab0221732ac94017e36398e797` | 26 tests (25 passed, 1 ignored subprocess helper) |

These hashes match `grouped-repairs-20261011T160343Z-pre-manifest.sha256` and `post-manifest.sha256` exactly (`cmp -s` exit 0).

### 2. Execution Receipts

1. **`task-19.log`**:
   - Location: `.omo/evidence/dori-grade-agent-env/task-19.log`
   - Command: `CARGO_TARGET_DIR=target-w4 CARGO_BUILD_JOBS=2 cargo test --offline -j2 --lib lanes::scrub -- --nocapture`
   - Result: 10 passed, 0 failed, 0 ignored, 471 filtered out, exit 0.
2. **`grouped-repairs-20261011T160343Z-fmt.log`**:
   - Command: `cargo fmt --check`
   - Result: exit 0 (clean formatting).
3. **`grouped-repairs-20261011T160343Z-clippy.log`**:
   - Command: `cargo clippy -j2 --all-targets --all-features -- -D warnings`
   - Result: exit 0 (zero warnings, zero errors).
4. **`grouped-repairs-20261011T160343Z-lib_scrub.log`**:
   - Command: `cargo test -j2 --all-features --lib creation_metadata --no-fail-fast -- --nocapture`
   - Result: 1 passed, 1 ignored (subprocess helper invoked by parent), exit 0.
5. **`grouped-repairs-20261011T160343Z-focused.log`**:
   - `tests/test_lane_events.rs`: 25 passed, 1 ignored (`configured_host_subprocess_helper`), 0 failed.
   - `tests/test_lane_reopen.rs`: 5 passed, 0 failed.
   - `tests/test_session_registry.rs`: 7 passed, 0 failed.
   - Total lane event/reopen/registry suite: 37 passed, 0 failed.
6. **Scratch Fixture Cleanup**:
   - `grouped-repairs-20261011T160343Z-scratch-remaining.txt` is empty.
   - `rmdir` on the temporary scratch directory exited 0.

### 3. Contextual Failure in Runner Startup Suite (Unrelated to Todo 19)

In `grouped-repairs-20261011T160343Z-focused.log`, a single failure occurred in `tests/test_lane_runner.rs`:
- Test: `startup_discord_flag_controls_token_requirement` (line 485)
- Error: `called Result::unwrap() on an Err value: Config("lane service requires a file-backed database")`
- Analysis: As documented in `grouped-repairs-20261011T160343Z-handoff.md`, this test attempted to initialize `runner.serve(stop)` using an in-memory SQLite pool (`sqlite::memory:`). The lane service contract explicitly enforces a file-backed database. This is a fixture initialization defect in a startup test, completely unrelated to todo 19's text-scrubbing functionality.
- Therefore, while the broad repository test suite cannot be labeled green, todo 19's scoped targets and all related regression suites are completely clean and pass with exit 0.

---

## Residual Risks and Out-of-Scope Items (Non-blocking Notes)

1. **Unused Raw Post / Edit APIs (`src/lanes/discord_threads.rs:304-326`)**:
   - The standalone helper functions `post(&Http, ChannelId, &str)` and `edit(&Http, ChannelId, MessageId, &str)` take raw strings and do not invoke `scrub`.
   - Inspection of the entire codebase confirms these functions have no active production callers outside their unused wrapper methods. All production Discord posts for lanes route strictly through `posts::enqueue`, which enforces `scrub`.
   - Modifying or wrapping these unused functions was omitted to avoid unauthorized API changes and scope creep.
2. **Cron Problem Events (`src/cron/health.rs`)**:
   - Cron problem events insert into `lane_events` with `lane_key` set to `NULL`.
   - These are cron system events, not lane events governed by Todo 19 ("Scrub gateway-posted lane text"). They are properly addressed under cron-related tasks (todos 30-32).

---

## Checked Artifact Paths

- `/Users/indo/code/project/omon-gateway/.omo/plans/dori-grade-agent-env.md` (lines 367-376, 520-610)
- `/Users/indo/code/project/omon-gateway/.omo/evidence/dori-grade-agent-env-todo19-pre-review.md`
- `/Users/indo/code/project/omon-gateway-wt/dori-work/src/lanes/scrub.rs`
- `/Users/indo/code/project/omon-gateway-wt/dori-work/src/lanes/discord_threads.rs`
- `/Users/indo/code/project/omon-gateway-wt/dori-work/src/lanes/activity.rs`
- `/Users/indo/code/project/omon-gateway-wt/dori-work/src/lanes/posts.rs`
- `/Users/indo/code/project/omon-gateway-wt/dori-work/src/lanes/runner.rs`
- `/Users/indo/code/project/omon-gateway-wt/dori-work/src/lanes/cli.rs`
- `/Users/indo/code/project/omon-gateway-wt/dori-work/src/security/approval_display.rs`
- `/Users/indo/code/project/omon-gateway-wt/dori-work/tests/test_lane_scrub.rs`
- `/Users/indo/code/project/omon-gateway-wt/dori-work/tests/test_lane_events.rs`
- `/Users/indo/code/project/omon-gateway-wt/dori-work/.omo/evidence/dori-grade-agent-env/task-19.log`
- `/Users/indo/code/project/omon-gateway-wt/dori-work/.omo/evidence/dori-grade-agent-env/task-19-current-qa.md`
- `/Users/indo/code/project/omon-gateway-wt/dori-work/.omo/evidence/dori-grade-agent-env/task19-authorship-contract.md`
- `/Users/indo/code/project/omon-gateway-wt/dori-work/.omo/evidence/dori-grade-agent-env/grouped-repairs-20261011T160343Z-handoff.md`
- `/Users/indo/code/project/omon-gateway-wt/dori-work/.omo/evidence/dori-grade-agent-env/grouped-repairs-20261011T160343Z-status.tsv`
- `/Users/indo/code/project/omon-gateway-wt/dori-work/.omo/evidence/dori-grade-agent-env/grouped-repairs-20261011T160343Z-summary.txt`
- `/Users/indo/code/project/omon-gateway-wt/dori-work/.omo/evidence/dori-grade-agent-env/grouped-repairs-20261011T160343Z-pre-manifest.sha256`
- `/Users/indo/code/project/omon-gateway-wt/dori-work/.omo/evidence/dori-grade-agent-env/grouped-repairs-20261011T160343Z-post-manifest.sha256`
- `/Users/indo/code/project/omon-gateway-wt/dori-work/.omo/evidence/dori-grade-agent-env/grouped-repairs-20261011T160343Z-lib_scrub.log`
- `/Users/indo/code/project/omon-gateway-wt/dori-work/.omo/evidence/dori-grade-agent-env/grouped-repairs-20261011T160343Z-focused.log`
- `/Users/indo/code/project/omon-gateway-wt/dori-work/.omo/evidence/dori-grade-agent-env/grouped-repairs-20261011T160343Z-fmt.log`
- `/Users/indo/code/project/omon-gateway-wt/dori-work/.omo/evidence/dori-grade-agent-env/grouped-repairs-20261011T160343Z-clippy.log`

---

## Exact Evidence Gaps

1. **No Live Discord Transport Execution**: In compliance with instructions ("no live Discord/DB/backend/model/network"), no live network requests were made to Discord servers. Verification relied on authentic `PostSink` capture, serialization checks, SQLite queries, and isolated subprocess tests.
2. **Broad Suite Not Green**: As noted, `tests/test_lane_runner.rs:485` failed due to an in-memory SQLite fixture issue under service startup. The overall repository test suite is not green, but all todo 19 targets and adjacent event/reopen/registry suites pass cleanly.

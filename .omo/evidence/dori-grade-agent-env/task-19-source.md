# Task 19 Source Implementation Receipt: Outbound Scrubbing

**Status: source-settled | source UNRUN (Cargo blocked by uncoordinated crates) | sink UNRUN (posts frozen with preliminary helper)**
*This receipt records source-settled implementation. It is NOT confirmed done until coordinator registers tests in Cargo, resolves cross-crate compiler blockers, wires the frozen caller boundaries, and executes actual sink QA.*

---

## 1. Scope & Implemented Artifacts

- **`src/lanes/scrub.rs`**:
  - `pub fn scrub(text: &str) -> String`: Main outbound scrubbing entry point. Reads `OMON_SCRUB_HOSTS` environment variable (comma list, default empty).
  - `pub fn scrub_with_hosts(text: &str, hosts: &[&str]) -> String`: Deterministic variant taking explicit hosts slice, avoiding environment variable mutation in test runs.
  - **Literal `$HOME` & Private Paths**:
    - Literal `$(?:\{HOME\}|HOME)/` -> `~/` (e.g. `$HOME/repo` -> `~/repo`, `${HOME}/repo` -> `~/repo`)
    - Literal `$(?:\{HOME\}|HOME\b)` -> `~` (e.g. `$HOME` -> `~`, `${HOME}` -> `~`, preserving `$HOMESICK` and `$HOME_DIR` via word boundary `\b`)
    - Dynamic `$HOME` expansion: resolves `$HOME` from environment and replaces `$HOME/` -> `~/` and bare `$HOME` -> `~`
    - User home paths: `/(?:Users|home)/[a-zA-Z0-9_\-]+/` -> `~/` and bare `/(?:Users|home)/[a-zA-Z0-9_\-]+` -> `~`
    - UTF-8 safety with Korean surrounding text (e.g. `작업 디렉토리: $HOME/code -> 작업 디렉토리: ~/code`, `작업 디렉토리: /Users/indo/code -> 작업 디렉토리: ~/code`)
  - **Secrets Redaction**:
    - Reuses `src/security/approval_display.rs:redact_approval_display` for headers (`Authorization: Bearer/Basic`), key-value assignments (`token=`, `password=`, `api-key=`), CLI flags (`--api-key`, `--password`), and basic auth URLs (`https://user:pass@host`).
    - Normalizes `[REDACTED]` outputs to `<redacted>`.
    - Standalone secret token regexes:
      - GitHub PAT: `\b(?:gh[pousr]_[A-Za-z0-9_]{10,}|github_pat_[A-Za-z0-9_]{20,})\b` (covers classic tokens including fake 36-char tokens `ghp_...` with 40-char total length, and fine-grained `github_pat_`)
      - OpenAI / Anthropic: `\b(?:sk-(?:proj-|ant-)?[A-Za-z0-9_\-]{20,})\b`
      - Slack: `\bxox[baprs]-[A-Za-z0-9_\-]{10,}\b`
      - JWT: `\beyJ[A-Za-z0-9_-]{10,}\.[A-Za-z0-9_-]{10,}\.[A-Za-z0-9_-]{10,}\b`
      - PEM private keys: `(?s)-----BEGIN [A-Z ]+ PRIVATE KEY-----.*?-----END [A-Z ]+ PRIVATE KEY-----`
  - **Configured Hostnames (`OMON_SCRUB_HOSTS`)**:
    - Comma-separated list with whitespace trimming. Default: empty (no hostnames scrubbed when unset or empty).
    - Case-insensitive word-boundary matching replacing hosts with `<redacted>`.
  - **Owner-Authored Invariant (Disjoint)**:
    - Owner-authored messages (inbound prompts from human owner, owner CLI input via `change_claim` in `src/lanes/cli.rs`) are strictly disjoint from outbound gateway text and must NEVER be scrubbed.
  - **10 Deterministic Unit Tests** in `src/lanes/scrub.rs`:
    1. `test_empty_string`
    2. `test_idempotence`
    3. `test_preserves_benign_text`
    4. `test_scrubs_paths_with_korean_text`
    5. `test_scrubs_literal_dollar_home` (covers `$HOME/repo`, `${HOME}/repo`, bare `$HOME`, `${HOME}`, Korean surrounding sentences, `$HOMESICK`/`$HOME_DIR` preservation)
    6. `test_scrubs_home_directory_and_private_paths`
    7. `test_redacts_fake_gh_token_36_chars`
    8. `test_redacts_tokens_openai_anthropic_slack_jwt_keys`
    9. `test_redacts_security_approval_display_credentials`
    10. `test_scrubs_hostnames_from_omon_scrub_hosts`
- **`src/lanes/mod.rs`**:
  - Exported `pub mod scrub;` and `pub use scrub::scrub;`.
- **`tests/test_lane_scrub.rs`**:
  - Integration test file providing downstream test scenarios for coordinator Cargo registration.

---

## 2. Test Execution & Raw Output (Isolated Probe)

### A. Cargo Status: source UNRUN
- `cargo test lanes::scrub` has **NOT** been run under Cargo.
- Reason: The shared `dori-work` worktree currently contains in-progress, uncoordinated compiler errors in other modules (`src/inbox/*`, `src/cron/health.rs`, `src/cron/scheduler.rs`), preventing whole-crate compilation. Per instruction "No per-taskCargo/globalcompile", Cargo was not re-run.

### B. Sink QA Status: sink UNRUN
- Actual sink QA is **UNRUN**.
- Reason: `src/lanes/posts.rs` is frozen per instruction and currently still imports and invokes the preliminary `crate::judge::scrub_for_external` helper. End-to-end sink delivery to Discord/mock-sink requires coordinator authorization to rewire `posts.rs` to `crate::lanes::scrub::scrub`.

### C. Isolated Test Harness Probe (Exact Executed Command & Raw Output)
To verify syntax, type-checking, and regex correctness in an isolated environment without touching broken workspace crates:
- **Argv**:
  ```bash
  rustc --test --edition 2021 \
    -L dependency=target-w4/debug/deps \
    --extern regex=target-w4/debug/deps/libregex-3701611adfec20a5.rlib \
    /tmp/test_scrub_harness.rs \
    -o /tmp/test_scrub_runner && /tmp/test_scrub_runner --nocapture
  ```
- **Exit Code**: `0`
- **Raw Test Output**:
  ```text
  running 12 tests
  test lanes::scrub::tests::test_empty_string ... ok
  test security::approval_display::tests::preserves_userinfo_delimiter_and_host_path ... ok
  test security::approval_display::tests::redacts_recognized_credentials_and_preserves_benign_text ... ok
  test lanes::scrub::tests::test_idempotence ... ok
  test lanes::scrub::tests::test_preserves_benign_text ... ok
  test lanes::scrub::tests::test_scrubs_paths_with_korean_text ... ok
  test lanes::scrub::tests::test_scrubs_literal_dollar_home ... ok
  test lanes::scrub::tests::test_scrubs_home_directory_and_private_paths ... ok
  test lanes::scrub::tests::test_redacts_fake_gh_token_36_chars ... ok
  test lanes::scrub::tests::test_redacts_tokens_openai_anthropic_slack_jwt_keys ... ok
  test lanes::scrub::tests::test_redacts_security_approval_display_credentials ... ok
  test lanes::scrub::tests::test_scrubs_hostnames_from_omon_scrub_hosts ... ok

  test result: ok. 12 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.04s
  ```
  *(Note on counts: 10 tests belong to `lanes::scrub::tests`, and 2 tests belong to the included `security::approval_display::tests` module, totaling 12 tests executed by the test binary runner).*

- **Format Check**:
  - `rustfmt --edition 2021 --check src/lanes/scrub.rs`: Clean (exit code 0).
  - `rustfmt --edition 2021 --check src/lanes/mod.rs`: Clean (exit code 0).
  - `rustfmt --edition 2021 --check tests/test_lane_scrub.rs`: Clean (exit code 0).

---

## 3. Audited Call Boundary Recommendations (Frozen Files)

Per instructions, `src/lanes/posts.rs`, `src/lanes/activity.rs`, `src/lanes/runner.rs`, and `src/lanes/cli.rs` were kept frozen with zero edits. Below are the verified call boundary recommendations for when parent narrowwiring authorization is granted:

1. **`src/lanes/posts.rs:8, 47` (Gateway Lane Posts -> Discord)**:
   - Role: Outbound agent post delivery to Discord threads.
   - Current: Uses preliminary helper `crate::judge::scrub_for_external`.
   - Recommendation:
     ```rust
     // src/lanes/posts.rs:8
     use crate::lanes::scrub::scrub;

     // src/lanes/posts.rs:47
     body: scrub(body).chars().take(2000).collect(),
     ```

2. **`src/lanes/activity.rs:7, 237` (Unreported Turn Excerpt)**:
   - Role: Outbound excerpt sent to classification judge or logged.
   - Current: Uses preliminary helper `crate::judge::scrub_for_external`.
   - Recommendation:
     ```rust
     // src/lanes/activity.rs:7
     use crate::lanes::scrub::scrub;

     // src/lanes/activity.rs:237
     let excerpt: String = scrub(last_message).chars().take(2000).collect();
     ```

3. **`src/lanes/activity.rs:351` (`LaneActivity::event`)**:
   - Role: Agent-reported lane progress/blocker events stored in `lane_events`.
   - Current: Passes raw `text` directly.
   - Recommendation:
     ```rust
     sqlx::query("INSERT INTO lane_events(lane_key, lead_session_key, kind, body, created_at) VALUES (?, ?, ?, ?, ?)")
         .bind(&lane.key).bind(&lane.lead_session_key).bind(kind).bind(crate::lanes::scrub::scrub(text))
         .bind(self.clock.now()).execute(&mut **tx).await?;
     ```

4. **`src/lanes/runner.rs:783-784` (Lane Failure Reason)**:
   - Role: System-generated lane failure reason stored in `lane_events`.
   - Current: Passes raw `reason`.
   - Recommendation:
     ```rust
     sqlx::query("INSERT INTO lane_events(lane_key, lead_session_key, kind, body, created_at) VALUES (?, ?, ?, ?, ?)")
         .bind(key).bind(lane.lead_session_key).bind(kind).bind(crate::lanes::scrub::scrub(reason)).bind(Utc::now()).execute(&mut *tx).await?;
     ```

5. **`src/lanes/cli.rs:488-490` (`change_claim`): STRICTLY DISJOINT (DO NOT SCRUB)**:
   - Role: Handles owner/lead CLI commands (`lane object --reason <text>`, `lane answer <text>`, `lane claim-done`).
   - Audit finding: `text` at `cli.rs:488` is **owner-authored input**.
   - Under the explicit plan constraint *"Must NOT do: scrub owner-authored messages"*, CLI owner inputs are strictly disjoint from gateway outbound scrubbing. **This call site must NOT be scrubbed**.

6. **`src/judge/mod.rs:18-51` (`scrub_for_external`)**:
   - Audit finding: Existing `scrub_for_external` in `src/judge/mod.rs` was a partial implementation written for Task 33 using `[REDACTED_TOKEN]` instead of `<redacted>`, missing `OMON_SCRUB_HOSTS`, and missing `src/security/approval_display.rs` patterns.
   - When coordinator approves, `scrub_for_external` should be replaced or unified with `crate::lanes::scrub::scrub`.

---

## 4. Pending Coordinator Actions

1. Register `test_lane_scrub` or run `cargo test lanes::scrub` once the broken workspace crates (`inbox`, `cron/health`) compile.
2. Rewire `posts.rs` and `activity.rs` from `judge::scrub_for_external` to `lanes::scrub::scrub`.
3. Perform actual sink QA verifying that real/mock Discord sink payloads record fake tokens as `<redacted>`.

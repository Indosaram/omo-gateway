# Delta Code Review Report

## 1. Scope & Verification Summary

Reviewed `.omo/evidence/review-20261019/CONFIRMED-r1.md` and related reports (`r1/agent-daemon-deploy.md`, `r1/main-runtime.md`, `r1/cron.md`) against current sources:
- `scripts/deploy.sh`
- `src/main.rs`
- `src/dashboard.rs`
- `src/cron/scheduler.rs`

All relevant confirmed fixes (F5-F9, F10-F13, F35, F36) have been verified directly in the codebase.

---

## 2. Item-by-Item Verdicts on Confirmed Fixes

### A. Deploy Script (`scripts/deploy.sh`)

| ID | Issue Description | Expected Location | Verdict | Evidence / Current State |
|---|---|---|---|---|
| **F5** | `deploy.sh --verify` recursion `gateway_pid` -> `detect_old_pid` -> `gateway_pid` | `scripts/deploy.sh:212` | **VERIFIED** | Line 212 now redefines `gateway_pid` without calling `detect_old_pid`: `if [[ -z "$(gateway_pid)" ]]; then gateway_pid() { pgrep -f "$TARGET_BIN run" \| head -1 \|\| true; }; fi`. |
| **F6** | `launchctl print` failure under `set -euo pipefail` silently aborts after install | `scripts/deploy.sh:327-328` | **VERIFIED** | Wrapped in command group with fallback: `loaded_env="$( { launchctl print "gui/${UID:-$(id -u)}/$LABEL" 2>/dev/null \|\| true; } \| sed ... \| sort)"`. Exit 113 no longer terminates deploy. |
| **F7** | `DEPLOYED` written before verify and not restored on rollback; pins failed rev | `scripts/deploy.sh:346-353` | **VERIFIED** | `printf ... > "$INSTALL_DIR/DEPLOYED"` is relocated inside `if verify "$since" "$old_pid"; then`. If verification fails, `DEPLOYED` is untouched and rollback triggers cleanly. |
| **F8** | Missing `DEPLOYED` file aborts silently under `set -e` | `scripts/deploy.sh:221` | **VERIFIED** | Line 221 appends `|| true`: `deployed_rev="$(sed -n 's/^rev=\([0-9a-f]*\).*/\1/p' "$INSTALL_DIR/DEPLOYED" 2>/dev/null \| head -1 \|\| true)"`. |
| **F9** | `deploy.sh` executable bit lost | `scripts/deploy.sh` | **VERIFIED** | Git index and file system reflect executable permissions (100755 / `-rwxr-xr-x`). |

---

### B. Main Runtime (`src/main.rs`)

| ID | Issue Description | Expected Location | Verdict | Evidence / Current State |
|---|---|---|---|---|
| **F35** | `boot_slack` single failed `auth.test` or owner DM lookup at boot disables Slack permanently | `src/main.rs:804-870` | **VERIFIED** | Implemented `retry_slack_boot` (`src/main.rs:804-837`) with exponential backoff up to 6 attempts across ~60s. Fails fast on permanent auth codes (`invalid_auth`, `not_authed`, `account_inactive`, `token_revoked`, `token_expired`, `missing_scope`) while transient network errors retry. Applied to both `api.auth_test()` (line 848) and `owner_dm_key` (line 868). |

---

### C. Dashboard (`src/dashboard.rs`)

| ID | Issue Description | Expected Location | Verdict | Evidence / Current State |
|---|---|---|---|---|
| **F36** | Dashboard "Allow Session" on proposal-id approval downgraded to Once | `src/dashboard.rs:2458-2462` | **VERIFIED** | `resolve_approval` matches `"session" => omon_gateway::ApprovalDecision::Session`, matching `parse_custom_id` behaviour instead of collapsing to `Once`. |

---

### D. Cron Scheduler (`src/cron/scheduler.rs`)

| ID | Issue Description | Expected Location | Verdict | Evidence / Current State |
|---|---|---|---|---|
| **F10** | `cron_report` enqueued with `lane_key = NULL`; dispatch collapses across jobs | `src/cron/scheduler.rs:2442-2454` | **VERIFIED** | `report_key` constructed per job and destination: `let report_key = format!("cron:{}:{}:{}", job.id, destination.platform, destination.chat_id);` and passed as `Some(&report_key)` to `enqueue_lane_event`. Dispatch no longer merges distinct job reports. |
| **F11** | `review_session` has no default lead fallback, causing delivery failure when `OMON_LANE_DEFAULT_LEAD` unset | `src/cron/scheduler.rs:2652-2656` | **VERIFIED** | `review_session` provides fallback: `std::env::var("OMON_LANE_DEFAULT_LEAD").ok().filter(...).or_else(\|\| Some(crate::lanes::kibitzer::DEFAULT_LEAD.to_owned()))`. Fallback is returned if Slack team/bot lookup fails or platform is not Slack. |
| **F12** | `ack_command` runs when review is queued before delivery occurs | `src/cron/scheduler.rs:2393-2405` | **VERIFIED** | `reviews_before_delivery` returns `false` if `payload["ack_command"]` is non-empty (`is_none_or(\|command\| command.trim().is_empty())`). Jobs with `ack_command` bypass review and use direct delivery before executing `ack_command`. |
| **F13** | `~/.omon/cron-reports` files grow unbounded and unredacted | `src/cron/scheduler.rs:2430-2434`, `2685-2735` | **VERIFIED** | Writes use `redact_owner_tokens(content)` at line 2430. Periodic pruning `prune_cron_reports(&dir).await` runs on each write, removing report files older than 7 days. |

---

## 3. Deep Domain Inspection

### 1. Deployment Failure Propagation (`scripts/deploy.sh`)
- **Rollback mechanics**: When `verify "$since" "$old_pid"` returns nonzero, lines 354-374 execute rollback. Binary is restored from `$backup` via temporary file atomic move (`cp -p "$backup" "$INSTALL_BIN.rollback" && mv -f "$INSTALL_BIN.rollback" "$INSTALL_BIN"`).
- **Web bundle sync**: Served web directory is restored from `$web_backup` to prevent mismatch between gateway binary and dashboard bundle.
- **Rewind guard**: `DEPLOYED` file recording occurs only after verification succeeds (line 350). Aborted or rolled-back deploys do not corrupt the deployed rev stamp.
- **Exit codes**: Unhealthy rollback exits with failure message (`die`), cleanly propagating non-zero status to CI/supervisor.

### 2. Signature & Receipt Correctness (`scripts/deploy.sh`)
- Codesigning at line 278: `codesign --force --options runtime --timestamp --sign "$SIGN_IDENTITY" --identifier omo-gateway "$INSTALL_BIN.new"`.
- Strict verification at line 280: `codesign --verify --strict "$INSTALL_BIN.new"`.
- Receipt check at line 284: Asserts `TeamIdentifier=5DUM8WPB4C` before moving the binary to production path, aborting and cleaning up `$INSTALL_BIN.new` on failure.

### 3. Boot Retries (`src/main.rs`)
- `retry_slack_boot` wraps `auth.test` and `owner_dm_key`.
- Permanent token rejection is recognized early (`invalid_auth`, `not_authed`, etc.) to prevent hanging boot when misconfigured.
- Transient connection failures back off predictably up to 30s intervals.

### 4. Cron Delivery & Report Handling (`src/cron/scheduler.rs`)
- Lane event dispatch keying is isolated per job ID and chat destination (`cron:<job_id>:<platform>:<chat_id>`).
- Sensitive secrets in cron reports are scrubbed with `redact_owner_tokens`.
- Disk retention is bounded by 7-day TTL pruning.
- Output review respects `ack_command` and `direct_delivery` flags.

---

## 4. Reachable P0/P1 Defects

**None found in the reviewed scope.**
All confirmed defects F5-F9, F10-F13, F35, and F36 have been fixed cleanly. LSP diagnostics for `src/main.rs`, `src/dashboard.rs`, and `src/cron/scheduler.rs` report zero errors.

<results>
<files>
- /Users/indo/code/project/omon-gateway/scripts/deploy.sh - F5 (verify recursion), F6 (launchctl print under pipefail), F7 (DEPLOYED rollback pinning), F8 (missing DEPLOYED), and F9 (executable permissions) verified
- /Users/indo/code/project/omon-gateway/src/main.rs - F35 (boot_slack auth.test and owner DM retries) verified
- /Users/indo/code/project/omon-gateway/src/dashboard.rs - F36 (proposal approval "session" -> Session mapping) verified
- /Users/indo/code/project/omon-gateway/src/cron/scheduler.rs - F10 (cron report lane key isolation), F11 (review session default lead fallback), F12 (ack_command review bypass), and F13 (cron report token redaction and 7-day prune) verified
</files>

<answer>
All targeted fixes in `scripts/deploy.sh`, `src/main.rs`, `src/dashboard.rs`, and `src/cron/scheduler.rs` (F5-F9, F10-F13, F35, F36) have been audited and confirmed as **VERIFIED**. Deployment failure propagation, signature receipt verification, Slack boot backoff retries, and cron report isolation/lifecycle work as intended. No new reachable P0/P1 defects were identified in these modules.
</answer>

<next_steps>
Ready to proceed - no follow-up needed.
</next_steps>
</results>

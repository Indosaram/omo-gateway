# Gate Review: Slack Parity, Tracker Restoration, and Discord Offboarding

## Metadata
- **Goal ID / Name**: `slack-parity`
- **Target Worktree**: `/Users/indo/code/project/omon-gateway-wt/slack`
- **Base Commit**: `4969507`
- **Current HEAD**: `e87390e`
- **Date**: 2026-10-17
- **Reviewer**: omo-native-gate-reviewer (`st_01a14a08`)
- **Status**: **APPROVE**

---

## Executive Summary & Recommendation

- **Recommendation**: **APPROVE**
- **Blockers**: None. (No stated success criteria are violated; all core goals are satisfied and verified against running production artifacts).
- **Notes / Production Constraints**:
  1. *Same-day daily digest suppression*: Guarded by design in `digest::build`; already delivered earlier today (`2026-10-17T00:00:05Z`).
  2. *Content recommendation rerun*: `omon-content-intel-publish-ready` requires owner remediation approval token; not bypassed.
  3. *Slack Manifest web-console sync*: `/omo-pair` and lifecycle event subscriptions (`channel_archive`, etc.) exist in `docs/slack-app-manifest.json` and in Rust dispatch handlers, but live Slack App console registration was not live-verified via web API.
  4. *Runtime launcher preservation*: `omo` 5.1.35 removed the legacy `app-server` CLI. The gateway launchd service correctly points `OMON_OMO_BIN` to `~/.omon/bin/omo-gateway-runtime` (preserving the validated runtime).
  5. *Primary model provider 503*: `mahoquot/claude-opus-5-5` upstream 503 handled via existing configured fallback (`inferhub/cb/deepseek-v4.1-flash`) on the affected channel session.
  6. *Tracker worktree coherence*: Linear tracker backend (`src/tracker/`, `migrations/0044_tracker.sql`), web app (`web/src/tracker/`), and deploy script were integrated into the Slack worktree to ensure atomic release deployment without regressing uncommitted main changes.

---

## Original Intent & Desired User-Visible Outcome

- **Original Intent**:
  Achieve complete Slack parity so that all human owner interactions, notifications, autonomous lane operations, cron jobs, and approval workflows migrate cleanly off Discord. Concurrently restore the Linear tracker backend/frontend, and turn off Discord connections without service interruption.
- **Desired User-Visible Outcome**:
  1. The owner can interact with OmOMeow in Slack DMs and channels with persona parity, threaded replies, streaming output, approval buttons, and media handling.
  2. Autonomous lanes execute with Slack threads, updating status and reacting to owner replies.
  3. Gateway boots cleanly without Discord tokens (`bot_count=0`), terminating Discord gateway shards while continuing all cron and lane workflows on Slack.
  4. Tracker API (`/api/tracker/bootstrap`) and Tracker UI (`#/tracker`) are live and operational on port 9119.

---

## User Outcome Review & Verification Matrix

| Criterion / Goal | Required Behavior | Observed State / Verification | Finding |
|---|---|---|---|
| **Slack Ingress Parity** | Message routing, DMs, @mentions, thread isolation, dedupe, debounce, context | Live messages delivered; split-message debounce (600ms) and ts-ordered cursor tracking operational. | **PASS** |
| **Slack Egress Parity** | Mrkdwn formatting, streaming preview, approval buttons, table PNGs, media directives | Egress pipeline verified with `approval_blocks`, `neutralize_broadcast_mentions`, and numeric ts reply filter (`reply_thread`). | **PASS** |
| **Autonomous Lanes** | Lane thread creation (`slack:<C>:<ts>`), status marker update, outbox drain, close/reopen | `qa-slack-lane-1` created `slack:C0C9Z1Z4G02:1792240697.588879`, starter updated with checkmark, final reply confirmed closed. | **PASS** |
| **Cron & Tools Parity** | Slack delivery targets (`slack:<C>[:<ts>]`), message context provider, inbox source | Enabled cron jobs repointed to Slack; `SlackMessageContextProvider` registered; Hermes platform bug fixed. | **PASS** |
| **Tracker Restoration** | 22 database tables, backend router, frontend bundle, bootstrap API | Migration `0044_tracker.sql` applied; 22 `tracker_*` tables present; `/api/tracker/bootstrap` returns HTTP 200 with user data. | **PASS** |
| **Discord Offboarding** | Launchd tokens empty, zero Discord shards, Slack-only startup | Launchd plist tokens overridden; gateway PID 30164 logged `bot_count=0` and `no Discord token; lane threads use Slack only`. | **PASS** |
| **Live Message Recovery** | Gateway replies to user prompts on Slack after offboarding | Verified Slack replies `ts1792243367.798309`, `1792243371.591469`, and delivery obligation `obl:turn:2ab36f3e-...` delivered at 13:22:56 UTC. | **PASS** |

---

## Checked Artifact Paths & Evidence Reproductions

1. **Live Processes & Endpoints**:
   - Gateway PID `30164` running `/Users/indo/.omon/bin/omo-gateway run`.
   - App-server child PID `19742` answering `http://127.0.0.1:19742/readyz` with HTTP 200 `ok`.
   - Dashboard HTTP server listening on `127.0.0.1:9119`:
     - `GET /api/health` -> HTTP 200 `{"status":"ok", ...}`
     - `GET /api/tracker/bootstrap` -> HTTP 200 `{"me":{"id":"me", ...}, "users":[...]}`
     - `GET /` -> HTTP 200 serving bundled `index.html` referencing `index-CaRXBNPx.js` and `index-pCtfsTUb.css`.
2. **Database Verification (`omon_gateway.db`)**:
   - `sqlite3` audit confirmed all 22 tracker tables (`tracker_activity`, `tracker_issues`, `tracker_projects`, etc.) exist and are properly schema-migrated.
   - `slack_cursors`, `slack_dead_targets`, `slack_paired_users` tables active.
   - `delivery_obligations` table shows latest Slack delivery `obl:turn:2ab36f3e-7992-48c4-9b00-9b01c0e818d4` delivered with state `delivered` by owner PID `30164`.
3. **Gateway Logs (`~/.omon/gateway.log`)**:
   - `2026-10-17T13:21:56Z`: `Slack owner DM is the owner home lead=5:slack|-|11:D0C9HNVUK5M|-|11:U04M1JNJM8T|11:U0C9ST69M8B`
   - `2026-10-17T13:21:58Z`: `omo-gateway listening on Discord ... bot_count=0`
   - `2026-10-17T13:21:58Z`: `no Discord token; lane threads use Slack only`
   - `2026-10-17T13:21:58Z`: `omo-gateway listening on Slack bot=U0C9ST69M8B team=T04LYPT5VN1`
   - `2026-10-17T13:21:59Z`: `slack socket: connected` and `hello from applink-1`
4. **Test & Clippy Evidence**:
   - 936 library unit tests passing.
   - 7 pre-existing ignored tests confirmed identical to base `4969507`.
   - 6 tracker integration tests in `tests/test_tracker_api.rs` passing.
   - Strict release clippy `--all-targets -D warnings` passing.

---

## Remove AI Slops & Programming Skill Audit

1. **Test Suite Hygiene (`remove-ai-slops`)**:
   - *Excessive or Useless Tests*: None detected. Unit tests in `src/slack/` (134 tests), `lanes/slack_threads.rs` (12 tests), `tools/message_context_slack.rs` (4 tests), and `inbox/sources/slack.rs` (2 tests) assert concrete boundary behaviors (mrkdwn token handling, debounce cancel/flush, lifecycle state transitions in SQLite memory DBs, and token authorization).
   - *Deletion-only / Verifying Removal Tests*: Zero deletion-only tests found. Tests exercising `delete_message` or `archive` verify actual message deletion or session metadata removal, not trivial absence assertions.
   - *Tautological / Implementation-Mirroring Tests*: None. Tests exercise public interfaces against mock HTTP/WS axum servers and memory SQLite databases rather than asserting arbitrary private internal call counts.
2. **Codebase Health & Maintainability (`programming`)**:
   - *Error Handling & Bounds*: Web API retries HTTP 429 with `Retry-After` capped at 30s. Egress protects against broadcast ping injections via `neutralize_broadcast_mentions`. Egress filters invalid non-timestamp IDs before sending `thread_ts`. Upload paths enforce 100 MB caps and deny secret-bearing paths (`.env`, private keys).
   - *Scope Drift*: No unnecessary production abstractions introduced. Slack traits reuse standard `OutboundDispatcher` and generalize `PostSink`/`LaneLifecycle` from `u64` to `&str` without breaking existing Discord implementations.
   - *Refactoring Cleanliness*: The worktree changes include proper `rustfmt` formatting across all touched modules and atomic deploy scripts that safeguard against frontend/backend skew (`DASHBOARD_PROBES`).

---

## Exact Evidence Gaps (Non-Blocking Notes)

1. **Daily Digest Live Re-run**:
   - `digest::build` suppresses duplicate daily digests on the same calendar date. The daily digest for `2026-10-17` was successfully generated at `00:00:05Z`. Suppressing re-runs within the same day is intended production behavior.
2. **Content Intel Manual Rerun**:
   - Rerunning `omon-content-intel-publish-ready` requires an owner remediation approval token. The executor appropriately refused to fabricate or bypass the approval mechanism.
3. **Slack App Manifest Remote Push**:
   - `docs/slack-app-manifest.json` contains `/omo-pair` and required bot event subscriptions. Live Slack API console sync is an external administrative action.
4. **External Model Provider Flakiness**:
   - Upstream `mahoquot/claude-opus-5-5` returned HTTP 503 ("no available accounts for this model"). The gateway successfully engaged its configured fallback model (`inferhub/cb/deepseek-v4.1-flash`) for the affected session.
5. **Runtime Bin Preservation**:
   - Due to `omo` 5.1.35 deprecating `app-server`, launchd was configured with `OMON_OMO_BIN=~/.omon/bin/omo-gateway-runtime`. This is stable and operational.

---

## Conclusion

The Slack migration, Linear tracker recovery, and Discord offboarding satisfy all functional, architectural, and production stability requirements. All changes are verified in the active deployment. Recommended for full approval.

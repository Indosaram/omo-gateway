# Lane main-runtime Deliverable Report

## Direct retry audit (2026-10-20)

This retry directly read the current implementation and found the four requested fixes already
present. No Rust source was changed or rewritten. The authoritative four-line ledger below is
retained; the predecessor sections remain historical evidence, not fresh validation.

Static evidence checked in this retry:
- `src/inbox/health.rs:136,244`: the classifier rejects refresh preambles, DNS/connect/timeout
  failures and an embedded `1401`, while accepting explicit credential verdicts and standalone
  `401`. The existing focused test also checks four repeated transient failures stay Degraded.
- `src/main.rs:3356,3526`: the supervisor delegates authentication classification to that module;
  `record_watcher_health` calls `report_dead` only for Dead, never Degraded. A plain Failed run
  resets the authentication-failure counter in `src/inbox/health.rs:69`.
- `src/runtime_config.rs:9,17,31`, `src/lib.rs:20`, `src/main.rs:782,877`: OnceLock stores the
  override, the getter falls back to the environment, and Slack boot calls the setter.
- `src/storage/retention.rs:13,99,193`, `src/storage/mod.rs:5`: 30-day default, age and completion
  predicates, preserved active source health and existing dedupe dependencies, plus a focused
  database regression test. `src/main.rs:1182,1347,2002` schedules daily sweeps in both modes;
  shutdown joins them before pool closure at `:1358,2083`.

Only this report was edited by this retry. No children were spawned. No build, test, formatter,
commit, push, deployment, running-gateway change, engine-install change, or external action ran.
No implementation blocker remains in this lane; compilation and test execution remain unverified
under the explicit write-only restriction. The out-of-scope CLI readers listed below remain for
the lead. The four canonical ID lines were counted as four before this report update.

## Restart audit (2026-10-20, authoritative current ledger)

A2 FIXED src/inbox/health.rs:136 — preserved successor classifier; removed refresh-preamble-only auth verdict and recognized standalone 401 without requiring Unauthorized text; test inbox::health::tests::refresh_preamble_requires_a_credential_verdict (written, not run).
A3 FIXED src/main.rs:3356 — existing supervisor maps only classifier-positive failures to AuthFailed; src/main.rs:3536 reports dead only for HealthAction::Dead, and the classifier correction keeps repeated DNS/connect/timeout tails Degraded; test inbox::health::tests::refresh_preamble_requires_a_credential_verdict (four repeated failures, written, not run).
B16 FIXED src/runtime_config.rs:9 — existing OnceLock setter and environment fallback preserved; src/lib.rs:20 registers the module, src/main.rs:783 delegates reads and :877 uses set_default_lead instead of set_var; test runtime_config::tests::test_default_lead_runtime_config and legacy::headless_tests::test_default_lead_runtime_config_delegation (existing, not rerun).
B17 FIXED src/storage/retention.rs:13 — existing 30-day default and completed-row pruning preserved, with live source health and dedupe records retained; src/storage/mod.rs:5 registers the module; main.rs:1182 daily task is started in both gateway modes at :1347/:2002 and joined before pool close at :1358/:2083; test storage::retention::tests::test_prune_retention_only_removes_old_completed_rows (existing, not rerun).

This restart changed only `src/inbox/health.rs` and this report. Existing successor code,
historical evidence below, and other workers' changes were preserved. The prior description
below that permits `token expired` alone is superseded: a refresh preamble is never a credential
verdict. Existing explicit credential markers still outrank accompanying network errors.

Remaining verification: no builds, tests, formatting commands, deployment, engine modification,
external publishing, or calendar actions were run in this restart. The edit hook timed out
waiting for fresh LSP diagnostics; no compiler or test verdict is claimed. The owner integration
gate must run the new unit test. Historical gate results below are not current validation.

Out-of-scope direct environment readers found: `src/lanes/cli.rs:309,441` and
`src/sessions/cli.rs:204`. The lead can decide whether these one-shot CLI fallbacks should switch
to `runtime_config::default_lead`; no edits were made outside this lane. Other matches were
the runtime fallback, main's bootstrap guard, test child environments, or diagnostic strings.

Retention schema evidence: `migrations/0050_action_loop_state.sql:1` creates `action_loop_seen`,
not a table named action_loop_state. `migrations/0052_source_health.sql:1` is keyed by source and
stores current auth failure state, not completed history. `health::clear` deletes that row on
recovery. Keeping active rows avoids duplicate owner alerts and losing the dead-source marker.

## Preserved predecessor report (historical)

Scope: A2, A3, B16, B17 plus the lead's retention/B9 review notes. Files touched:
`src/inbox/health.rs`, `src/inbox/sources/{mail,mod}.rs`, `tests/test_source_health.rs`,
`src/runtime_config.rs`, `src/storage/retention.rs`, `src/main.rs`, this report.

Build/test status: no build or test of mine is authoritative. My own `cargo check --lib` exited on
the shared `target` lock and the gate owns compilation; gate run 1's results are quoted below, and
the only live provider check is the lead's A2 run. Anything below not tied to one of those is a
static review, not a compiler verdict.

## Status ledger

One line per ID: `ID STATUS - files - tests`. All four are implemented. Gate run 1 compiled and
ran them (see "Gate evidence"); the one live check that has run is A2's real zele fetch+parse.

- **A2 FIXED** - `src/inbox/health.rs` (three-way classifier, new `is_network_failure`),
  `src/inbox/sources/mod.rs` (`run_json` labels `auth_expired` through it) -
  `tests/test_source_health.rs::token_refresh_that_dies_on_dns_is_not_an_auth_failure`,
  `tests/test_source_health.rs::a_credential_verdict_outranks_a_network_error_in_the_same_tail`,
  `src/inbox/sources/mod.rs::run_json_tests::failed_refresh_preserves_network_error_for_health_classification`.
  Live: `live_mail_source_fetch_once` PASS, `QA_COUNT source=mail items=5` (fetch+parse only, see
  "A2 live evidence").
- **A3 FIXED** - no code change; `src/main.rs::supervise_watcher_with_health` reaches
  `health::report_dead` only through `is_auth_failure`, so A2 is the fix - test none (no test
  drives `supervise_watcher_with_health`; the classifier is covered by A2's tests).
- **B16 FIXED** - `src/runtime_config.rs` (new), `src/main.rs:782,875` -
  `src/runtime_config.rs::tests::test_default_lead_runtime_config`,
  `src/main.rs::headless_tests::test_default_lead_runtime_config_delegation`.
- **B17 FIXED** - `src/storage/retention.rs` (new), `src/storage/mod.rs` (module + re-export,
  pre-existing partial kept), `src/main.rs` (`prune_storage_once` 1120, `join_retention_task`
  1148 + `join_retention_task_within` 1156, `spawn_daily_retention_task` 1182, boot prune 1253 +
  daily task 1347/join 1358 in the Discord-free branch, boot prune 1629 + daily task 2002/join
  2083 in the Discord/Slack path) -
  `src/storage/retention.rs::tests::test_prune_retention_only_removes_old_completed_rows`,
  `src/storage/retention.rs::tests::prune_rejects_a_zero_day_window`,
  `src/storage/retention.rs::tests::retention_window_defaults_and_refuses_a_degenerate_value`,
  `src/main.rs::headless_tests::test_spawn_daily_retention_task_cancels_cleanly`.

Also fixed, no review ID: **B9** (`src/main.rs:1339`, startup claim-recovery failure is now
logged) and the mail date handling under **A2 follow-up** (`src/inbox/sources/mail.rs`).

## A2 FIXED — network failures no longer read as dead credentials
`src/inbox/health.rs`: `is_auth_failure` now splits markers three ways.
- `NETWORK_FAILURE_MARKERS` (ENOTFOUND, getaddrinfo, ECONNREFUSED, ECONNRESET, ETIMEDOUT,
  EAI_AGAIN, "fetch failed", "unable to connect", "network is unreachable", "timed out") drive
  the new `is_network_failure`.
- `AUTH_FAILURE_MARKERS` are credential verdicts (auth_expired, invalid_grant, unauthorized,
  unauthenticated, not authenticated, not logged in, login required, zele login, invalid
  credentials, credentials expired) and win outright, even when the same tail also shows a
  network error. The reverse mistake (a revoked refresh token that also logs a timeout staying
  silent) is the more expensive one.
- `"token expired"` alone is a weak marker: zele prints `Token expired, refreshing` before every
  refresh, so it counts only when no network marker is present. That is exactly the
  2026-10-20 02:10Z false alarm.

`src/inbox/sources/mod.rs`: `run_json` labels a failed run `auth_expired: …` only through that
classifier, restoring the `auth_expired` signal `inbox/digest.rs` and `main.rs:2958` look for
without re-introducing the DNS false positive.

Tests: `tests/test_source_health.rs` keeps the DNS regression and adds
`a_credential_verdict_outranks_a_network_error_in_the_same_tail` (auth+timeout tail, 401 with
ECONNRESET, plain EAI_AGAIN, `invalid_grant` is not a network failure).
`src/inbox/sources/mod.rs` keeps the `run_json` stderr test that proves the label is not applied
to a network tail.

## A3 FIXED — dead-source notification follows the classifier
No new code: `supervise_watcher_with_health` already routes `WatcherRun::AuthFailed` only when
`is_auth_failure(tail)` holds, so the A2 change is what stops a DNS outage from reaching
`health::report_dead`. The 2026-10-20 02:10Z `source auth is dead` line came from that path.

## Relative mail dates (A2 follow-up) — recognized forms only, no guessed timestamps
Verified against the installed zele 0.8.0: `src/commands/mail.ts:203` sets
`date: out.formatDate(t.date)` and `dist/output.js:216` emits `just now`, `14m ago`, `3h ago`,
`2d ago`, `Oct 3`, `Oct 3, 2025` via `toLocaleDateString` (locale-dependent month names).
Live run of `zele mail list` on 2026-10-20 returned `19h ago`, `21h ago`, `22h ago`, `2d ago`.

**Exact provider metadata is not reachable.** Every date zele prints goes through
`formatDate`: `mail list` (`src/commands/mail.ts:203`), `mail search` (`:292`), `mail read`
(`:414`, `Date: ${formatDate(msg.date)}`). The CLI has no `--json`/`--raw`/`--format` switch
(the only global option is `--account`, `src/cli.ts:31`) and `printList`/`printYaml`
(`src/output.ts:221`) only serialize the already-formatted string. Reading zele's private SQLite
cache instead would couple the gateway to an experimental, undocumented store, so that is not
something to build on.

`src/inbox/sources/mail.rs`:
- `display_date` parses every form zele emits to its **lower bound** (`"just now"` = now-1m,
  `Nm/Nh/Nd ago` = now-(N+1) unit, `Mon D[, YYYY]` = local midnight of that day, a future
  month/day rolls back one year). Lower bound is the safe direction for the `after:` cursor: an
  early timestamp re-fetches, a late one would skip mail.
- An unreadable date is now a **hard error, never a guess**. `parse_at` collects the offending
  threads and their raw values and fails the batch: `collect_all` records it as the cursor error
  and `commit_source` never runs, so nothing is committed, the cursor stays where it was, and the
  same rows are retried. The message names the threads, shows the raw date, lists the accepted
  forms, and states that nothing was committed. It matches no auth or network marker, so it is a
  plain degraded retry, not a false "auth is dead".
- Why no fallback at all: an epoch (or cursor-floor) timestamp makes `triage::classify` see
  `now - received_at > STALE_AFTER_HOURS` and file fresh mail as `fyi`, and `(source, source_ref)`
  dedupe plus `raw_hash` equality keeps that wrong verdict on every later fetch.
- Day parsing accepts `%b %e, %Y` and `%b %d, %Y` because `toLocaleDateString` pads the day
  differently per platform; that is the same format, not a guess.
- Cursor: `newest_ts` is seeded from the stored cursor and combined with `max` per readable row,
  so the next cursor is always >= the old one — it **cannot move backwards**. A lower-bound
  timestamp can therefore only *understate* the newest thread: the next `after:` re-fetches that
  window, dedupe on `(source, source_ref)` absorbs the repeats, and mail is never skipped.
  Unchanged rows never re-triage (`raw_hash` equality short-circuits).

Tests (in-module): `zele_display_dates_parse_to_their_lower_bound` (all six forms, cursor, and
rejection of `sometime`, an overflowing day count, and a negative one) and
`an_unreadable_date_fails_the_batch_instead_of_guessing` (localized `10월 3일` fails with the
thread id and raw value, is not an auth failure; a missing date reports `<missing>` instead of
becoming an epoch timestamp).

## Calendar-worker coordination — nothing left to remove
`src/autonomy/loop.rs` (other lane) already dropped the seven-day candidate window: the scan
now carries "No age window: an item that was never emitted must survive an arbitrarily long
outage, so staleness is marked (`[backlog]`) rather than filtered out", and its comment states
the dedupe contract this retention module implements (a seen mark is kept while the item row
exists; `enqueued`/`executed` outcomes are never pruned). `grep` finds no remaining 7-day window
in the action-loop or calendar path; the two surviving seven-day constants are unrelated
(`discord/attachments.rs:18` cache TTL, `cron/scheduler.rs:2689` cron report *file* pruning).
The loop's `item_from_a_long_outage_is_emitted_as_backlog_not_dropped` test covers the behaviour
that made the cutoff unnecessary. No edit was needed, and `loop.rs` is outside this lane's scope.

**Retention contract the loop now relies on (do not weaken):** `action_loop_seen` is pruned only
when its item row is gone, and `action_loop_outcomes` keeps `enqueued` and `executed` forever.
Both are asserted in `test_prune_retention_only_removes_old_completed_rows`.

## B16 FIXED — runtime default lead without `env::set_var`
`src/runtime_config.rs`: `set_default_lead` is first-write-wins and refuses blank values, so a
Slack reconnect cannot move the owner home under running lanes; `default_lead` prefers the
runtime value, then a non-blank `OMON_LANE_DEFAULT_LEAD`, then `kibitzer::DEFAULT_LEAD`.
`src/main.rs:875` calls `set_default_lead(key.storage_key())` instead of `env::set_var`, keeping
the same guard (`optional_env(...).is_none()`) and the same boot position; `default_lead()` in
main delegates to the module.

Tests: `runtime_config::tests::test_default_lead_runtime_config` (lib) and
`legacy::headless_tests::test_default_lead_runtime_config_delegation` (bin) each re-exec the test
binary (`--exact`) and assert the child ran exactly one test, so a renamed test cannot pass
vacuously. The delegation test now observes the OnceLock through the child instead of comparing
two reads of the same process state, which was true by construction before.

The bin selector is **derived**, not hardcoded. `src/main.rs` is `include!`d as `mod legacy` inside
the `omo-gateway` binary (`src/entry.rs:9-11`), so the harness names this test
`legacy::headless_tests::test_default_lead_runtime_config_delegation`. The first version hardcoded
`headless_tests::test_...`, which matched nothing: `--exact` exits 0 having run nothing, so the gate
saw `panicked at src/main.rs:2240: child ran no test: running 0 tests`
(`.omo/evidence/review-20261019/gate/review-20261020-integrated1/test.log:1289-1295`).
`child_test_selector` now builds the path from `module_path!()` minus its crate-root segment, so it
stays correct under any wrapping module, and the `"1 passed"` count guard is kept as the thing that
actually catches a wrong selector.

That stripping rule is measured, not inferred, with an isolated `rustc --test` fixture
(`mod legacy { include!("included.rs"); }`, built as both a bin-shaped and an
integration-shaped crate):

| Selector | bin-shaped crate | integration-shaped crate |
| --- | --- | --- |
| `module_path!()` value | `bin_entry::legacy::headless_tests` | `test_review_dashboard::legacy::headless_tests` |
| `--list` / runnable name | `legacy::headless_tests::probe_target` | `legacy::headless_tests::probe_target` |
| `--exact <full module_path>` | 0 tests, exit 0 | 0 tests, exit 0 |
| `--exact <crate segment stripped>` | 1 passed | 1 passed |

So libtest strips exactly the crate-root segment from the displayed and matchable name, and both
the unstripped path and a hardcoded `headless_tests::...` run nothing while still exiting 0. The
helper itself was then run inside the same fixture (`helper_matches_libtest_name`): it produces
`legacy::headless_tests::probe_target` and its nested child reports `1 passed`.

Note on the gate evidence: run 1 compiled code with the hardcoded literal, so it never exercised
either derived helper. The `child ran no test` panic at `src/main.rs:2240` is evidence against the
hardcoded string only.

## B17 FIXED — retention that cannot eat live state
`src/storage/retention.rs` rewritten against the lead's review:

| Table | Pruned | Why it is safe |
| --- | --- | --- |
| `turn_status_messages` | `state <> 'running'`, `updated_at < cutoff` | a `running` row is the restart-adoption record (`report/status.rs:115`) |
| `action_loop_seen` | `seen_at < cutoff` **and the item row is gone** | the mark is the only thing stopping `scan_unseen_candidates` from re-enqueueing an existing item |
| `action_loop_outcomes` | `verified` and `failed`, `updated_at < cutoff` | `verified` carries an external id; `failed` is the loop's give-up. `enqueued` is in flight and `executed` is not externally confirmed, so both stay |
| `session_watch_notices` | `sent_at < cutoff` | dedupe keys carry a per-session message count or question hash, so an expired key cannot re-fire a live notice |
| `report_claims` | `state <> 'pending'` | pending rows are swept to `unmeasurable` at boot |
| `source_health` | **never** | the row is the live dead-source marker plus its once-per-KST-day notification claim; deleting it would drop an active dead source and let the next failure notify again the same day |

- `prune_table_if_exists` is gone. Every table is created by migrations, so a missing table is a
  broken schema and the `sqlx` error propagates instead of reporting `0`.
- Retention window: `OMON_STORAGE_RETENTION_DAYS` (`retention_days_from_environment`), minimum 1;
  0 or negative is a config error rather than "delete everything every sweep". `prune` reads that
  window too, so no caller can silently use the 30-day default while the operator configured
  something else; `parse_retention_days` is split out so the parsing is unit-tested without
  mutating process-wide environment state.
- `src/main.rs`: one `prune_storage_once(pool, retention_days, when)` helper is the single sweep
  for both gateway modes, so the logged fields cannot drift between them.

### Every gateway mode sweeps
`run_gateway_with_dashboard` has two mutually exclusive modes, and both now prune:

| Mode | Entered when | Boot prune | Daily task | Joined on shutdown |
| --- | --- | --- | --- | --- |
| Discord-free (headless) | `OMON_DISCORD_ENABLED=false` (line 1236) | 1253 | 1347, on `shutdown` | 1358, before `pool.close()` 1365 |
| Discord/Slack | default (`OMON_DISCORD_ENABLED` unset or true) | 1629 | 2002, on `lane_shutdown` | 2083, before `pool.close()` 2088 |

The headless branch returns at 1366 without ever reaching the Discord path's setup, so it needed
its own boot prune and task; it runs the same inbox collector, lanes and action loop against the
same database, which is why the gap mattered. Both tasks are joined explicitly instead of being
detached: each holds a pool handle, and `pool.close()` would otherwise race a sweep.

The join is bounded and actually ends the task (`join_retention_task_within`): the timeout
borrows the handle (`timeout(limit, &mut task)`) instead of consuming it, because
`timeout(limit, task)` only *drops* the handle on expiry, which detaches the task and leaves the
sweep free to race or block `pool.close()`. On expiry the task is aborted and then awaited, so
the call returns only once the future has been dropped. `join_retention_task_aborts_and_reaps_a_task_that_ignores_shutdown`
pins that down: a task that never resolves holds a `Drop` guard, and the flag is set only if the
abort actually dropped the future — a detach would leave it unset.

**Does the real Slack-only run use the headless branch?** No. `ai.omon.gateway.plist` sets no
`OMON_DISCORD_ENABLED` (only `OMON_HOME_PLATFORM=slack`, empty `DISCORD_BOT_TOKENS`), so
`parse_bool_from(None, true)` returns true and the process takes the Discord/Slack path, where
retention was already wired. The headless branch is reached only when the variable is explicitly
false, which `docs/AGENTS_DELEGATE.md:11,169` documents as the supported Discord-free lane mode.

Tests: `test_prune_retention_only_removes_old_completed_rows` at a fixed `now` (2026-10-20) with
fixed old/recent stamps — it asserts the pruned counts per table, that a `running` status row, a
`source_health` row, a `seen` mark for a still-existing inbox/lane_event/source_health item and an
unknown-prefix mark all survive, and that `executed`/`enqueued` outcomes survive.
`prune_rejects_a_zero_day_window` covers the window guard and
`retention_window_defaults_and_refuses_a_degenerate_value` covers the env parser
(`""`, `"0"`, `"-1"`, `"30d"`, `" 30"`).
`headless_tests::test_spawn_daily_retention_task_cancels_cleanly` keeps the shutdown check, and
`headless_tests::join_retention_task_aborts_and_reaps_a_task_that_ignores_shutdown` covers the
bounded-join expiry path.

## Gate evidence (integrated run 1)
`.omo/evidence/review-20261019/gate/review-20261020-integrated1/test.log`. Every test this lane
touched passed in that run:

| Test | Result |
| --- | --- |
| `inbox::sources::mail::tests::zele_display_dates_parse_to_their_lower_bound` | ok |
| `inbox::sources::mail::tests::an_unreadable_date_fails_the_batch_instead_of_guessing` | ok |
| `inbox::sources::run_json_tests::failed_refresh_preserves_network_error_for_health_classification` | ok |
| `storage::retention::tests::test_prune_retention_only_removes_old_completed_rows` | ok |
| `storage::retention::tests::prune_rejects_a_zero_day_window` | ok |
| `storage::retention::tests::retention_window_defaults_and_refuses_a_degenerate_value` | ok |
| `runtime_config::tests::test_default_lead_runtime_config` | ok |
| `legacy::headless_tests::test_spawn_daily_retention_task_cancels_cleanly` | ok |
| `legacy::headless_tests::join_retention_task_aborts_and_reaps_a_task_that_ignores_shutdown` | ok |
| `legacy::headless_tests::test_default_lead_runtime_config_delegation` | **FAILED** in run 1 (wrong child selector, fixed after) |

Five lib tests failed in that run, all in files outside this lane: `autonomy::action_loop::tests`
(`readback_matches_the_parsed_id_and_the_whole_item_key`,
`item_is_done_only_when_the_calendar_reads_the_event_back_for_it` — stub calendar returns "event not
found"), `context::pack::tests::precedent_lookup_queries_lane_owned_worktrees` (SQLite FK 787),
`report::claims::tests::unclosed_measure_marker_keeps_rest_of_reply` (assertion),
`sessions::watch::tests::mark_sent_failure_after_a_successful_send_does_not_send_again`
(`PoolTimedOut`). Reported, not touched: they belong to other lanes.

## A2 live evidence — real zele fetch + parse
Run by the lead against the real provider, not by this lane:

```
cargo test --test test_inbox_sources -- --ignored --exact live_mail_source_fetch_once --nocapture
```

- Result: **PASS**, exit 0, `1 passed; 0 failed`.
- Emitted evidence line: `QA_COUNT source=mail items=5` — five real threads parsed.
- Test: `tests/test_inbox_sources.rs:974` `live_mail_source_fetch_once` (pre-existing `#[ignore]`
  QA harness, unchanged by this lane).
- Monitor: `mon_P3XVVHYRDXMCD570`.

**What this proves.** One real `zele mail list` subprocess (`MailSource::fetch_since(None)`,
60s timeout) produced a payload that `MailSource::parse_at` accepted: no
`unreadable date` error, no `auth_expired`/network misclassification, and five `InboxItem`s built
from real relative display dates. That is the A2 parse path exercised end-to-end on live data
instead of a fixture, which is what the 2026-10-19 `mail thread has invalid date` failure needed.

**What this does not prove**, and must not be read as proving:
- **No DB collection.** `fetch_since` returns before `commit_source`, so nothing was inserted into
  `inbox_items`, no cursor advanced, and the dedupe/`raw_hash` behaviour is unproven here. The
  collect→insert path is covered only by fixtures
  (`kakao_bad_endpoint_collects_other_three_sources_and_preserves_cursor`) and has not been run
  against live mail.
- **No deployed watcher.** This was a test binary, not the gateway: `run_mail_watch_once` /
  `supervise_watcher_with_health` did not run, so nothing is proven about the live process, the
  backoff/health wiring, or the 2026-10-20 production behaviour after a deploy.
- **No `--source mail` CLI path.** `CollectSource` has only `Github`, so that command still cannot
  reach mail.

## B9 FIXED — startup claim recovery is no longer silent
`src/main.rs:1339`: `let _ = recover_stale_report_claims(...)` became a `match` that logs
`recovered` at info and a failure at warn. The call stays at the same boot position, before any
turn can post a claim, which is what makes the clear-all-pending sweep correct; the comment now
says so.

## For the lead
1. Nothing here has been compiled. `cargo check` / `cargo test` were not run: the single
   `cargo check --lib` I started exited on the shared `target` lock and the integrated run is
   yours. Formatting is not a compiler verdict, but every touched file is now `rustfmt --check`
   clean with **zero** diffs — `src/main.rs` was reformatted to zero rather than left at its
   inherited 13, so the gate has no outstanding formatting to report.
2. Mail behaviour change worth knowing before the next deploy: a zele date form the gateway does
   not recognize now stops mail collection with an explicit cursor error naming the raw value,
   instead of collecting with an invented timestamp. That is deliberate — the alternative silently
   misfiles mail — but it means a zele format change is a visible outage, not a quiet one.
3. Retention behaviour change: `source_health` rows are permanent until `health::clear` runs on
   recovery. Outcomes prune only `verified` and `failed`: `verified` is now a real provider
   readback (`record_outcome` runs `<calendar cmd> cal get <id>` and requires the event to carry
   the item key), and `failed` is the loop's give-up. `executed` is the calendar worker's normal
   answer and only says the lead acted, so those rows persist until the item is verified or fails;
   that is the point, not an oversight.
4. `src/lanes/cli.rs:309,441` and `src/sessions/cli.rs:204` still read
   `OMON_LANE_DEFAULT_LEAD` from the environment. They are one-shot CLI processes where the
   environment is the only channel, so they are correct as-is; the `OnceLock` is process-local.
5. `run_json`'s new `auth_expired:` label applies to every source CLI, not just mail, so a
   `gh`/`imsg` tail with "not logged in" is now labelled the same way. That matches what
   `inbox/digest.rs` asks for.
6. Mode coverage for B17 is now both branches of `run_gateway_with_dashboard`, with the daily task
   joined before `pool.close()` in each. The live Slack deployment never took the headless branch
   (no `OMON_DISCORD_ENABLED` in its plist), so this is a correctness fix for Discord-free mode
   rather than a change to the running service.

# One final verification pass

Prepared only. No Cargo command or gateway binary was executed while preparing
these artifacts. The integration owner is `st_01a12db2`; its readiness receipt is
`.omo/evidence/dori-grade-agent-env/final-integration.md` in this worktree.

When the owner starts the final pass, invoke:

```bash
bash /Users/indo/code/project/omon-gateway-wt/dori-work/.omo/evidence/dori-grade-agent-env/final-verify.sh
```

The script first acquires a persistent `final-verify.claim` directory, then waits
for that receipt and for `pgrep -x cargo` to find no Cargo process anywhere on this
machine. The one-second readiness loop is a cheap local check, not a model call
or a test wait. The owner must keep source and other Cargo producers quiescent
through completion; the process check is a readiness snapshot, not a system-wide
lock. A second invocation is refused, including after an interrupted attempt.

## Batch and evidence

1. Capture `final-verify-pre.sha256`: every file in `src`, `tests`, `migrations`,
   `bot_profiles`, plus `Cargo.toml` and `Cargo.lock`, SHA-256 with C byte-order
   sorting of relative paths.
2. Run `cargo fmt --all -- --check` once.
3. Run strict all-target/all-feature Clippy once.
4. Run `cargo test --offline --locked -j4 --workspace --all-features
   --no-fail-fast -- --format pretty` once. No selector, filter, `--ignored`,
   focused repeat, or per-todo verification invocation. Default Cargo test
   selection includes library, binary, integration tests, and doctests, unlike
   `--all-targets`, which would omit doctests. Existing ignored live tests remain
   ignored and are reported as such; this pass does not certify live deployment.
5. Parse each target's terminal summary and named failures, writing separate
   `final-verify-target-*.log` files. The last harness summary per target is used;
   native child helper summaries are not summed into parent totals. Missing
   targets, incomplete summaries, and count/name mismatches fail classification.
6. If fmt and Clippy passed, build `omo-gateway` once, even if tests have named
   failures, as requested. Always capture the post manifest and compare it. Copy
   only a successful, source-identical build to
   `target-w4/release-bins/<binary-sha256>/omo-gateway`, then rehash the copy.
   This is a frozen debug build, not a signed deployment or release-profile build.
7. Write `final-verify.json`: gate exit codes (null means not run), target pass,
   fail and ignored counts, exact new failures, parsing/completeness errors,
   source manifest paths/hashes, copied binary path/hash, integration receipt
   identity, and retained owned TMPDIR. Exit 0 means a complete baseline-bounded
   pass, not necessarily zero test failures. Failures never trigger reruns.

Every Cargo command inherits `RUSTC_WRAPPER=`,
`CARGO_TARGET_DIR=$PWD/target-w4`, `CARGO_BUILD_JOBS=4`, and
`CARGO_NET_OFFLINE=true`. Clippy, test, and build also explicitly use `-j4`,
`--offline`, and `--locked` (preventing Cargo.lock updates). Cargo fmt does not
accept `-j4` or `--offline`; the inherited settings apply there. The script writes
only evidence, owned temporary state, and build artifacts, never source or .env.
Its owned `/tmp/dori-final-verify.*` directory remains available after completion
or failure for diagnosis. Python 3.9's standard library suffices; no TOML package
or installation is needed.

## Authoritative failure baselines

Todo 2 receipts were located by searching `baseline` in the existing evidence.
`batch-integration-plan.md` points to the authoritative baseline worktree:

`/Users/indo/code/project/omon-gateway-wt/dori-grade-agent-env/.omo/evidence/dori-grade-agent-env/task-2-failing.txt`

Read alongside `task-2-summary.txt`: HEAD `245a587`, 16 named failures across
six targets. The script snapshots that failure file into
`final-verify-baseline.txt`; it never regenerates the baseline. For every target
except `test_omo_backend`, comparison uses exact `(target, failure name)` identity
from that file. A target absent from it has an empty allowed-failure set.

The explicitly requested current canonical backend set overrides the historical
10 backend names and permits only:

- `test_omo_backend_e2e_thread_lifecycle_and_streaming`
- `test_omo_backend_aborts_turn_on_repeated_approval_denials`
- `test_omo_backend_deadline_interrupts_looping_turn`

Every other backend failure is new, even if present in the historical todo 2
file. Name matching does not establish equivalence of panic causes.

## Acceptance-test mapping

Source of requested criteria:
`/Users/indo/code/project/omon-gateway/.omo/plans/dori-grade-agent-env.md`.
All listed Cargo targets are covered by the single unfiltered suite; these rows
are a mapping, not instructions to run additional tests.

| Todo | Acceptance behavior | Test targets / batch checks |
| --- | --- | --- |
| 15 | Close authorization; dirty worktree refusal and close_failed; reopen owner thread exactly once without chat routing; restart event replay | `test_lane_close`, `test_lane_reopen`, `test_lane_runner`, `test_lane_dispatch`, `test_lane_events`; adapter unit coverage in `lib:omon_gateway` |
| 16 | Unarmed/disabled zero turns; score gate; one wake event; daily cap; malformed decision refusal; 6000-character cap | `test_lane_kibitzer`; enabled/disabled startup `qa_gaps` tests in `bin:omo-gateway` |
| 17 | One guard event per transition; no repeated-sample events; queued recovery without CPU-only hold | `test_lane_guard_alerts`; guard unit tests in `lib:omon_gateway`; runner ownership in `test_lane_runner` |
| 18 | Two milestones/two posts/two edits; blocked rename; 404 replacement ID; persisted suppression on restart | `test_lane_status_transport`, `test_lane_post_transport`, `test_lane_thread_transport` |
| 20 | Append once/idempotence; marked-region replacement; untouched profile columns; unknown bot exit 2; shipped event and lane CLI sentinels | `test_profile_import`; batch `todo20_sentinels` parses `bot_profiles/omomeow.json` for `[LANE_EVENT]` and `omo-gateway lane start` |
| 24 | Empty pass zero calls; gated batching; decisions; unknown IDs/malformed JSON; urgent event; kill switch | `test_inbox_triage`, supporting `test_inbox_store`; inbox startup `qa_gaps` in `bin:omo-gateway` |
| 25 | Seeded CLI filters and due/received ordering; set transitions; empty digest returns None | `test_inbox_cli_digest`; cron digest integration in `test_cron_report` |
| 26 | Explicit owner approval starts one linked lane; no approval/no auto lookup produces no action; duplicate/restart protection | `test_inbox_proposals`; adapter proposal interception in `lib:omon_gateway` |
| 28 | Exact local/SSH send argv and idempotency; exit 3 retention message; adopted external lane done lifecycle | `test_sessions_control` (`local_send_has_exact_argv_and_idempotency_key`, `remote_send_preserves_text_and_exact_ssh_words`, `exit_three_retains_not_running_message`, `adopted_lane_reads_claim_then_settles_after_objection_window`) |
| 29 | Persona exposes sessions CLI sentinel | Batch `todo29_sentinel` parses shipped JSON for `omo-gateway sessions list`; no separate Cargo target is specified by this criterion |
| 30 | Synchronous failed/delivery_failed/stuck/empty_success/missed problems; clean recovery; kill-off and hook-error isolation | `test_cron_health`; includes real scratch SQLite and scheduler/fake executor integration |
| 31 | Non-publishing remediation lane and rerun signal; publishing readiness/owner approval; retry ceiling; natural recovery without rerun | `test_cron_remediation`, supporting `test_cron_health`, `test_lane_done` |
| 32 | Seeded collapsed healthy jobs and problem lane keys; run_id/evidence outcome; health sentinel | `test_cron_report` (`seeded_health_collapses_ok_jobs_and_lists_problem_lane`, `outcome_contains_native_run_id_and_machine_evidence`, `persona_block_exposes_health_command_sentinel`) |
| 35 | Purpose token totals; usage JSON; low fulfills reopens; unavailable judge skips while signals close | `test_usage_done_gate`, supporting `test_judge`, `test_lane_done`; backend usage notification ingestion in `test_omo_backend` |

Machine sentinel checks run in the same batch's evidence parser, without another
Cargo invocation or a gateway execution. The requested list contains 14 todos;
all 14 are mapped above. Shell-only tests and live service QA are not silently
claimed as Cargo-suite coverage.

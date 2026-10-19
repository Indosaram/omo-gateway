# Todo 35 source settlement

Workspace: `/Users/indo/code/project/omon-gateway-wt/dori-work`.

Status: owned implementation settled; focused test is running. This is not a claim of integrated or live acceptance.

## Requirements and dependency evidence

Read the full todo 35 action text, Must NOT do, acceptance and QA at the main plan lines 545-552. Read `todo13-final-gate.md` (CONFIRMED), `../dori-grade-agent-env-todo33-gate-review.md` (confirmed), and `../dori-grade-agent-env-todo34-gate-review.md` (confirmed). Their full-suite baseline exceptions do not become a passing full suite here.

## Applied source

- `src/usage.rs`: eight typed purposes; exact app-server last-turn counters; stable session/thread/turn identity; snapshot upsert instead of summing repeated notifications; stale frame rejection; per-purpose/model totals; judge attempt/fallback counts and ratio; cap state; bounded since parsing; JSON/text output; KST previous-day digest summary; lane start revision persistence and purpose resolution.
- `migrations/0039_model_usage.sql`: `model_usage`, time index and `lane_usage_context`. The latest number at initial inspection was 0037; 0038 was explicitly reserved for todo 15 and subsequently appeared as `0038_lane_lifecycle.sql`. 0039 was the next available unreserved number.
- `src/lanes/settle.rs`: after signals and worktree checks pass, judge the stored start revision against title plus brief (maximum 1500 characters). `fulfills < 0.65` or `risk > 1.5` produces not_done and existing inbox guidance. Verdict numbers persist in fenced lane history. A missing revision, unreadable brief, or unavailable judge records/logs `done_diff skipped` and proceeds on signals. Existing generation/claim fencing remains in force.
- `src/judge/mod.rs`: DoneDiff outages and caps return an unavailable result, never a fabricated failing verdict or model fallback. Other judge purposes retain their existing behavior.
- `src/lib.rs`: export the usage module.
- `tests/test_usage_done_gate.rs`: nine focused fake-only tests covering snapshots, purpose totals, stale/negative frames, seeded JSON and time window, KST/filter counters, unrelated and risky verdicts, threshold acceptance, unavailable-judge signal completion, and disabled/capped no-model fallback. Local git fixture setup uses no commits.

The counter snapshot and outage-policy comments document non-obvious accounting and fail-open contracts; they are retained for that reason.

## Integration artifact, not applied

`task35-wiring.patch` uses apply_patch format. It includes CLI entry wiring, lane start capture, escalation purpose update, the real app-server notification consumer, digest summary insertion, and DB attachment to ephemeral inbox/kibitzer/judge backends. The named forbidden files were not edited by this worker: runner.rs, cli.rs, storage/lanes.rs and discord/adapter.rs. Additional cross-owner call sites are also supplied only as patch hunks.

The patch is not compiled or applied. Thus the real `usage` subcommand, automatic token ingestion, new-lane baseline capture, and 09:00 digest line remain integration work for the owning worker. There is no live todo 8 lane-close/usage QA or README-only live lane QA. Their fake equivalents exercise the owned accounting and settlement code, not the unintegrated CLI or WebSocket path. Explicit `usage_purpose` metadata is the contract for digest/cron_fix or other callers that lack a dedicated key; unknown purposes are not guessed from a model ID.

## Verification

Scoped `rustfmt --edition 2021 --config skip_children=true` followed by the same scoped `--check` completed with exit 0 for usage.rs, settle.rs, judge/mod.rs and the focused test. No repository-wide format was run.

LSP error diagnostics were requested for all four files before cargo. Every request timed out after 30000 ms because the shared LSP daemon did not respond. This is unavailable diagnostic evidence, not a clean LSP claim.

Exactly one focused command was started via a 1800-second monitor:

```sh
cargo test --test test_usage_done_gate --target-dir target-w35 -j2 --offline
```

Monitor: `mon_A0E4Y7GHY60BYBCY`, terminal `bash_7`. Completion pending at this document's creation. No duplicate test, broad build, service change, environment-file edit or git commit was performed.

## Source identities after scoped formatting

```text
8edc5b337d2cf20a153eda553f02baaf689420fd5cc8a37f2645b71b013226e0  src/usage.rs
d2df78bad9c34ad609ba6d59f8d9777bf0eedd716fbe179a3d0724de66f1a8c2  src/lanes/settle.rs
a5b091c4ac25d1fd38ba292dec9f39228dac66b7a76f9e25258f498f2568e7a8  src/judge/mod.rs
7ecc6458287e70f371c32de985737fb8f8397472ba4bfd601c68816048841bf9  tests/test_usage_done_gate.rs
3c531c16fa32269de828567d943c17fa4a6d4d55c45c049a74db2a00728b1915  migrations/0039_model_usage.sql
```

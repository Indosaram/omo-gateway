# Coordinator receipt: shared todo12 integration wiring

Status: coordinator wiring only; this is not the todo12 owner's source receipt
and does not claim todo12 completion. The owner-authored `task-12-integrated.md`
path is reserved for the owner. Its prior evidence needs restoration from the
owner's transcript; do not infer or recreate it here. Todo12's object-level
blocker and todo23/todo30 source fixes remain active, so integrated source is
not frozen. Read-only reviews alone need not delay a freeze.

## Integrated source paths

- `src/lanes/activity.rs`, `src/lanes/posts.rs`, `src/lanes/runner.rs`,
  `src/lanes/mod.rs`, `src/agent/omo_backend.rs`, `tests/test_lane_events.rs`,
  and `tests/test_lane_reopen.rs`: todo12 implementation and lifecycle fixes,
  pending the owner's restored evidence and object-level fix.
- `src/main.rs`: coordinator removed obsolete Herdr heartbeat startup/shutdown,
  retained generic heartbeat clocks, and wires the inbox collector to native
  startup and shutdown in Discord-enabled and Discord-free paths.
- `src/work_heartbeat.rs`: concrete `HerdrCliLivenessChecker` and its `herdr`
  subprocess implementation removed; generic clock, liveness trait, and fake
  checker APIs remain.
- `src/inbox/mod.rs`: exports `sources`; todo23 adapters remain owner source.
- `src/cron/mod.rs` and `src/cron/scheduler.rs`: canonical public `cron::health`
  module and single parent import, wired against todo30 source API.
- `Cargo.toml`: integration test registrations for `test_lane_events`,
  `test_cron_health`, and `test_inbox_sources`, alongside previous registrations.

No runner or adapter API was independently changed in this receipt. No schema,
migration, environment file, live database, service, or commit was changed.

## Inbox startup collection

Both native startup branches construct Mail, Kakao, iMessage, and read-only
Discord adapters over the existing gateway SQLite pool. A model-free Tokio task
calls the existing `collect_once`/`collect_all` path immediately and repeats on
a five-minute local timer. That existing path isolates per-source failures and
persists cursors/errors. Startup logs only per-source counts or error strings;
it does not log collected snippets. The task is aborted before the shared pool
closes. This is timer-based collection, not an event watcher; no model is polled
and no new daemon is launched. Default filter loading uses the existing API,
which creates `~/.omon/inbox-filters.json` if absent.

The todo23 source receipt reports Zele `mail list` without `--json`, structured
JSON/YAML parsing, PirateTalk pagination that omits initial `before_id=0`, and
iMessage object/numeric identifier support. Its owned-file rustfmt check is
reported exit 0. Independent adapter review `st_01a12a49` remains active; review
concerns about absent mail `items` and RFC3339 `internal_date` cursor extraction
are recorded in `batch-integration-plan.md`. No coordinator shim was added.

## Test and runtime status

No current-tree Cargo tests, clippy, format gate, compile, scratch gateway QA,
or live adapter QA were run for this coordinator integration. Todo12's worker
receipt must be restored by its owner from their transcript before its concrete
test status can be represented here. Todo23 reports its focused cargo test was
not retried because shared compile blockers remain. Todo30 source fixes are
active; its focused test previously failed before execution. Todo9 runner
generation repair and todo12's object-level fix remain required.

## Coordinated gate driver

`.omo/evidence/dori-grade-agent-env/batch-gates.sh` is executable and
`bash -n`-validated, but has not been run. It serializes fmt, strict clippy,
full `--no-fail-fast`, and dashboard gates against warmed `target-w4`; preserves
one raw log per command and actual status; records relevant-input pre/post
manifests, toolchain/target provenance, and environment variable names only.
The summary contains statuses and artifact paths, not duplicated raw logs. A
manifest mismatch or any gate failure returns nonzero after all independent
commands complete. It refuses to begin if Cargo or rustc is active or the warmed
target is missing. Run only after todo9/todo12 writers settle, todo30 returns a
source-fix receipt, and target ownership is confirmed. Existing reviews are
read-only and need not finish before source freeze.

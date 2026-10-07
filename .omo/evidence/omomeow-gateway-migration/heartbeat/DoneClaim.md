# DoneClaim: work_items Table Migration and 15-Minute Progress Heartbeat

## Changed Files
- `migrations/0030_work_items.sql`
- `src/work_heartbeat.rs`
- `src/storage/db.rs`
- `src/storage/mod.rs`
- `src/agent/mod.rs`
- `src/lib.rs`
- `src/main.rs`
- `tests/test_work_heartbeat.rs`
- `.omo/evidence/omomeow-gateway-migration/heartbeat/test-output.txt`
- `.omo/evidence/omomeow-gateway-migration/heartbeat/DoneClaim.md`

## Changes
- **Migration Schema (`migrations/0030_work_items.sql`)**:
  - Created `work_items` table with all 9 required columns: `id TEXT PRIMARY KEY`, `title TEXT NOT NULL`, `thread_id TEXT`, `delegate_pane TEXT`, `status TEXT NOT NULL DEFAULT 'in_progress'`, `last_progress_at DATETIME NOT NULL`, `receipt_path TEXT`, `created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP`, `updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP`.
  - Added index `idx_work_items_status_progress` on `(status, last_progress_at)` for high performance scans.
- **Storage Layer (`src/storage/db.rs`, `src/storage/mod.rs`)**:
  - Implemented `WorkItem` model with `id`, `title`, `thread_id`, `delegate_pane`, `status`, `last_progress_at`, `receipt_path`, `created_at`, `updated_at`.
  - Added CRUD and scanning helper functions:
    - `insert_work_item`: Upserts work item records with timestamp defaults.
    - `get_work_item`: Retrieves work item by ID.
    - `mark_work_item_done`: Sets status to `'done'`, records receipt path, and updates timestamp.
    - `update_work_item_progress`: Updates `last_progress_at` and `updated_at`.
    - `get_stale_in_progress_work_items`: Finds active items (`status IN ('in_progress', 'running')`) exceeding progress cutoff.
- **Heartbeat Scanner & Discord Poster (`src/work_heartbeat.rs`)**:
  - Implemented `WorkHeartbeatRunner` with 60s ticker scan loop and `watch::Receiver<bool>` graceful shutdown.
  - 15-minute overdue detection using configurable threshold and clock abstraction.
  - Worker liveness check via `DelegateLivenessChecker` (`HerdrCliLivenessChecker`, `MockLivenessChecker`).
  - Discord REST reporting via `WorkHeartbeatSink` (`SerenityHeartbeatSink` with safe allowed mentions, `RecordingHeartbeatSink`).
  - Exact 2-line progress format when active:
    `⏳ Work item in progress: {title} ({id})`
    `Status: in_progress | Pane: {pane} | Last progress: {last_progress_at}`
  - Single-line warning escalation when worker is dead:
    `⚠️ Worker dead for work item '{title}' ({id}) in delegate pane {pane}`
  - Updates `last_progress_at` in SQLite after posting to throttle repeat notifications for 15 minutes.
- **Runtime Wiring (`src/lib.rs`, `src/main.rs`, `src/agent/mod.rs`)**:
  - Exported `work_heartbeat` module and `WorkHeartbeatConfig`.
  - Wired background heartbeat runner into `src/main.rs` clean startup and shutdown sequence using primary bot HTTP client.
  - Fixed test initializers in `src/main.rs` for `HermesJob` fields (`brief_file`, `catch_up_hours`).

## Verification
- **Unit and Integration Tests (`tests/test_work_heartbeat.rs`)**:
  - `cargo test --test test_work_heartbeat` passed (7/7 tests ok):
    - `migration_0030_creates_work_items_table_with_expected_columns`
    - `heartbeat_15_minute_threshold_triggers_single_post_and_subsequent_step_skips`
    - `heartbeat_dead_worker_escalation_posts_single_line_warning`
    - `heartbeat_post_restart_recovers_without_spamming_stale_items`
    - `heartbeat_discord_rest_payload_contract_matches_serenity_spec`
    - `heartbeat_disabled_by_config_skips_processing`
    - `heartbeat_scans_tasks_with_status_running`
- **Lint & Code Style**:
  - `rustfmt --edition 2021 --check` verified clean.
  - `cargo clippy -- -D warnings` and `cargo clippy --tests -- -D warnings` passed with 0 warnings.

# DoneClaim: omomeow-delegate CLI Tool and Persona Documentation

## Overview
Implemented and verified `bin/omomeow-delegate` CLI tool and agent delegation persona documentation in `docs/AGENTS_DELEGATE.md`, completing todo 8 of the Omomeow gateway migration plan.

## Changed & Created Files
- `bin/omomeow-delegate`: Executable delegation script supporting `--title`, `--repo`, `--brief`, `--thread-id`, `--db`, `--cwd`, and positional arguments. Integrates with Herdr tab creation and inserts work items into the SQLite `work_items` table with default `status = 'in_progress'` and `last_progress_at = CURRENT_TIMESTAMP`. Prints receipt confirmation message:
  `✅ 작업 위임이 접수되었습니다. (ID: <id>, Pane: <pane>)`
- `docs/AGENTS_DELEGATE.md`: Persona documentation defining when and how agents invoke `omomeow-delegate` for long-running / deep background tasks and how the gateway heartbeat monitors them.
- `tests/test_omomeow_delegate.sh`: Integration test script verifying CLI help, argument validation, Herdr/fallback pane resolution, SQLite insertion, and receipt output formatting.
- `.omo/evidence/omomeow-gateway-migration/delegate-cli/DoneClaim.md`: Evidence claim documentation.

## Verification
- Executed `tests/test_omomeow_delegate.sh`:
  - Verified help output `--help` and `Usage:`.
  - Verified failure on missing required arguments.
  - Verified CLI output matches receipt specification: `✅ 작업 위임이 접수되었습니다. (ID: wi_<epoch>_<suffix>, Pane: <pane>)`.
  - Verified SQLite table `work_items` row insertion with `status = 'in_progress'` and non-empty `last_progress_at`.
  - Verified support for `sqlite://` URI strings.
  - All integration tests passed cleanly.

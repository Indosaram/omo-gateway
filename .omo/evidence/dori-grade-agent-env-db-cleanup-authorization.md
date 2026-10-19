# Repo DB cleanup authorization

The owner answered the pending DB cleanup question with:

> DB 정리: 백업 후 행 삭제 (권장)

Authorized scope: back up `/Users/indo/code/project/omon-gateway/omon_gateway.db`, then delete only the unintended incident lane `qa-1`, created at `2026-10-11T06:52:53.346587+00:00`, using all guarded predicates in `task-8-live-db-incident.md`. Preserve migrations, tables, and unrelated rows. Do not stop services.

Executor: `st_01a12b3a`. The executor must validate a consistent backup before deletion, require exactly one deleted row, compare unrelated logical data and migration records against the backup, and report the backup path and integrity checks in `task-8-approved-db-cleanup.md` under the dori-work evidence directory.

Status: dispatched; deletion and backup have not yet been verified. This authorization does not authorize deployment, commits, pushes, or any broader cleanup.

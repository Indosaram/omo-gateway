# AGENTS_DELEGATE.md: Omomeow Agent Delegation Protocol

## Overview
`omomeow-delegate` is the delegation tool for Omomeow / OMON Gateway agents when handling long-running, multi-step, or compute-heavy background tasks.

Instead of blocking interactive chat turns or hitting the 300-second session turn deadline, agents delegate long-running or autonomous engineering tasks into background workspaces / panes managed by Herdr and tracked in OMON Gateway SQLite `work_items`.

---

## When to Delegate
Delegate when:
1. **Long-running execution**: Tasks expected to exceed 60-120 seconds (e.g. extensive test suites, full repo refactoring, compilation, docker builds, model fine-tuning).
2. **Deep autonomous tasks**: Tasks requiring multi-stage research, multi-file code modifications across independent worktrees, or continuous iterations.
3. **Background jobs requiring liveness monitoring**: Work that should be tracked by the gateway's 15-minute progress heartbeat (`WorkHeartbeatRunner`).

Do **NOT** delegate when:
- Answering quick informational questions, simple file edits, or single command inspections that complete within standard turn limits (< 30s).
- Direct user conversation steering.

---

## CLI Usage

### Binary / Script Location
`bin/omomeow-delegate`

### Arguments & Flags
```bash
omomeow-delegate --title <TITLE> --repo <REPO> --brief <BRIEF> [OPTIONS]
```

- `--title <TITLE>` (Required): Short descriptive summary of the work item.
- `--repo <REPO>` (Required): Path or name of the target repository/worktree.
- `--brief <BRIEF>` (Required): Detailed mission prompt and instructions for the worker.
- `--thread-id <ID>` (Optional): Discord channel or thread ID (defaults to `$THREAD_ID` or `$DISCORD_THREAD_ID`).
- `--db <PATH>` (Optional): Path to SQLite database file or `sqlite://` URI (defaults to `$DATABASE_URL` or `omon_gateway.db`).
- `--cwd <PATH>` (Optional): Working directory for the workspace or pane (defaults to `--repo`).

### Example
```bash
bin/omomeow-delegate \
  --title "Migrate voice transcription fallback" \
  --repo "/Users/indo/code/project/omon-gateway-wt/omomeow-migration-w1" \
  --brief "Implement whisper-cli and ffmpeg fallback hook for Discord voice notes" \
  --thread-id "1234567890"
```

### Confirmation Output
On successful delegation, the tool outputs:
```text
✅ 작업 위임이 접수되었습니다. (ID: wi_1728345600_ab12cd, Pane: tab_987654)
```

The receipt reports:
- `ID`: The registered work item ID in SQLite `work_items` table.
- `Pane`: The herdr tab/pane ID or simulated delegate pane (`omomeow-task-<id>`).

---

## Lifecycle & Heartbeat Integration
1. **Creation**: Inserted into `work_items` with status `in_progress` and `last_progress_at = CURRENT_TIMESTAMP`.
2. **Heartbeat Monitoring**: The gateway's `WorkHeartbeatRunner` automatically polls stale items every 60 seconds.
   - If in progress for over 15 minutes, progress updates are posted to Discord.
   - If the Herdr pane or worker process dies, an escalation warning is posted:
     `⚠️ Worker dead for work item '{title}' ({id}) in delegate pane {pane}`.
3. **Completion**: Upon task completion, the worker records the receipt artifact and updates status to `done` (`mark_work_item_done`).

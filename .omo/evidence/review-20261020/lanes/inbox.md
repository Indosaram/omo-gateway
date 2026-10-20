# Lane inbox recovery

C13 FIXED src/inbox/triage.rs:301 — preserved the existing GitHub `[CI failed]` exemption from the 24-hour stale-item shortcut; the source uses this exact prefix even when PR updatedAt is old; test github_ci_failure_not_dropped_as_stale_when_pr_older_than_24h (triage.rs:710) checks a 48-hour-old failure becomes owner while an ordinary old update becomes fyi.
C14 FIXED src/inbox/prefilter.rs:184 — preserved acceptance of source-classified mention room types despite a truncated snippet; existing successor code in src/inbox/sources/slack.rs:59 computes mention from full content with matching SQL bind order; test slack_keeps_mention_room_type_when_snippet_truncated (prefilter.rs:292), with existing source-to-prefilter regression long_channel_message_mentioning_owner_past_snippet_cap_is_collected (sources/slack.rs:337).
C15 FIXED src/inbox/cli.rs:142 — preserved PrePolledSource reuse of the fetched items and cursor for storage and rendering, and added a real collect_github-path regression with an isolated local gh executable that records API invocations; test collect_github_polls_once_and_prints_stored_changes (cli.rs:221) requires two API commands total (one poll), one inserted row, and printed id/snippet equal to the stored row. Existing pre_polled_source_commits_without_repolling also checks cursor persistence.

## Recovery changes and evidence

- This recovery edited only src/inbox/cli.rs and this report. Existing triage, prefilter, and Slack-source successor fixes were read and preserved, not rewritten. The Slack source was read only.
- The new Unix-only regression uses tempfile isolation, GithubSource::with_gh_bin, an in-memory migrated SQLite database, a valid baseline cursor with required updated_at, and no network or timing waits. Reinstating the original double-poll path would produce four recorded API commands instead of two.
- Read the added test hunk, GithubSource poll/diff/notification parsing signatures, collect_all storage flow, and inbox_cursors migration. Full-content mention CASE and all seven SQL placeholders/binds were inspected.
- No builds, tests, formatting commands, commits, pushes, deployments, engine changes, or external publishing/calendar actions were run in this recovery. Active QA/card-render/evidence workers were not contacted or changed.
- Automatic fresh LSP diagnostics timed out after 3000 ms; this is not a compiler result. Compile, test, lint, and runtime verification remain with the lead's integrated run. No passing checks from earlier workers are claimed here.
- No outstanding implementation blocker was found for these three defects. The newly added regression is written but unrun; its cfg(unix) matches the executable fixture platform requirement.

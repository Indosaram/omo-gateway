# CONFIRMED-r1 (lead falsification, in progress)

Rubric: P0 = reachable trigger -> data loss / misdelivery / security breach / crash / broken build. P1 = reachable trigger -> wrong user-visible behavior, hang, or accumulating leak.

## agent-backend (st_01a13aa2, claude-opus-5-5)
- [P1 CONFIRMED] omo_backend.rs:1214-1230 + 1857-1861 + 2020-2021. The inactivity clock (`last_activity_at`) is renewed only at :1295, i.e. on the start ack or on `frame_is_turn_progress` frames. An interactive turn whose single tool emits nothing for 180s (a subagent task, a long silent command) hits "timeout during turn streaming: no activity from server". `is_backend_stall` matches that string, so `force_restart_owned_daemon` SIGKILLs the shared app-server and `execute_turn` runs the same prompt again. That breaks the NEVER-resubmit invariant stated at :1994-1996: tool side effects run twice, and other sessions' turns on the daemon are cut off. Regression from commit 9fd6a6c (self-heal).
  Fix direction: count open items (item/started without item/completed). Skip both the inactivity abort and the stall restart while any item is open, and retry only when no tool call happened.
- P2 x3 recorded (cancel before ack returns cancelled while the remote turn runs; shared obl:turn:{stream_id} obligation across pre-ack relays; the settings/update ack is bounded only by the turn deadline).
  FIX APPLIED (lead, omo_backend.rs): `open_tool_ids` tracks tool items started but not completed. The inactivity abort and its wake timeout are skipped while one is open. After tool work, an inactivity timeout returns a distinct message ("server went silent after tool calls") that `is_backend_stall` does not match, so there is no daemon kill and no re-run. Unit test extended (only_missing_turn_events_count_as_a_backend_stall). The total turn deadline still bounds a hung tool.

## lanes-signals (st_01a13aaa, claude-opus-5-5)
- [P1 CONFIRMED] settle.rs:294/322/359. `git diff --cached --name-only` lists repo-root paths, but the new side was read via cwd.join(path). For a lane whose cwd is a repo subdirectory, every modified file reads as deleted and every added file vanishes, so jev rejects correct work forever. git show/cat-file rev:path already resolve from the root.
  FIX APPLIED: resolve rev-parse --show-toplevel once and read the new side from there. Test added: lane_in_a_subdirectory_still_sees_its_added_file.

## cron-scheduler (st_01a13aa4/st_01a13aca, claude-opus-5-5)
- [P1 CONFIRMED] scheduler.rs:2794 normalize_cron_expression only prepends seconds. cron 0.15 numbers weekdays 1-7 with Sun=1 (verified in the crate source: days_of_week.rs inclusive_min 1 and sun=>1; schedule.rs number_from_sunday). The dashboard picker (web/src/lib/schedule.ts, Sunday=0) and Hermes croniter emit Unix numbering. Result: `1-5` runs Sun-Thu, and `0` is rejected.
  Live DB (read-only): omon-maho-weekly-v3 `0 0 * * 1` (enabled) fires Sunday now and will fire Monday after the fix. omon-maho-bot-weekly-v2 `0 9 * * 7` (disabled) moves from Sat to Sun.
  FIX APPLIED: unix_day_of_week_to_crate converts the 5th field of 5-field expressions. Test added: five_field_weekdays_use_unix_numbering.

## tools-exec (R1b st_01a13acb, claude-opus-5-5)
- 0 P0/P1. The lane capped all 4 findings at P2 on reachability, and the lead verified that claim: ast-grep `$T.execute_with_context($$$)` across src finds only test call sites plus tools/mod.rs:122/188 (the registry forwarding itself), and main.rs:937-987 builds the registry but no agent loop calls execute for terminal/file/web/mcp/browser. The omo app-server runs its own tools.
- P2 x4 recorded: OMON_OMO_AUTH_TOKEN alias not stripped (terminal.rs:716); an escaped grandchild holds the pipes until timeout (terminal.rs:440); web_fetch default of 100k vs the documented 8k (web.rs:131); MCP stdio write outside the timeout (mcp.rs:138).

## security (R1b st_01a13acd, claude-opus-5-5)
- [P1 CONFIRMED] hardline.rs:25. The system-dir alternatives (/etc, /etc/\*) are followed by a tail that does not accept '/', so rm -rf /etc/, //etc, /./etc and /var/. escape the unconditional hardline (under /yolo or ApprovalPolicy::Never they run). Same class as the rm -rf /"" bypass graded P1 on 10-13.
  FIX APPLIED: hardline_system_dirs is now (?:/+\.?)*/+(?:home|root|etc|usr|var|bin|sbin|boot|lib|System)(?:/+\.?)*(?:/\*)?. Lead probe of the same pattern in JS: 9/9 blocked spellings hit, 7/7 subpaths/lookalikes (/etc/nginx/sites, /usr/local/tmp, /tmp/etc, ./etc, /etcetera, /var/log/app.log, /home/indo/build) miss. Test added: system_directory_spellings_are_all_hardline.
- P2 x2 recorded (cron prompt scan runs the shell hardline parser on prose -> false rejects; approval_display misses -u user:pass).

## discord-adapter-b (R1b st_01a13ac5, claude-opus-5-5)
- [P1 CONFIRMED] adapter.rs:3370/3445. stream() lets the 60s dead-target probe through, but on success it never calls clear_for_bot (only the dispatch arms do). Production final replies use Stream, so a recovered channel stays dead: other replies within each 60s window fail with "dead target short-circuited", and sweep_failed_for_runtime does not retry that text, so they are lost.
  FIX APPLIED: after throttler.update(..) succeeds, call self.dead_targets.clear_for_bot(&identity, channel_id). clear_for_bot only writes to the DB when an entry was really removed, so this adds no per-send overhead.
- P2 x1 recorded (approval content plus mentions can exceed 2000 chars).

## agent-runtime (R1b st_01a13ac9, claude-opus-5-5)
- [P1 CONFIRMED] omo_daemon.rs:303-313 + 589. The pidfile stored only the daemon pid, and reclaim killed any live app-server --listen <url> it pointed to. A standalone dashboard (no runtime lock, dashboard_runtime.rs:147 -> ensure -> reclaim_leftover_daemon) therefore SIGTERM/SIGKILLs the running gateway's daemon. The gateway watcher then saw the port served and returned for good, so after the dashboard exited the gateway had no daemon and no supervisor.
  FIX APPLIED: (a) the pidfile records "<daemon_pid> <owner_pid>"; leftover_daemon_pid refuses to reclaim while a different owner pid is alive. Legacy single-pid files keep the old behaviour. (b) The watcher no longer returns when another daemon serves the port: it waits while readyz answers, then loops back and respawns its own.
- [P1 DECLINED as product decision] omo_daemon.rs:685: a 45s no-output stall force-kills the shared daemon, collateral to other turns. This is the self-heal the user explicitly required (memory 2026-10-14: on app-server non-response, the gateway SIGKILLs and restarts its owned app-server with a 2 min cooldown and retries). A readyz pre-check would defeat it, because the wedged app-server still answers readyz while dropping turn events. The tool-call resubmission part is covered by F1 (silence after tool work no longer counts as a stall).
- P2 x1 recorded (log rotation only at spawn).

## multiplexer (st_01a13aa1, claude-opus-5-5)
- [P1 CONFIRMED] actor.rs shutdown drain (was :869). A queued ActorCommand::Event has no transcript row (persist_inbound runs only at dequeue), and the live path already advanced the backfill cursor (adapter.rs:1690, right after route() accepted it). At shutdown it is only completed as failed. Restart recovery reads only the last transcript row (db.rs:490) and backfill starts after the cursor, so the message is lost for good. Normal deploy restarts trigger it whenever a message is queued behind a running turn (for example with an attachment, which is not steered).
  FIX APPLIED: rewind_backfill_cursor() in the shutdown drain moves discord_bot_cursors.last_message_id back to message_id-1 (only when the stored cursor is >= the message id), so the startup backfill re-fetches it; the ledger row is failed and reclaimable (service.rs ON CONFLICT ... WHERE status='failed').
- P2 x1 recorded (restart-loop guard keeps future-dated boots after a clock step-back).

## main-runtime-a (R1b st_01a13ac6, claude-opus-5-5)
- [P1 CONFIRMED] main.rs:803-944: the headless (OMON_DISCORD_ENABLED=false) branch builds OmoBackendConfig and backends but never calls OmoDaemonSupervisor::ensure (the Discord path does at :995/:1123, the dashboard at dashboard_runtime.rs:147). With the default URL and no external daemon, every lead, cron and lane turn fails to connect.
  FIX APPLIED: the headless branch holds _daemon_supervisor = ensure(&backend_config), plus a cron supervisor when the cron URL differs, mirroring the Discord path. ensure returns None when an external daemon serves, so co-located setups are unchanged.
- P2 x2 recorded (OMON_OWNER_ID only partly applied; approval_timeout_secs + 5 overflow).

## main-runtime-b (R1b st_01a13ac7, claude-sonnet-5-5)
- 0 P0/P1. P2 x3 recorded: the runtime lock hash does not normalize ./ or symlinked DB paths (entry.rs:346); approval_timeout_secs + 5 overflow (duplicate of main-runtime-a); the cron_script_timeout_reaps_descendants test uses a fixed sleep and an unchecked pid-file read (main.rs:2722, flaky on a loaded host).

## discord-adapter-a (R1b st_01a13ac4, claude-opus-5-5)
- [P1 CONFIRMED] adapter.rs:919 (pre-edit). The owner's "제안" listing joins every pending proposal into one Discord message with no length bound. Over 2000 chars Discord rejects it, presented() never runs, and since approve() matches only by message_id, no proposal can be approved until the backlog shrinks.
  FIX APPLIED: proposals are packed into messages of at most DISCORD_MESSAGE_LIMIT minus the footer, split only between proposals (a single oversized entry is truncated with an ellipsis). Each proposal is presented() with the id of the message that shows it, and every chunk carries the reply instruction.
- P2 x2 recorded (debounce merges different authors in shared guild sessions; concurrent Ready backfills can advance the cursor past a still-running turn).

## discord-commands (st_01a13a9c, claude-sonnet-5-5)
- [P1 CONFIRMED] commands.rs:1247. `/thread` builds CreateThread with no kind. serenity 0.12.5 documents that standalone threads then default to PrivateThread, so the invoking user cannot open the thread the bot reports, and without Create Private Threads permission the command fails with 403.
  FIX APPLIED: `.kind(serenity::ChannelType::PublicThread)`; CreateThread::kind verified in the serenity-0.12.5 source (create_thread.rs:85).
- P2 x3 recorded:
  - /steer and /deny echo unbounded text after the side effect (the reply over 2000 chars fails and the user retries);
  - yolo_toggle does a full state_json read-modify-write that can clobber omo_thread_id;
  - a button without a rebuildable row stays clickable after its action ran.

## discord-auth (st_01a13a9d, claude-sonnet-5-5)
- 0 P0/P1. P2 recorded: the pairing failed-attempt counter never ages out below 5, so scattered typos eventually cause a 15-minute platform lockout (pairing.rs:183). The lane confirmed that the 10-13 P1s in scope (table placeholder, pipe splitting, typing abort, pairing attempt bump, broad Always key) are fixed.

## lanes-runner (R1b st_01a13ace, claude-opus-5-5)
- [P1 CONFIRMED] runner.rs:1337. tokio::time::interval ticks immediately, and last_activity_at is written only by capture, guard launch, and adopt, never at turn start. A lane that sat blocked or idle for more than 20 minutes (or a gateway that was down that long) therefore gets an immediate quiet_generation hit at the start of the next turn: a false 'stale' lane event to the lead plus a 'stale:' post in its Discord thread.
  FIX APPLIED:
  - Before spawning the turn, `UPDATE lanes SET last_activity_at = now, stale_posted_at = NULL WHERE key = ? AND status='working'` (the runner uses SystemClock, so Utc::now() matches).
  - quiet_check uses interval_at(now+60s, 60s), so the first check waits one period.
  - The existing stale tests (tests/test_lane_events.rs:765/809) call LaneActivity::quiet directly and are unaffected.
- P2 recorded: the dispatcher runs cron remediation (including a full job rerun) inline and delays other leads' events.

## web-deploy (st_01a13af6, claude-opus-5-5; the R1b copy died, relaunched)
- Citations resolved: Dockerfile:18, docker-compose.yml:8,11-25, deploy.sh:97, watchdog:81 (all match).
- P1 Docker crash-loop: CONFIRMED.
  - The runtime image has no omo CLI, compose leaves autospawn on, ensure() spawns 'omo' and fails with ENOENT, then '?' exits at main.rs:1003, and 'restart: unless-stopped' turns it into a loop.
  - README documents 'docker compose up -d'.
  - FIX F12: compose sets OMON_OMO_AUTOSPAWN=off, OMON_OMO_APPSERVER_URL (default ws://host.docker.internal:19742), OMON_OMO_APPSERVER_AUTH_TOKEN and extra_hosts host-gateway; the README explains the host daemon. With autospawn off, ensure() returns Ok(None) at omo_daemon.rs:480.
- P1 deploy.sh rollback writes the signed binary in place: CONFIRMED (plausible, documented macOS behaviour; cheap fix).
  - FIX F13: copy to $BIN.rollback, then mv -f (atomic rename).
- P2 watchdog restarts on the first failed probe: recorded only (alerting already has a FAIL_LIMIT; this is a policy/tuning item).

## lanes-surface (st_01a13aab, claude-sonnet-5-5; R1b)
- Citations resolved: guard.rs:680, cli.rs:413, guard.rs:431, posts.rs:417 (all match).
- P1 queued lane stranded after a transient launch error: CONFIRMED.
  - The skip is keyed on any launch_failed event newer than updated_at. retry_queued never changes updated_at on failure, so the lane is skipped forever.
  - FIX F14: the skip only applies within LAUNCH_RETRY_BACKOFF (10 min) of the failure, via julianday(created_at) > julianday('now') - backoff_days. A failure therefore re-posts at most once per 10 min, which keeps the no-repost-spam intent.
  - Test failed_launch_is_retried_after_the_backoff (channel 998 now 404 in the mock): skipped within the backoff, retried after the event is aged past it.
- P2 recorded:
  - a lane start that fails BEGIN/commit leaks a thread;
  - the host guard needs macOS sysctl/memory_pressure (Linux/Docker);
  - a persistent edit error on an archived thread repeats every loop.

## cron-store-exec (st_01a13aa5, claude-sonnet-5-5; R1b)
- Citations resolved: 7/7 (guard.rs:10, remediate.rs:215, executor.rs:658/883/489, report.rs:235/86).
- 0 P0/P1. All 7 findings are P2 and recorded.
- remediate.rs:215 checked against the live DB (read-only): the production publish job ids are bare (omon-content-intel-publish-ready, omon-playstore-daily-publish, omon-vpn-mall-daily-publish-v2), so the hermes-prefix bypass does not apply today. Kept at P2.

## inbox-migrate-voice (st_01a13aae, claude-sonnet-5-5; R1b)
- Citations resolved: 4/4 (migrate/mod.rs:187, inbox/triage.rs:290, inbox/mod.rs:249, inbox/set.rs:62), read directly.
- 0 P0/P1. All 4 findings are P2 and recorded:
  - the migrate preflight ignores the profile selection;
  - a writer failure causes triage to re-judge items (bounded by the jev cap);
  - a mail refresh leaves a stale pending proposal;
  - `inbox set approved` exits 0 on AlreadyClaimed.

## tools-context (st_01a13acc, claude-sonnet-5-5; R1b)
- Citations resolved: 5/5 (tools/mod.rs:162, tools/cron.rs:292/595/366/701).
- P1 cron approval prompt truncates the shell text to 300 chars: DOWNGRADED to P2 (unreachable in production), same reachability ruling as tools-exec.
  - Lead check: ast-grep `$T.execute_with_context($$$)` hits only tests plus the registry forwarding at tools/mod.rs:122/188.
  - The only production registry.execute calls are discord/commands.rs:573/607/632 (skills list/search/read) and :743 (cron list). None of them reaches CronTool add/update or requires_approval.
  - Agent turns run inside the omo app-server with its own tools, so no production path can submit a cron add for approval.
  - Re-grade to P1 if CronTool is ever exposed to the agent.
- P2 x4 recorded:
  - deliver=origin without an origin delivers nowhere;
  - update uses the UTC next_run instead of the job timezone;
  - delete leaves incidents/notepads behind;
  - native jobs ignore context_from/skills.

## dashboard (st_01a13ac8, claude-sonnet-5-5; R1b)
- Citations resolved: 5/5 (dashboard.rs:1216/1654/2010/78/144).
- P1 a session DELETE cascade-deletes cron jobs: CONFIRMED.
  - migrations/0001:48 has cron_jobs.session_key REFERENCES sessions ON DELETE CASCADE; the live DB schema is identical, and 1 of 16 live jobs is session-bound.
  - The tool and digest jobs bind to sessions. The dashboard delete removed them silently, with no warning in the confirm text.
  - FIX F15: delete_session counts bound cron_jobs first and returns 409 CONFLICT "session has N cron job(s); delete or move them first".
  - Test deleting_a_session_with_cron_jobs_is_refused.
  - Not covered by the 10-13 P2 "deleted sessions stay in message_search" (a different item).
- P2 x4 recorded:
  - a bad session_key on cron create returns 500;
  - the skill description shows the frontmatter name line;
  - an empty DASHBOARD_PORT enables the dashboard (loopback only);
  - a bracketed IPv6 host passes validation but cannot bind.

## storage-ledger (R1b st_01a13ad0, claude-sonnet-5-5)
- NO FINDINGS. There are no citations to resolve.

## sessions-judge (R1b st_01a13acf, claude-sonnet-5-5)
- Citations resolved: 5/5 (sessions/external.rs:488/463/111, judge/mod.rs:44/404).
- P1 remote done-check rejects on any unpushed local branch, deleted worktree or old linked worktree: CONFIRMED.
  - The remote python script used `git log --branches --not --remotes` and took every `worktree list` entry, with no missing-path skip and no creation-time filter.
  - The local settle.rs:550-578 uses `log HEAD`, skips missing paths, and filters with include_worktree.
  - Not a duplicate of the 10-13 external P1s (20s timeout, NotRunning, multi-line goal).
  - The live DB has 0 external lanes today, but the path is shipped (`sessions adopt`), so it is graded as written.
  - FIX F16:
    - the script now uses `git log HEAD --not --remotes`;
    - include() keeps cwd/subpaths, drops missing paths, and keeps a linked worktree only if st_birthtime/ctime >= lane start;
    - the lane start is passed as argv[2] (lane.created_at in epoch seconds; empty -> no filter).
  - Lead check: ran the extracted script in a temp repo.
    - an unrelated unpushed branch -> exit 0;
    - an unpushed HEAD -> exit 1 'unpushed';
    - a deleted worktree -> exit 0.
    - The temp dir was removed.
  - Existing remote_checks_with tests use fake runners that do not inspect argv, so they are unaffected.
- P2 fixed anyway: adopt now stores the trimmed lead (`let lead = lead.trim();`), so the validated value is the stored value.
- P2 recorded:
  - the remote file signal lacks [n] json-path support and a case-insensitive sha;
  - scrub_for_external misses github_pat/JWT/PEM;
  - judge fallback persists noise rows (bounded by the fallback cap).

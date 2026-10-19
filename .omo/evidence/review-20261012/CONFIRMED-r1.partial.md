# CONFIRMED R1 (partial: 3/23 lanes) — lead-verified at source

## P1 (confirmed)
1. discord-adapter: thread archive/delete session key mismatch — src/discord/adapter.rs:1395 (vs inbound key adapter.rs:1977-1987: guild+parent+thread+bot). stop() hits no actor; result discarded.
2. discord-adapter: UPDATE sessions SET metadata (nonexistent column) — src/discord/adapter.rs:1405/1412/1432; grep migrations: no 'metadata' column; error discarded with let _.
3. dashboard: combined-mode trigger_cron_job uses WebDashboardDispatcher (always Ok, dashboard.rs:492) -> scheduler.rs:1988-2001 marks delivered; bypasses dispatcher.is_none() guard scheduler.rs:1790. Discord cron output silently lost — src/dashboard_runtime.rs:171.
4. dashboard: combined-mode separate SmartApprovalGuard — src/dashboard_runtime.rs:56 vs src/main.rs:772. Discord approvals invisible/unresolvable in dashboard.
5. dashboard: combined-mode dashboard chat drives second multiplexer on Discord session — src/dashboard_runtime.rs:131 (source-confirmed construction; concurrency impact by reasoning).
6. tools-exec: terminal read loop waits pipe EOF before child.wait — src/tools/terminal.rs:398 (structure confirmed; backgrounded grandchild holds pipes -> full-timeout hang).
7. tools-exec: browser SSRF guard checks only initial URL — src/tools/browser.rs:247 (redirect/rebinding). keep P1.

## P2 (recorded)
- browser owned_tabs shared across sessions — browser.rs:213 (confirmed).
- file search aborts on one unreadable entry — file.rs:333/338 (confirmed: map_err ? on symlink_metadata/read_dir/entry).
- text /stop not detected after topic/reply prefix — adapter.rs:1198.
- steer success + ledger error -> re-enqueue duplicate — adapter.rs:1466 (confirmed: record_incoming_as(...).await? after Ok(true); caller enqueues on Err at 1214-1221). Narrow (DB error) -> P2.
- POST /api/cron/jobs existing id overwrites — dashboard.rs:1510.

## Added (agent-backend, multiplexer) — lead-verified
P1:
8. multiplexer: /model mid-turn reverted — actor.rs:542 'self.context = turn_context' overwrites SetModel at actor.rs:484 (turn_context cloned at :364). CONFIRMED.
9. multiplexer: /reset mid-turn restores omo_thread_id — actor.rs:533-537 unconditional copy after Reset arm cleared it (:488-495). CONFIRMED.
10. multiplexer: queued plain Event dropped at shutdown drain — actor.rs:788 '_ => {}' (only EventWithAck/EventIfIdle answered). CONFIRMED.
11. agent-backend: mid-turn status emitted is_final=false + messages_delivered+=1 + full_content.clear() (omo_backend.rs:1452-1466); adapter stream() drops non-final (adapter.rs:3026 completed_stream_content); final placeholder suppressed when messages_delivered>0 (omo_backend.rs:1638). CONFIRMED pending completed_stream_content semantics check.
P2: reset thread_ids cache (omo_backend.rs:280); active_turns leak on early returns (:1123); model update rejected no fallback (:758); deadline drain intent-gate shadow (:1033) + settings.json overwrite (:457/470).

## Added (migrate) — lead-verified
12. P1 migrate: gateway-down before cutover validation — src/migrate/mod.rs:186 then :191 (confirmed ordering; no preflight).
13. P1 migrate: normalize_job_payload ignores neither _omon_discord_home_* (injected store.rs:932/936) nor schedule.timezone -> cutover refuses — src/migrate/cron_cutover.rs:64 (confirmed ignore-list).
14. P1 (security) migrate: profile .env value used when root lacks key for DISCORD_ALLOWED_*/ALLOW_ALL — src/migrate/config_import.rs:462 (confirmed root_value.or(profile_value)). Override-of-YAML part not re-checked; fallback widening confirmed.

## Added (lanes-surface, agent-runtime) — lead-verified
15. P1 storage/lanes.rs:202 request_close upsert WHERE force=excluded.force -> forced retry of failed intent is a no-op; returns old id. CONFIRMED.
16. P1 lanes/posts.rs:244 refresh_lane_status(...).await? in drain aborts whole lane service. CONFIRMED.
17. P1 lanes/posts.rs:309 Sending|Uncertain receipt returns early forever; no reconciliation. CONFIRMED.
18. P1 agent/omo_daemon.rs:162 autospawn hardcodes --ws-auth off regardless of configured token. CONFIRMED (line).
19. P1 agent/llm.rs:167 reqwest client without connect/read timeout. CONFIRMED.
20. P1 agent/llm.rs:647 parse_stream_line drops in-stream error frames -> partial treated as success. CONFIRMED.
P2: posts.rs:193 pending forever w/o thread; cli.rs:381 thread before insert; cli.rs:141 cap race; omo_config.rs:252 zero timeouts; agent_workspace.rs:57 case collision.

## Added (web-frontend) — lead-verified
21. P1 web/src/App.tsx:586 onmessage ignores event type send_message (agent error/direct reply) -> isSending stuck. CONFIRMED (handler branches read).
22. P1 web/src/App.tsx:513 sessionMessages(id) w/o paging -> only first 50 (server default per_page 50 per lane; dashboard.rs list_messages). CONFIRMED call; server default not re-read -> verify in fix.
23. P1 web/src/App.tsx:571 socket closure uses captured currentSessionId -> cross-session render. CONFIRMED.
24. P1 web/src/pages/BotsPage.tsx:121 + src/dashboard.rs:1986-1991 update_bot uses payload.X.or(existing.X) so null cannot clear. CONFIRMED server side.
P2 (11): BotsPage create overwrites, ws onerror duplicate turn, transcript wiped on error, sidebar stale, security page Promise.all, approval card session_id, overview model/disk, cron running label, logs dup, search race, remote img beacon.

## Added (runtime-main) — lead-verified
- entry.rs:201 attached dashboard runtime: duplicate root cause of #3/#4/#5 (already in fix-dashboard-combined). Adds: /stop marks session suspended on shared DB without stopping live turn -> include in dashboard fix follow-up.
25. P1 src/main.rs:448 clear_session_resume_pending before multiplexer.route; route Err only logged (:471) -> marker lost. CONFIRMED.
26. P2->P1? src/main.rs:1158 'if let Ok(Err(err))' ignores JoinError (panic) -> discord_task_failed false -> exit 0. CONFIRMED code; impact depends on supervisor policy -> keep P1 (restart loss of gateway).
P2: main.rs:336 startup recovery ignores paired users; main.rs:2277 parse_u64_list drops invalid channel ids -> unrestricted.

## Added (discord-commands, lanes-runner) — lead-verified
27. P1 commands.rs:1272 yolo_toggle writes DB only; grep 'yolo' src/multiplexer = 0 matches -> actor flush overwrites. CONFIRMED.
28. P1 commands.rs:1034 /retry DELETE then route(...)? (:1042) -> drain error loses exchange. CONFIRMED.
29. P1 commands.rs:1083 compress drops Err chunks, accepts partial acc, then replace_history deletes. CONFIRMED.
30. P1 lanes/settle.rs:93/:99 remote_checks/settle_lane '?' -> settle_due Err -> runner.rs:1035 'return Err(error)' kills lane service. CONFIRMED.
31. P1(downgrade to P2? kept P1) buttons.rs:395 in-flight button state lost on restart (process-local IN_FLIGHT). Restart-only trigger -> regrade P2.
P2: commands.rs:601/1197 route result discarded; :123 thread owner persist fire-and-forget; :644 memory list unwrap_or_default; :560 skill search >2000; settle.rs:485 --branches scan; runner.rs:846 errored owner reply never retried.

## Added (cron-scheduler, runtime-aux, security) — lead-verified
32. P1 cron/scheduler.rs:1245 TimeDelta::hours(catch_up_hours as i64) panics on huge value -> scheduler loop dies. CONFIRMED (unchecked).
33. P1 cron/scheduler.rs:1545 complete_success CAS on stale payload_json, rows_affected unchecked -> re-run/duplicate delivery. CONFIRMED.
34. P1 mirror.rs:147 fallback query without thread filter -> cron output mirrored into unrelated thread transcript. CONFIRMED.
35. P1 voice/pipeline.rs:141 reqwest::Client::new() (no timeout) for STT. CONFIRMED.
36. P1 security/hardline.rs:24 rm_prefix only flags before protected path -> 'rm -rf build ~' bypasses hardline. CONFIRMED regex shape.
P2: scheduler.rs:1704/964 tz ignored; :1200 repeat limit manual; :153 LIKE suffix; readiness.rs:240 wrong url; usage.rs:232 tz; lanes_block.rs:172 updated_at; voice_transcription.rs:22 ffmpeg; normalize.rs:1023 budget exhaustion silent; normalize.rs:677 apostrophe; approval_display.rs:12 token scheme leak; dangerous.rs:252 [^;|&\n] class.

## Added (cron-store-exec, tools-context) — lead-verified
37. P1 cron/store.rs:599,:640 trimmed[..8] on len>=8 non-ASCII -> char-boundary panic. CONFIRMED.
38. P1 cron/remediate.rs:321 inbox_decisions ON CONFLICT DO UPDATE keeps stale status (done/dismissed) -> owner never re-notified. CONFIRMED.
39. P1 tools/cron.rs:593 register_with_id upsert over existing/imported id. CONFIRMED shape.
40. P1 tools/message_context.rs:389 default target = session.channel_id (parent in thread sessions). CONFIRMED.
41. P1? cron/executor.rs:357 ack before delivery; :166 monitor snapshot committed before run — plausible, fix node must verify (NOT-A-BUG allowed).
42. P1? tools/cron.rs:394 update skips imported read-only guard; :542 script no approval; :192 thread deliver format; tools/web.rs:169 SSRF — fix node must verify each first.
Regraded: web.rs:77 DDG markup -> P2 (external markup drift, not code defect provable here).
P2: executor.rs:234; store.rs:1074; report.rs:200,170,269; remediate.rs:569,474; web.rs:293.

## Added (lanes-signals) — lead-verified
43. P1 lanes/guard.rs:631 Discord create_thread awaited inside open tx (begin :601) on max_connections(1) pool (db.rs:182). CONFIRMED.
44. P1 lanes/guard.rs:639 retry_queued never calls bind_lane_thread for created thread. CONFIRMED (no bind in loop).
45. P1 lanes/guard.rs:638 per-lane '?' aborts whole pass; starter message re-posted each tick. CONFIRMED.
46. P1 lanes/scrub.rs:37 PRIVATE_KEY_BLOCK_RE requires a word before ' PRIVATE KEY' -> PKCS#8 '-----BEGIN PRIVATE KEY-----' not redacted. CONFIRMED regex.
P2: guard.rs:600 newest-first; signals.rs:1181 unbounded output; signals.rs:1127 no process group; kibitzer.rs:238 consumed before judge; scrub.rs:8 URL paths.

## Added (storage-db) — lead-verified
- migrations/0041 FK failure: REPRODUCED (sqlite3: temp DB with inbox_decisions child row -> 'FOREIGN KEY constraint failed' at COMMIT). BUT production DB copy shows _sqlx_migrations 41 success=1 (already applied), and editing an applied migration changes its checksum -> sqlx refuses to start ('previously applied but modified'). Regraded P2 / DO-NOT-EDIT: affects only fresh upgrades of pre-0041 DBs with triaged inbox rows; a fix must be a NEW forward migration or documented upgrade note, never an edit of 0041. Recorded, not dispatched.
- P2 models/events.rs:69 '[\\s*_~`]' in raw string (confirmed) -> 'No responses' swallowed as silence.
P2: ledger/service.rs:541 prune never called; message_search.rs:225 empty author; message_search.rs:221 thread under parent channel.

## Added (deploy-scripts) — lead-verified
47. P0 Dockerfile:4-7 does not COPY tests/ while Cargo.toml declares 15 [[test]] with explicit paths -> manifest load fails. CONFIRMED (15 [[test]] blocks, no COPY tests).
48. P0 Dockerfile:1 rust:1.85 < time 0.3.55 rust-version 1.88.0 (local registry Cargo.toml) with --locked. CONFIRMED.
49. P0 Dockerfile builder lacks cmake/pkg-config/libopus-dev for audiopus_sys. PLAUSIBLE (not built); include in Docker fix.
50. P0->P1 docker-compose.yml:27-28 bind mounts ./data ./workspace created root-owned on Linux vs USER omon. CONFIRMED config; regraded P1 (Linux-host + fresh clone only).
51. P1 Dockerfile runtime has no fonts -> table PNG blank. CONFIRMED (only ca-certificates libopus0).
Regraded P2 (one-shot historical cutover scripts, migration already executed in prod; only harmful on manual re-run): seed_omomeow_crons.sql next_run_at, cutover_phase1 ordering, cutover/rollback_phase3 token sed + pkill, verify_phase2 '|| true'. bin/omomeow-delegate brief overwrite P2.

## Added (inbox, sessions-judge) — lead-verified against source
52. P1 inbox/watchers.rs:30 on_gateway_message awaits bounded send; called inline from actor persist_inbound/adapter steer. CONFIRMED (send().await, try_ variant unused).
53. P1 inbox/sources/mod.rs:46 run_json Command without kill_on_drop under 60s timeout. CONFIRMED.
54. P1 sessions/external.rs:394/415 control.output(...).map_err(invalid)? propagates; runner.rs:1035 Ok(Err) => return Err stops lane service. CONFIRMED.
55. P1 sessions/external.rs:304-307 send_goal/read_new/send_pending '?' -> runner fail(dead) on one transient error. CONFIRMED.
56. P1 sessions/external.rs:151 BEGIN IMMEDIATE held across control.send(...).await on max_connections(1) pool. CONFIRMED.
57. P1 sessions/external.rs:115-123 release only clears external_target/released_at, lane status stays working -> slot leak. CONFIRMED.
58. P1 judge/mod.rs:47 scrub_for_external caps 4000 chars, applied to diff at :1439. CONFIRMED.
P2 recorded: judge:762 RFC3339 vs space ts; external.rs:225 generation 0; judge:862 alerter None; external.rs:66 lead not validated; inbox digest.rs:87 401 substring.

## Fix verification
- fix-multiplexer: VERIFIED (diff actor.rs only; RED 3 fail -> GREEN 9/9 in /tmp copy; repo rerun pending omo_daemon.rs compile).
- fix-deploy: VERIFIED diff in scope (Dockerfile, compose, Cargo.toml rust-version, README docker note, new tests/test_fix_fix_deploy.rs). RED 0/5 -> GREEN 5/5 (rustc standalone). Note: #47 regraded P0->P1: cargo 1.92 'cargo build --bin' does not fail on missing [[test]] paths; fix still correct. No docker available -> image build unverified (stated).

- fix-agent-runtime: VERIFIED. diff only in src/agent/{agent_workspace,llm,omo_config,omo_daemon}.rs (its scope). RED 5 fail (+d1 compile-RED) -> GREEN 8/8 in REPO (target/, exit 0). omo_daemon token-file mode fixes --ws-auth off; file 0600. Compile blocker in omo_daemon.rs now resolved (lib builds in repo).
- fix-discord-adapter: VERIFIED (report + green log read). Scope adapter.rs only per report (git diff scope re-check pending at batch verify). RED: thread-delete stop Elapsed timeout; state_json metadata Null vs true; ledger failure Err after steer. GREEN 5/5 in repo target/ (exit ok). Defect 3 was P2 (steer ledger) - also fixed.

- fix-main: report read. 2 P1 (resume marker on route Err; JoinError counted as discord failure) + 2 P2. RED 0/4 -> GREEN 4/4 (cargo test --bin omo-gateway fix_main). Scope src/main.rs only. exit(1) path not live-tested (stated).
- fix-commands / fix-settle / fix-migrate: DAG marked COMPLETED with NO report and GREEN logs containing only "Blocking waiting for file lock" -> NOT verified. Revived fix-commands and fix-migrate via send; fix-settle revive refused (task capacity full) -> retry after capacity frees.

- fix-inbox: VERIFIED. Scope src/inbox/watchers.rs + sources/mod.rs only. #52 try_send (Full=Ok, signature kept), progress try_send; #53 kill_on_drop. RED 0/3 (incl. 10s waitpid bound) -> GREEN 3/3.

- fix-settle: VERIFIED. settle.rs (+28/-?) + tests/test_fix_fix_settle.rs. P1 per-lane error -> not_done & continue. RED 0/2 -> GREEN 2/2. settle_lane error branch code-read only (OnceLock).
- fix-commands: VERIFIED. commands.rs only. P1 /retry restore + drain precheck; P1 /compress aborts on stream errors; P2s. RED 0/4 -> GREEN 17 pass, 1 ignored. P1 YOLO-to-live-actor = BLOCKED-SCOPE (needs ActorCommand::SetYolo in multiplexer) -> FOLLOW-UP node.
- fix-dashboard-combined: VERIFIED. dashboard.rs + dashboard_runtime.rs (fmt churn +26/-32 noted). 3 P1 fixed (fail-closed web dispatcher; 409 for messenger chat/stop; detached approvals warning/409) + P2 dup cron 409. RED 0/4 -> GREEN test_review_dashboard 78/78. True approval sharing = BLOCKED-SCOPE (main.rs) -> FOLLOW-UP node.
- settle follow-up: external.rs remote_checks script uses 'git log --branches' -> forwarded to fix-sessions-judge (owner).

- fix-security: VERIFIED. src/security/{hardline,dangerous,approval_display,normalize}.rs + tests/test_fix_fix_security.rs. P1 rm-prefix hardline bypass fixed; 4 P2. RED 0/5 -> GREEN 5/5; lib security 26/26.
- fix-omo-backend: VERIFIED. omo_backend.rs only (+tests/test_fix_fix_omo_backend.rs). P1 mid-turn status delivered as final; 4 P2 (active_turns leak, model-reject fallback, deadline intent-gate, reset thread cache). RED 0/5 -> GREEN 5/5; lib 3/3. Some leak paths untested (stated).
- fix-cron-scheduler: VERIFIED. scheduler.rs (+tests file). 2 P1 (catch_up_hours panic; stale-CAS no schedule advance) + 3 P2. RED 0/6 -> GREEN 6/6; lib cron::scheduler 28/28.

- fix-lanes: NOT VERIFIED. Marked completed with no report. Own tests RED 0/5 -> GREEN 5/5, but tests/test_lane_events.rs::missing_destination_never_calls_sink regressed (Pending->Failed) due to its P2 #4 fix; old assertion encodes the defect. Relaunched (P) to update that assertion, rerun lane suites, write report.

- fix-lanes: lead-completed (see fix-r1/fix-lanes.md). Updated stale test_lane_events assertion; reordered cli.rs start() so no Discord I/O runs inside BEGIN IMMEDIATE. PENDING final-gate confirmation.

- fix-web: VERIFIED (web/src only + rebuilt web/dist). 3 P1 fixed (turn completion on all terminal shapes, latest pages, session binding); RED 4/13 (pre-fix transcription) -> GREEN 13/13; tsc+vite build exit 0. P1 #4 bot clear: persona fixed by worker; model/toolsets BLOCKED-SCOPE completed by LEAD: src/dashboard.rs update_bot now maps explicit empty model/prompt/toolsets to NULL (omitted still = keep); bot-payload.ts sends "" / []; added backend assertions to bot_update_with_only_name_preserves_other_fields (runs in final gate) and a web test -> bun test 14/14.
- NOTE: web/dist changed as build output (index.html + asset hash). Keep or revert at commit time per user.

## Post-directive (no builds) fix reports - code read, test pending final gate
- fix-tools-context: cron.rs/message_context.rs/web.rs + tests/test_fix_fix_tools_context.rs; RED 1/7 captured earlier; GREEN deferred to final gate.
- fix-yolo-live: actor.rs SetYolo + router.rs set_yolo + commands.rs yolo_toggle/stop_session; RED captured (live actor did not observe yolo=true); GREEN deferred to final gate.
- fix-sessions-judge: external.rs/judge/mod.rs + tests/test_fix_fix_sessions_judge.rs (8 tests); RED 7/8 fail captured; GREEN deferred. --lead validation not applied (P2, breaks test_sessions_control.rs:248; follow-up in cli.rs).
- fix-cron-exec: store.rs/executor.rs/remediate.rs/report.rs + tests/test_fix_fix_cron_exec.rs (8 tests); RED 0/8 captured; GREEN deferred. Residual P2: delivery failure after successful run still consumes monitor snapshot (scheduler.rs); dead cron_ack_command handling in omo_backend.rs.
- fix-dashboard-share: main.rs attach_gateway_dashboard + run_gateway_with_dashboard; entry.rs routes combined mode to shared gateway dashboard (Discord on); standalone unchanged. Test runner_tests::fix_share_combined_dashboard_resolves_gateway_approvals written. RED NOT CAPTURED (compile timed out at load ~70) - gap noted; GREEN deferred to final gate.
- fix-lanes-guard: guard.rs (Discord outside tx, bot binding, per-lane failure isolation, oldest-first), scrub.rs (PKCS#8, URL-safe home regex), signals.rs (bounded tail, process-group kill), kibitzer.rs (restore_admitted) + tests/test_fix_fix_lanes_guard.rs. RED NOT CAPTURED (killed while compiling); GREEN deferred. Compile risk: Executor generic in active_lane_count.
- fix-migrate: mod.rs dry-run cutover precheck before bring_gateway_down; cron_cutover.rs import-injected key normalization; config_import.rs ACCESS_CONTROL_KEYS never fall back to profile. RED+GREEN captured pre-directive (test_fix_fix_migrate 3/3, lib migrate:: 51/51). VERIFIED (report+logs).
- Lead edit tests/test_migrate.rs: cutover_survives_startup_sync and cutover_second_store_failure_preserves_bytes_and_receipt were wall-clock dependent (newest_missed_occurrence only finds 09:00 if now is shortly after 09:00 UTC). Pinned scheduler clock via with_clock(2026-01-15T10:00:30Z) and next_run_at=08:59Z so daily(09:00) and standup(10:00) are deterministically due. Test-only; runs in final gate.
- fix-mirror-voice: mirror.rs (no thread fallback), voice/pipeline.rs (STT timeouts), voice_transcription.rs (prereq check), readiness.rs (appserver URL), lanes_block.rs (updated_at) + tests/test_fix_fix_mirror_voice.rs. RED NOT CAPTURED (lock wait); GREEN deferred. usage.rs KST bucket = NOT-A-BUG (regraded). text_content skip BLOCKED-SCOPE (adapter.rs:1564) -> P2 follow-up.
- fix-mirror-voice restore check: all 5 source files identical to mv-fixed backups (no RED variant left).
- fix-tools: terminal.rs (process group + killpg drain), browser.rs (final-URL SSRF check + per-session owned_tabs), file.rs (skip unreadable entries). RED 0/5 captured; GREEN deferred to final gate. Residual P2: final-URL check fail-open when /json/list unreadable.

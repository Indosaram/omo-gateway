# CONFIRMED-r2 (lead falsification)

## r2-web-deploy (st_01a1307c)
- [P1 CONFIRMED -> FIXED by lead] web/src/lib/chat-turn.ts:52 is_final chunk ended turn. Falsified via sed: omo_backend.rs:1464/1534 call deliver_rendered_result mid-turn -> emit_chunk(...,true); actor.rs release_typing emits Typing{active:false} at turn end. Fix: is_final chunk -> replace chunk; ev.is_final terminal check removed; test 'final stream chunk is a mid-turn message, not a turn end' updated (run in final gate).
- [P2 recorded] HTTP fallback completes on 202 ack (chat-turn.ts:125-136).
- [P2 recorded] Playground unmount leaves socket open (App.tsx:520).
- [P2 recorded] Session switch keeps stale transcript (App.tsx:520-533).
- [P2 recorded] Docker image ships no web/dist (Dockerfile:26); dashboard loopback-only by design.

## r2-security (st_01a1307d)
- [P0 CONFIRMED -> FIXED by lead] approval_display.rs:12 Authorization redaction missed quoted name (JSON/dict) and quoted credential. Regex now allows authorization["']? and a quoted credential. Tests added to approval_display_redacts_any_authorization_scheme (4 new inputs).
- [P1 CONFIRMED (pre-existing) -> FIXED by lead] hardline.rs:23 cmdpos missed /bin/rm, /sbin/reboot, sudo -u root, env -i. cmdpos now allows a path prefix on sudo/env/program, sudo options with one argument, env dash options. Test hardline_catches_path_qualified_and_wrapped_programs (+ negative /bin/rm -rf target).
- [P1 CONFIRMED (R1 regression) -> FIXED by lead] normalize.rs:601 comment break + BACKSLASH_ESCAPE_RE turned \# into word-start #, hiding --pre. BACKSLASH_ESCAPE_RE no longer unescapes '#'. Test escaped_hash_does_not_hide_execution_flags.
- [P2 recorded] variant-budget exhaustion on benign large heredoc cron scripts -> unapprovable hardline (normalize.rs:1064).
- [P2 recorded] '# ... $HOME' trailing comment after rm -> hardline false positive (hardline.rs:24).
- [cleanup by lead] removed print-only scratch_probe_tests module from terminal.rs (left by fix-tools).

## r2-discord (st_01a1327a)
- 0 P0/P1. 8 P2 recorded (see r2/r2-discord.md): /retry restore drops platform_message_id + non-atomic; /retry may resend [system]/mirror rows; steered messages keep start reaction; drain-path flush drops buffered messages; archive handler stops long silent turns + fragile metadata write; interaction ids as platform_message_id; steer probe treats failed as duplicate; /compress & /skill read >2000 chars.
- Lead rubric check: the drain-flush and /retry-system-row items have realistic triggers but outcomes are recoverable/rare (drain: backfill or resend; /retry: only when the last user row is system-origin) -> keep P2.

## r2-lanes (st_01a13280)
- [P1 CONFIRMED (amplified by R1 200k diff cap) -> FIXED by lead] judge/mod.rs:1466 stdin write_all outside the 15s timeout; a jev child that never reads stdin blocks forever on >64KiB diff, stalling settle_tick for all lanes. Write + wait_with_output now joined inside the single 15s timeout (child dropped on expiry -> kill_on_drop). Test done_diff_returns_when_judge_never_reads_a_large_stdin (bounded 60s outer timeout).
- 8 P2 recorded (r2/r2-lanes.md): auto-close never re-arms failed intent; per-row ? aborts drain/serve; terminal Failed posts silent; status receipt duplicate after 15min bound; retry_queued orphan thread not archived; start() Err after commit; remote vs local json_path grammar; judge scrubber weaker than lanes::scrub.

## r2-multiplexer-agent (st_01a1327c)
- [P1 CONFIRMED (pre-existing, same in HEAD) -> FIXED by lead] actor.rs idle Reset set SessionState::default(), dropping profile enabled_toolsets/system_prompt/model (toolset restriction unenforced until reload). New reset_state() rebuilds via fresh_context() (split out of load_context None branch; actor now keeps profile_router). Test idle_reset_keeps_profile_toolset_restriction.
- [P1 CONFIRMED (pre-existing) -> FIXED by lead] mid-turn Reset broke TurnOutcome::Shutdown, ending the actor and failing/dropping queued messages. New TurnOutcome::Reset: interrupt turn, release typing, fail current ack, reset_state, flush, reply; actor keeps processing pending events. Test mid_turn_reset_keeps_actor_serving_the_session. Existing shutdown_drain_fails_queued_plain_event_delivery re-pointed to channel-close trigger (still covers drain path); mid_turn_reset_does_not_restore_thread_binding_or_mark_resume_pending unchanged.
- P2 recorded (r2/r2-multiplexer-agent.md): zero-config daemon --ws-auth off (P2 pending Origin check); fallback ignores messages_delivered; interrupt drain stops at non-text frame; router DashMap guard across await (set_model/set_yolo/reset); WaitIdle reply dropped / last_active_at not refreshed; llm EOF-without-terminator.
- Lead note: router DashMap-guard-across-await risk is reduced now that mid-turn Reset no longer exits the actor (no remove_if during the awaited reply).

## r2-security re-verify (st_01a132a8, scoped)
- All three lead regex fixes verified correct for listed inputs; no new P0/P1 from them.
- [P2 -> regraded P1 by lead rubric, consistent with prior hardline bypass P1 -> FIXED] env option with argument (env -u HOME rm -rf /, env -C /tmp rm -rf /etc) bypassed cmdpos. env options now take one optional non-dash, non-= argument.
- [P2 regression from lead's own fix -> FIXED] path prefix made ./shutdown.sh, scripts/reboot-test, /usr/local/bin/halt.sh hardline. shutdown|reboot|halt|poweroff rule now requires a command boundary (\s|$|[;&|)`]) instead of \b.
- Tests added to hardline_catches_path_qualified_and_wrapped_programs (2 positive env cases, 3 negative script-name cases).
- P2 recorded: SUDO_STDIN_RE ignores /usr/bin/sudo -S and sudo -n -S (pre-existing); AWS SigV4 multi-param Authorization partially visible.

## r2-dashboard (st_01a1327b)
- [P1 CONFIRMED (R1 regression, compile break) -> FIXED by lead] main.rs:1048 'home' Option<String> shadowed PathBuf and was moved, then used at 1210 hermes_skill_dirs(&hermes_root, &home). Renamed DM binding to home_dm. Verified by final-gate compile.
- [P1 CONFIRMED -> FIXED by lead] dashboard.rs create_bot ON CONFLICT DO UPDATE silently clobbered an existing profile. Now ON CONFLICT DO NOTHING + 409 when rows_affected == 0. Test bot_create_with_existing_id_returns_conflict_and_keeps_profile. (Single statement, so no check-then-write race.)
- [P1 CONFIRMED -> FIXED by lead] main.rs StartupAuthorization rejected DMs via DISCORD_ALLOWED_CHANNELS while live ingress exempts DMs -> DM resume markers deferred forever. Allow-list now applies only when guild_id is Some; ignored channels still apply. Test startup_recovery_exempts_dms_from_channel_allowlist.
- [P1 CONFIRMED (amplified by R1 fail-closed dispatcher) -> FIXED by lead] standalone dashboard/serve always started its own cron scheduler on the shared DB and claimed Discord jobs it cannot deliver. Scheduler now opt-in via OMON_DASHBOARD_RUN_SCHEDULER (zero-config default off; combined mode already passed false). Test standalone_scheduler_is_opt_in.
- 7 P2 recorded (r2/r2-dashboard.md): recovery auth vs live admission (roles/thread ids/ingress source); DELETE session not detached-guarded; cron create/bot update check-then-write races; invalid IGNORED_CHANNELS/ALLOWED_USERS parse; standalone shutdown does not drain multiplexer; detached WS chat_available misleading; entry.rs dashboard result masking; config view inconsistency.

## r2-inbox-migrate-voice (st_01a1327f)
- [P1 CONFIRMED -> FIXED by lead] config_import.rs: profile .env APPROVAL_MODE=never won over root config.yaml approvals.mode (root_value.or(profile_value)), disabling approvals gateway-wide. APPROVAL_MODE and APPROVALS_DESTRUCTIVE_SLASH_CONFIRM added to ACCESS_CONTROL_KEYS (never inherited from a profile; root .env then root config.yaml). Test profile_approval_mode_never_overrides_root_config.
- [P1 CONFIRMED -> FIXED by lead] voice_transcription.rs: ffmpeg/whisper-cli .output() had no timeout and no kill_on_drop; a stalled child blocked routing, debounce flush and drain. New run_bounded helper (kill_on_drop + tokio timeout; ffmpeg 60s, whisper 300s); temp .wav/.txt removed on every error path. Test bounded_helper_kills_a_stalled_tool.
- 8 P2 recorded (r2/r2-inbox-migrate-voice.md): GatewayMessage drop on full channel; dropped progress results strand triage; migrate preflight vs non-selected profiles; /api/ready never gates on backend; voice double transcription; merge drops existing bot tokens; bring_gateway_down unloads before pid verify; readiness probe keeps URL path.

## r2-cron (st_01a1327d)
- All five R1 cron fixes verified correct.
- [P1 CONFIRMED -> FIXED by lead] terminal.rs is_protected_env_key protected only 7 keys; GIT_SSH_COMMAND/BASH_ENV/NODE_OPTIONS/PYTHONPATH etc. change what an approved command runs. Now blocks PATH, shell startup (BASH_ENV, ENV, SHELLOPTS, PROMPT_COMMAND, IFS), interpreter preload (NODE_*, PYTHON*, PERL5*, RUBY*, JAVA_TOOL_OPTIONS) and the LD_/DYLD_/GIT_/BASH_FUNC_ prefixes, case-insensitively. Test caller_env_cannot_override_executable_search_or_loader extended (7 new keys incl. lowercase).
- [P1 CONFIRMED -> FIXED by lead] tools/cron.rs add stored ack_command (run unattended via sh -c after delivery) with only the lifecycle check; now screened by check_cron_script (lifecycle + hardline + dangerous). update has no ack_command path. Test cron_ack_command_rejects_hardline_and_dangerous_commands.
- 6 P2 recorded (r2/r2-cron.md): failed runs count toward repeat.times / manual trigger disable without CAS; interval catch-up count as i32; reconcile_fixed on failed runs; CronTool update RMW without CAS; profile key inconsistency + LIKE fallback cross-profile; misc (obligation id collision, ack no process group, browser final-URL after request, file write TOCTOU, report Seoul TZ).

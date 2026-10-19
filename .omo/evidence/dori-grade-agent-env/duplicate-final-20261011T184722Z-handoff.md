# Scoped duplicate-final gate release
All requested scoped checks passed once: fmt0, strict offline Clippy alltargets/allfeatures0; completed_4passed; failed_final_delivery_is_not_acknowledged1passed; headless5passed; events25passed/1helperignored; runner12passed/1liveignored. The three startup child cases also each passed. Exact names/raw FRAME identifiers and final stream944f3e3d-fb8d-49fc-8c55-ad799fb23e52 bodyTASK14_ACTUAL_REPLY retained in logs. Updated terminal assertions passed.
205 inputs unchanged across launch/gate/build/recovery, sourcecmp0. Manifest SHA256 a9570ea35fb0d696bffd5f7c5154f5f01a3663fd6c59ff26f0cd0acbedb3d06d. Binary SHA256 f81c077ee6a1252d647b7fc8f45758f8499b687dc9d2bb84d57bb3b512f15912. Machine release sibling -release.json gives absolute paths. Sources stay frozen for parent12/14 actualQA; none executed here.
Original requested build attempted once exit101 because this evidence harness deleted owned TMPDIR before compilation; no product error. Failure raw retained. Build-only corrected recovery PID93332 monitor mon_X1HBVPZK60C0YV2V exit0,5.24s, sourcecmp0/cleanup0. No test/Clippy replay. Original gate PID87422 monitor mon_W6ZWZCGZWKE0B1ZQ completed. Bash LSP unavailable; bash-n syntax checked. No source edits/service/env/git/model/Discord actions.
Prior durability-fixtures180128Z 30minute backend timeout receipt preserved, not relabeled green; no wholebackend/inbox/session/http/mux replay. Pending historical tests remain test_omo_backend_e2e_thread_lifecycle_and_streaming and test_omo_backend_retries_once_after_connection_drop. This new scoped pass does not certify those tests.
## Gate artifact hashes
a9570ea35fb0d696bffd5f7c5154f5f01a3663fd6c59ff26f0cd0acbedb3d06d pre-manifest.sha256
a9570ea35fb0d696bffd5f7c5154f5f01a3663fd6c59ff26f0cd0acbedb3d06d post-manifest.sha256
e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 fmt.log
27de499e5fbbdc50ab6ec5d841cf1d896da96970e37da985b87d2f4f0aa7a9ac clippy.log
6cddb500b9f2a2ea44e1de25a9f9bde51d8f9e9c6e83ee5ccd74e999ed1352f1 completed.log
732b8bac2f227badd64fb67b7f72d483a4e434d7576a5e8e51553b9fad51b731 delivery.log
58032ce6202518824ca929c32cb54026fd5832aebbeb7891a0b1790982164ed5 headless.log
03406b6dbb18915beceb440ed7e1f8c23a8ea8611f8995c1eaecc956ed2d14f2 lanes.log
4646ba7c8f7b5ed538d11d30b7ae5e1e2852f6152fa45d9db0be5c40f030e188 build.log
## Exact scoped statuses
completed: test test_omo_backend_ignores_turn_completed_without_content ... ok
completed: test completed_tool_only_finalizes_once_with_empty_result_fallback ... ok
completed: test test_omo_backend_ignores_turn_completed_for_other_thread ... ok
completed: TASK14_FRAME method=null item=null turn=null
completed: TASK14_FRAME method="item/started" item="tool-final" turn="t2"
completed: TASK14_FRAME method="item/completed" item="m-progress" turn="t2"
completed: TASK14_FRAME method="item/agentMessage/delta" item="m1" turn="t2"
completed: TASK14_FRAME method="item/completed" item="m1" turn="t2"
completed: TASK14_FRAME method="thread/status/changed" item=null turn="t2"
completed: TASK14_FRAME method="turn/completed" item=null turn="t2"
completed: TASK14_FINAL stream=944f3e3d-fb8d-49fc-8c55-ad799fb23e52 body=TASK14_ACTUAL_REPLY
completed: test completed_item_finalizes_once_at_turn_terminal ... ok
completed: test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 45 filtered out; finished in 0.05s
delivery: test failed_final_delivery_is_not_acknowledged ... ok
delivery: test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 48 filtered out; finished in 0.04s
headless: test legacy::headless_tests::headless_output_rejects_upload_and_approval ... ok
headless: test legacy::headless_tests::headless_watcher_cancels_and_joins ... ok
headless: test legacy::headless_tests::headless_active_shutdown_joins_interrupt_and_preserves_binding ... ok
headless: test legacy::headless_tests::headless_marker_reply_and_joined_shutdown ... ok
headless: test legacy::headless_tests::headless_early_shutdown_preserves_accepted_reply ... ok
headless: test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 56 filtered out; finished in 0.23s
lanes: test configured_host_subprocess_helper ... ignored, invoked by configured_hosts_scrub_stored_and_delivered_values_in_isolation
lanes: test cumulative_snapshots_replace_content_and_finalize_once ... ok
lanes: test backend_captures_completed_lane_items_with_stable_identity ... ok
lanes: test configured_hosts_scrub_stored_and_delivered_values_in_isolation ... ok
lanes: test ordinary_activity_rearms_stale_but_idle_turns_never_fire ... ok
lanes: test parser_rejects_foreign_malformed_and_empty_reports ... ok
lanes: test old_generation_item_cannot_enqueue_post ... ok
lanes: test done_report_requires_claim_and_never_verifies_lane ... ok
lanes: test missing_destination_never_calls_sink ... ok
lanes: test invalid_judge_choice_leaves_registry_unchanged ... ok
lanes: test interrupted_send_becomes_uncertain_on_restart ... ok
lanes: test concurrent_claim_wins_over_judge_response ... ok
lanes: test milestone_posts_without_waking_lead ... ok
lanes: test event_insert_failure_rolls_back_block_and_report ... ok
lanes: test claimed_lane_suppresses_done_and_question_events ... ok
lanes: test ambiguous_post_stays_uncertain_without_retry ... ok
lanes: test agent_claim_event_scrubs_and_bounds_without_changing_internal_evidence ... ok
lanes: test failed_post_enqueue_rolls_back_stale_event_and_latch ... ok
lanes: test acknowledged_post_is_not_sent_again_after_recovery ... ok
lanes: test runner_question_pauses_after_one_turn_without_outcome_lock_deadlock ... ok
lanes: test agent_post_and_event_scrub_token_but_owner_answer_stays_exact ... ok
lanes: test notification_does_not_suspend_backend_holding_sqlite_connection ... ok
lanes: test quiet_threshold_and_restart_are_idempotent ... ok
lanes: test stable_item_replay_does_not_duplicate_post ... ok
lanes: test question_and_blocker_pause_with_one_persistent_event ... ok
lanes: test unreported_intents_are_bounded_scrubbed_and_typed ... ok
lanes: test result: ok. 25 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 0.84s
lanes: test live_qa_runner_process ... ignored, requires a disposable app-server and explicit QA database
lanes: test startup_discord_flag_controls_token_requirement ... ok
lanes: test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 12 filtered out; finished in 0.29s
lanes: test repeated_cancellation_keeps_lane_resumable ... ok
lanes: test startup_discord_flag_controls_token_requirement ... ok
lanes: test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 12 filtered out; finished in 0.22s
lanes: test three_errors_fail_with_dead_event ... ok
lanes: test misleading_success_output_never_verifies_or_closes_lane ... ok
lanes: test active_send_steers_and_consumes_only_after_ack ... ok
lanes: test stale_lane_binding_prefers_durable_session_binding ... ok
lanes: test restart_uses_persistent_thread_and_idle_message ... ok
lanes: test two_failed_checks_escalate_small_lane_only_once ... ok
lanes: test chains_until_claim_and_captures_without_discord ... ok
lanes: test dirty_worktree_is_preserved_by_claim ... ok
lanes: test twenty_four_turn_cap_fails_with_event ... ok
lanes: test service_recovers_file_database_and_shutdown_releases_socket ... ok
lanes: test startup_discord_flag_controls_token_requirement ... ok
lanes: test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 12 filtered out; finished in 0.13s
lanes: test startup_discord_flag_controls_token_requirement ... ok
lanes: test result: ok. 12 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 0.68s

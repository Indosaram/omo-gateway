# Exact binary release under bounded baseline policy
Build actual exit0,25.38s, once: cargo build --offline -j2 --bin omo-gateway. PID43621 monitor mon_CGF50RERTWNYV6V4 completed0. Source pre/post cmp0; all205 inputs unchanged across gate, runner, build. No product edits or live QA here.
Binary /Users/indo/code/project/omon-gateway-wt/dori-work/target-w4/debug/omo-gateway SHA2563b852fbd4d5f93911a3200fe07bb8bb7d037f317c24a7bc46fcc60a5b314b933.
Final manifest durability-fixtures-20261011T180128Z-build-post-manifest.sha256 SHA2560d3fc34abc10daa6d15f4de0745762ff26e6e3afcfb39137e347b6b70095b9ee.
fmt0, strict offline Clippy0, headless5passed exit0. Inbox26passed/4ignored, dispatch10passed, events25passed/1ignored, mux19passed. Sequential runner exit0/12passed/1ignored includes three_errors_fail_with_dead_event and isolated startup token cases. Runner/sourcecmp/cleanup0; original empty owned scratch cleanup0.
Original combined monitor timed_out1 at1800000ms; focused Cargo exit unknown, no target terminal summary fabricated. Backend36explicit passes/8baseline failures/1ignored/2pending. Zero observed NEW failures; not all-green. Pending exact test_omo_backend_e2e_thread_lifecycle_and_streaming and test_omo_backend_retries_once_after_connection_drop. failed_final_delivery_is_not_acknowledged PASSED. Baseline raw canonical135508Z proves all8 observed failure names existed;15 old failures suite-wide/10backend. Separate timeout and baseline-policy receipts retained. Sourcefreeze stays in place for parent12/14 liveQA; no fresh preparation/service/model/env/git/Discord action.
## Retained artifact hashes
c5beb17f7a719daa724be89546132df4c45238b63d3775a8427940b82b14c1b8 -focused.log
69cf2e028964fcfd16b5dce8325398693996b8ee50b74c90f60eb5ff0d8dc030 -runner.log
deae790712db0895b349b61788405b2752fe8359cd325cebcc74478bb9ae49f7 -headless.log
4d3e1fa017ce726156b3cac943e5f71af3e0ef50004b4e3edd56897694df880b -clippy.log
8acd2e41230f51a3ae5cc7ffc057fc91841f50bdd176544c0e2a1b403c03e92c -build.log
0d3fc34abc10daa6d15f4de0745762ff26e6e3afcfb39137e347b6b70095b9ee -build-post-manifest.sha256
## Exact related-test status lines (interleaved output preserved)
test fixture_files_contain_only_sanitized_personal_text ... ok
test imessage_rejects_missing_history_and_bad_timestamps ... ok
test kakao_numeric_seconds_take_precedence_when_alias_conflicts ... ok
test imessage_accepts_single_object_output_and_numeric_rowids ... ok
test imessage_decodes_captured_rows_in_observed_ndjson_framing ... ok
test kakao_numeric_seconds_when_synthetic_contract_row_has_no_alias ... ok
test imessage_uses_only_read_subcommands_and_skips_own_messages ... ok
test imessage_parses_authenticated_sanitized_chat_and_history ... ok
test kakao_parses_authenticated_numeric_seconds_capture ... ok
test imessage_parses_multi_item_ndjson_history ... ok
test live_discord_source_fetch_once ... ignored
test live_imessage_source_fetch_once ... ignored
test live_kakao_source_fetch_once ... ignored
test live_mail_source_fetch_once ... ignored
test kakao_rejects_invalid_numeric_time_even_when_alias_valid ... ok
test kakao_rejects_send_at_when_supported_timestamp_absent ... ok
test kakao_skips_invalid_time_when_message_at_or_below_floor ... ok
test kakao_rfc3339_aliases_when_numeric_time_absent ... ok
test mail_parses_authenticated_empty_capture_without_populated_substitution ... ok
test mail_parses_sanitized_thread_and_builds_read_only_cursor_command ... ok
test mail_rejects_missing_shape_and_empty_cursor_is_stable ... ok
test mail_rfc3339_internal_date_advances_cursor_from_parsed_timestamp ... ok
test kakao_production_parser_and_per_room_cursor ... ok
test kakao_fetches_authenticated_historical_room_on_owned_loopback ... ok
test kakao_fetches_authenticated_room_and_numeric_page_on_owned_loopback ... ok
test kakao_event_driven_paging_loop ... ok
test discord_parses_authenticated_sanitized_indexed_row ... ok
test discord_producer_event_to_adapter_end_to_end ... ok
test discord_behavioral_db_cases_and_cursor ... ok
test kakao_bad_endpoint_collects_other_three_sources_and_preserves_cursor ... ok
test origin_defaults_for_legacy_events_and_roundtrips_system ... ok
test route_error_keeps_events_undelivered_for_retry ... ok
test user_admitted_after_idle_probe_refuses_system_turn ... ok
test system_event_triggers_no_outbound_reaction ... ok
test done_claimed_delivers_immediately_after_idle_delivery_slot ... ok
test two_done_claims_five_seconds_apart_wait_thirty_seconds ... ok
test event_during_in_flight_turn_waits_until_turn_ends ... ok
test jev_below_threshold_marks_digest_only_without_turn ... ok
test duplicate_lane_kind_pairs_collapse_to_latest_body ... ok
test three_events_within_ten_seconds_coalesce_into_one_turn ... ok
test configured_host_subprocess_helper ... ignored, invoked by configured_hosts_scrub_stored_and_delivered_values_in_isolation
test cumulative_snapshots_replace_content_and_finalize_once ... ok
test backend_captures_completed_lane_items_with_stable_identity ... ok
test configured_hosts_scrub_stored_and_delivered_values_in_isolation ... ok
test old_generation_item_cannot_enqueue_post ... ok
test parser_rejects_foreign_malformed_and_empty_reports ... ok
test acknowledged_post_is_not_sent_again_after_recovery ... ok
test interrupted_send_becomes_uncertain_on_restart ... ok
test claimed_lane_suppresses_done_and_question_events ... ok
test event_insert_failure_rolls_back_block_and_report ... ok
test concurrent_claim_wins_over_judge_response ... ok
test invalid_judge_choice_leaves_registry_unchanged ... ok
test agent_post_and_event_scrub_token_but_owner_answer_stays_exact ... ok
test failed_post_enqueue_rolls_back_stale_event_and_latch ... ok
test notification_does_not_suspend_backend_holding_sqlite_connection ... ok
test missing_destination_never_calls_sink ... ok
test agent_claim_event_scrubs_and_bounds_without_changing_internal_evidence ... ok
test milestone_posts_without_waking_lead ... ok
test done_report_requires_claim_and_never_verifies_lane ... ok
test runner_question_pauses_after_one_turn_without_outcome_lock_deadlock ... ok
test ambiguous_post_stays_uncertain_without_retry ... ok
test ordinary_activity_rearms_stale_but_idle_turns_never_fire ... ok
test question_and_blocker_pause_with_one_persistent_event ... ok
test quiet_threshold_and_restart_are_idempotent ... ok
test stable_item_replay_does_not_duplicate_post ... ok
test unreported_intents_are_bounded_scrubbed_and_typed ... ok
test route_reports_actor_startup_failure_instead_of_acknowledging_a_lost_event ... ok
test gc_does_not_evict_an_actor_with_an_active_turn ... ok
test stop_immediately_cancels_the_active_turn ... ok
test guild_lane_is_shared_per_bot_not_author ... ok
test persistence_failure_never_acknowledges_turn_on_user_insert ... ok
test events_arriving_during_running_turn_are_queued_and_processed_in_order ... ok
test scale_to_zero_evicts_and_flushes_idle_sessions ... ok
test delivery_ledger_deduplicates_concurrent_claims_and_records_latency ... ok
test dropping_multiplexer_releases_actor_cycle_and_flushes_dirty_state ... ok
test stop_cancels_active_turn_and_clears_queued_events ... ok
test live_controls_change_authoritative_session ... ok
test routes_multiple_sessions_in_parallel ... ok
test persistence_failure_never_acknowledges_turn_on_session_update ... ok
test flush_failure_replays_completed_turn_on_restart ... ok
test handles_events_sequentially_within_one_session ... ok
test thread_inherits_parent_profile ... ok
test persistence_failure_never_acknowledges_turn ... ok
test terminal_outcomes_release_typing ... ok
test transcript_level_inbound_dedup_skips_duplicate_platform_message_id ... ok
test lane_live_daemon_turn_against_mktemp_repo ... ignored
test lane_missing_cwd_returns_error ... ok
test missing_rollout_fails_with_continuity_error ... ok
test fallback_one_shot_guard_never_refalls_back_from_fallback ... ok
test discord_silent_turn_interrupted_after_45s ... ok
test correlation_distinguishes_interrupted_and_failed ... ok
test omo_approval_roundtrip_uses_session_policy ... ok
test profile_tool_restrictions_are_enforced ... ok
test correlation_requires_ack_and_ignores_subscription_replay ... ok
test rejects_foreign_and_stale_turn_frames ... ok
test lane_thread_start_carries_cwd_and_no_typing ... ok
test lane_silent_turn_not_interrupted_after_50s ... ok
test model_rejected_failure_triggers_fallback_to_deepseek_and_persists_marker ... ok
test lane_thread_resumes_on_subsequent_turn ... ok
test test_omo_backend_cron_session_suppresses_activity_lines_and_delivers_only_final_result ... ok
test test_omo_backend_cron_sessions_always_start_fresh_thread ... ok
test test_omo_backend_accepts_received_completion_at_deadline_edge ... FAILED
test test_omo_backend_emits_hermes_activity_lines ... FAILED
test test_omo_backend_deadline_fires_on_ping_only_stream ... ok
test test_omo_backend_ignores_turn_completed_for_other_thread ... ok
test test_omo_backend_ignores_turn_completed_without_content ... ok
test test_omo_backend_omits_workspace_when_per_agent_disabled ... FAILED
test test_omo_backend_empty_terminal_past_grace_fails_fast ... ok
test interactive_deadline_bounds_all_protocol_phases ... ok
test test_omo_backend_provisions_agent_workspace_and_passes_to_thread_start ... FAILED
test test_omo_backend_replaces_stale_cached_thread ... ok
test persistence_failure_prevents_turn_submission ... ok
test accepted_disconnect_does_not_resubmit_turn ... ok
test pre_ack_disconnect_does_not_resubmit_turn ... ok
test test_omo_backend_surfaces_turn_start_rpc_error_immediately ... ok
test test_omo_backend_skips_ack_when_delivery_fails ... ok
test failed_final_delivery_is_not_acknowledged ... ok
test test_omo_backend_waits_for_final_message_after_tool_activity ... FAILED
AG-01 cleanup peer=test stop_interrupts_remote_turn_before_ack ... ok
test stop_before_start_ack_never_reports_uncancelled_success ... ok
test test_omo_backend_runs_ack_command_after_cron_delivery ... ok
test stop_interrupts_remote_turn ... ok
test pending_start_stop_public_surface ... ok
test test_omo_backend_persists_without_preexisting_session_row ... ok
test failed_first_turn_keeps_durable_binding ... ok
test pre_submit_disconnect_retries_and_succeeds ... ok
test test_omo_backend_requires_turn_terminal_even_after_final_agent_message ... FAILED
test test_omo_backend_aborts_turn_on_repeated_approval_denials ... FAILED
test test_omo_backend_deadline_interrupts_looping_turn ... FAILED
test test_omo_backend_unreachable_daemon_error ... ok

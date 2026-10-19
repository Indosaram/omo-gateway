# Original combined gate terminal timeout

Original monitor mon_9QEJN90HPYP59H5H/session bash_14 reached1800000ms and delivered watcher timed_out exit1. This is monitor termination, not a fabricated Cargo101/0: original focused Cargo numeric exit is unavailable because batch shell was terminated before wait/status write. No completed marker or focused status persisted. Raw log preserved unchanged. At18:32:03Z, ps56122/58015/59356 had no rows and target-w4 lock no owner. No duplicate or premature kill was issued.

Actual recorded command statuses: fmt0, strict offline all-target/all-feature Clippy0/no diagnostics, headless0/5passed including headless_early_shutdown_preserves_accepted_reply. Related test target completed summaries: inbox26passed/0failed/4liveignored, dispatch10passed/0failed, events25passed/0failed/1helperignored, multiplexer19passed/0failed. All exact names/statuses in -focused.log. Backend unfinished:35 explicit ok lines,8 explicit FAILED lines,1ignored,2 exact pending names; no final backend summary invented. Partial name-line count totals46 while banner47; any remaining unclassified outcome remains unverified, not fabricated.

Pending at timeout:
* test_omo_backend_e2e_thread_lifecycle_and_streaming
* test_omo_backend_retries_once_after_connection_drop

Observed failures all match actual canonical batch135508Z backend failure names (todo2 mapping in preserved handoff). The old15 total failures are suite-wide,10 backend, not15 backend. Observed new failures0 does not certify uncompleted tests.
* test_omo_backend_accepts_received_completion_at_deadline_edge
* test_omo_backend_emits_hermes_activity_lines
* test_omo_backend_omits_workspace_when_per_agent_disabled
* test_omo_backend_provisions_agent_workspace_and_passes_to_thread_start
* test_omo_backend_waits_for_final_message_after_tool_activity
* test_omo_backend_requires_turn_terminal_even_after_final_agent_message
* test_omo_backend_aborts_turn_on_repeated_approval_denials
* test_omo_backend_deadline_interrupts_looping_turn

failed_final_delivery_is_not_acknowledged explicitly passed current raw log172; failed delivery semantics not assumed regressed. Original run sourcefreeze posthash recovered by queued runner script, not by replay. Original test TMPDIR remains for filesystem-only cleanup after handles gone. Queued one focused test_lane_runner now starts sequentially under mon_T0C8JG5ER0EYXT24/session bash_17, no original-target replay. Binary decision must carry original timeout/uncovered tests and runner numeric result; no blanket green claim or production source edits.

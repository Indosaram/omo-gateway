# Fix report: node `tools` (lanes: tools-exec, tools-context)

No compile/test command was run (per instructions); every item below is edit-only. Tests are in existing `#[cfg(test)]` modules.

## P1

- **web_search never parses DuckDuckGo results** -> FIXED
  - src/tools/web.rs: `parse_duckduckgo_results`, `element_with_class`, `resolve_duckduckgo_href`; `clean_html` now decodes `&#x27;`.
  - Splits on `result__body` substring; title from `result__a`, snippet from `result__snippet`, URL from `result__a` href with `uddg=` decoded (scheme-relative `//` and `&amp;` handled); ads / non-http targets skipped.
  - Tests: `duckduckgo_results_parse_live_markup_with_decoded_urls`, `duckduckgo_results_honor_max_results`, `duckduckgo_results_reject_non_http_targets`.
- **file write never approval-gated** -> FIXED
  - src/tools/file.rs: `FileTool::new` now defaults `require_write_approval: true` (main.rs / dashboard_runtime.rs need no change); `with_write_approval(false)` is the explicit opt-out.
  - Test: `writes_require_approval_by_default_and_can_opt_out`.
  - Note for the lead: write calls through `ToolRegistry` now fail closed without a session/approval requester (Hermes-cron registry at main.rs:579 `FileTool::new(&workdir)` is affected the same way; it runs agent turns, so add `.with_write_approval(false)` there only if unattended writes are intended. main.rs is outside my scope).
- **terminal SIGKILLs process group at max_output_bytes** -> FIXED
  - src/tools/terminal.rs: `read_capped` drains to EOF keeping head (2/5) + ring-buffer tail (3/5) + exact total (`CapturedOutput`); overflow kill removed (kills remain only for timeout and the existing post-exit PIPE_DRAIN_GRACE); `capture` reports the true omitted count and truncation iff total > limit.
  - Tests: `output_over_cap_is_drained_not_killed` (replaces `runaway_output_is_capped_without_buffering_everything`, which relied on the kill and an endless `yes`), `read_capped_keeps_head_and_ring_tail_with_exact_total`. `backgrounded_grandchild_does_not_hold_tool_until_timeout` still covers the grace kill.
- **browser navigate final-URL check vacuous and fails open** -> FIXED
  - src/tools/browser.rs: `verify_tab_settled` polls `/json/list` (default 250 ms x 40, `with_settle` override) until the tab URL is non-blank and unchanged across two reads, checking every observed URL; any listing error / missing tab / malformed JSON / never-settles closes the tab via `close_tab` and errors (fail closed); missing tab id in `/json/new` is an error; `snapshot` re-validates owned tabs and closes ones on internal hosts.
  - Tests: `navigate_fails_closed_when_tab_is_missing_from_listing`, `navigate_fails_closed_when_listing_is_not_an_array`, `navigate_fails_closed_when_tab_never_leaves_about_blank`, `navigate_catches_redirect_that_lands_after_the_first_read`, `snapshot_closes_owned_tab_that_moved_to_an_internal_host`; existing browser tests updated to use `with_settle(ZERO, n)` and a fake CDP list sequence (no sleeps).
- **cron script/ack_command bypass of approval + deny globs** (tools-context) -> FIXED
  - src/tools/cron.rs: `check_cron_script(script, deny_globs)` now applies `match_user_deny_rule`; deny globs = env `APPROVALS_DENY`/`OMON_APPROVALS_DENY` at call time plus `CronTool::with_deny_globs`; `update` now screens `script` and `ack_command`; `CronTool::requires_approval` returns a reason for add/create/update carrying a non-empty script or ack_command unless policy is Never (`APPROVAL_MODE` or `with_approval_policy`), so the registry's approval gate applies (fails closed with no session/requester). main.rs unchanged: `CronTool::new` is secure by default.
  - Tests: `shell_commands_require_approval_unless_policy_is_never`, `deny_globs_refuse_script_and_ack_command_on_add_and_update`, `update_rejects_dangerous_ack_command_and_invalid_expression`.
- **thread-scoped search** (tools-context) -> FIXED
  - src/storage/message_search.rs: `index_inbound_with_metadata` keys thread messages by `channel_id = thread id` (matches REST path; `thread_id` column kept); `search` additionally excludes rows with `thread_id` set and different from `channel_id` (hides legacy rows and transcript-trigger rows stored under the parent id).
  - Tests: `thread_messages_are_not_returned_to_parent_channel_search`, `transcript_trigger_thread_rows_are_hidden_from_parent_search`.
  - Side effect (outside scope, FYI): src/inbox/sources/discord.rs reply-to-owner lookup joins parents by `parent.channel_id = d.channel_id`; thread messages now carry the thread id as channel_id so owner replies inside threads still match only same-thread parents (previously matched parent-channel messages). Not asserted by any existing test I found (tests/test_inbox_sources.rs uses non-thread sessions).

## P2 (real defects)

- **MCP stdio stderr drain hang after kill** -> FIXED (src/tools/mcp.rs `request_stdio`: own process group + killpg, `stderr_task.abort()`, timeout no longer early-returns before the kill). No new test (needs a process-spawning fixture; MCP client unused in runtime).
- **MCP initialize handshake / response-id matching / body timeout** -> SKIPPED (protocol work, no runtime MCP client; larger change than a minimal fix).
- **Caller env denylist incomplete** -> FIXED (terminal.rs `is_protected_env_key`: added HOME, SHELL, XDG_CONFIG_HOME, RUSTC*, RUSTFLAGS, PAGER/EDITOR/VISUAL, SSH_ASKPASS, LESSOPEN, prefixes CARGO_, NPM_CONFIG_). Test: `caller_env_cannot_spoof_session_attribution_or_hijack_toolchain`.
- **Caller env can overwrite OMON_SESSION_*/HERMES_SESSION_*** -> FIXED (same function: prefixes protected). Same test.
- **Children inherit gateway secrets** -> FIXED (terminal.rs `is_secret_env_key`: DISCORD_BOT_TOKEN(S), DISCORD_TOKEN, OMON_OMO_APPSERVER_AUTH_TOKEN, *_BOT_TOKEN(S), *_APPSERVER_AUTH_TOKEN removed via `env_remove`). LLM `*_API_KEY` deliberately NOT stripped (tools like cargo/gh/aws legitimately read tokens; list is easy to extend). Test: `gateway_secrets_are_not_inherited_by_children`.
- **Terminal leaks readers/group when future dropped; pipe-closed child waits to timeout** -> SKIPPED (needs a drop guard restructure of execute_inner; not a regression, kill_on_drop covers the direct child).
- **SSRF filter coverage (web.rs + browser.rs)** -> FIXED (192.0.0.0/24, 198.18.0.0/15, multicast, 240/4, NAT64, 6to4; web `lookup_host` 5 s timeout). Tests: `reserved_and_embedded_ipv4_ranges_are_internal` (both files).
- **web_fetch returns raw HTML despite description** -> SKIPPED (behavior/doc, no HTML-to-text dependency; follow-up).
- **web_search default max_results 20 vs schema 5** -> FIXED (web.rs, default 5).
- **browser owned_tabs grows unbounded** -> FIXED (closed/vanished tabs removed in `close_tab` and `snapshot`). Test: `snapshot_forgets_tabs_that_no_longer_exist`.
- **file search: empty query matches everything** -> FIXED (rejects empty query; test `search_rejects_empty_query`). Unsorted-at-limit / no truncation flag / blocking walk bounds / non-atomic write -> SKIPPED (output-shape or perf changes; follow-up).

## tools-context P2s

- **message_context search runs FTS twice** -> FIXED (message_context.rs: first call replaced by `MessageSearchIndex::validate_query`).
- **Search hits not neutralized** -> FIXED (`message_from_search_document` applies `neutralize_untrusted_inline_text`; test `search_hits_are_neutralized_before_reaching_the_model`).
- **clients() fails whole set on one bad token** -> FIXED (skips with warn, errors only if none resolve). No offline test (needs Discord HTTP).
- **Lazy provider opens a second pool** -> BLOCKED-SCOPE/SKIPPED (needs wiring from the registry/main to inject the pool; message_context_lazy.rs is not changeable without the caller).
- **message_search non-numeric before_message_id** -> FIXED (rejects non-numeric cursor; test `non_numeric_before_cursor_is_rejected`). Mixed-format timestamp ordering -> SKIPPED (needs a normalization migration; migrations node).
- **messenger_policy corrupt override fails effective()** -> SKIPPED (fail-closed read is intentional; repairing/reporting is a policy decision).
- **skills find_all_skills nondeterministic shadowing** -> FIXED (skills.rs: per-root sort by path depth then path, roots keep precedence; test `shadowed_skill_resolves_to_shallowest_path_in_first_root`).
- **cron runs leaks claim_token** -> FIXED (removed from `runs` output; test `runs_output_omits_claim_token`).
- **cron update validates expression / ack_command** -> FIXED (new expression validated even when disabled; ack_command screened). Test above.
- **cron enabled=false add race; payload compare guard; notepad job-exists check; deliver restriction** -> SKIPPED / BLOCKED-SCOPE (enabled=false race needs `CronJobSpec`/scheduler change in src/cron; others are behavior changes beyond a minimal fix).

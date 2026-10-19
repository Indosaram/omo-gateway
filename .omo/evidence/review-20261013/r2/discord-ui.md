# discord-ui review

Nothing was built or run. Everything below comes from reading the current files and their direct callers.

I traced each fix from this round and found no regression. The fixes are the ephemeral defer and operator gate on pending writes, the `DELETE ... RETURNING` retry and restore, the double reset, the compress window and cutoff, the synthetic empty `platform_message_id`, the typing best-effort, the per-code `attempts` bump removal, the 2s bounded expiry wait, `prune_attachment_cache`, the WAV parser, the table splitter and `fallback_text`, and the `std::sync::RwLock` paired cache. I did find one remaining P1, a dead-end pairing flow, and four P2s.

### [P1] Pairing can never be completed: the DM prompt and `/pair` require mutually exclusive configs
- Location: /Users/indo/code/project/omon-gateway/src/discord/adapter.rs:1766-1777 (`should_prompt_unauthorized_dm`), /Users/indo/code/project/omon-gateway/src/discord/commands.rs:206-223 (`is_user_authorized`), commands.rs:349-405 (`command_check`), commands.rs:1405-1423 (`pair`)
- Trigger: Run the default deployment with `DISCORD_ALLOWED_USERS`, `DISCORD_ALLOWED_ROLES` and `DISCORD_ALLOW_ALL_USERS` unset. An unknown user DMs the bot. `should_prompt_unauthorized_dm` returns true only when both lists are empty and `allow_all_users` is false. The bot replies "Ask an administrator or operator to approve ... with `/pair {code}`". Anyone who then runs `/pair` is rejected. `command_check` fails `is_user_authorized` for an empty list and replies "You are not authorized to use this command." The `pair` handler repeats the same `is_user_authorized` check against the same empty `data.allowed_users`, `data.allowed_roles` and `data.allow_all_users`. `src/main.rs` sets those fields only from the env config. A grep of `src/` shows no other path that calls `approve_code`.
- Impact: When a prompt is issued, no one is able to approve it. Setting any allowlist or `allow_all_users` suppresses the prompt, and clearing them removes every possible operator. Pairing is unreachable in every configuration.
- Fix: Give `/pair` an operator identity that is independent of the allowlists, for example `OMON_OPERATOR_IDS`, the application owner, or a bootstrap operator. Alternatively, send the pairing prompt only when a non-empty operator list exists. Pick one and keep the DM gate and the `/pair` gate consistent.

### [P2] Test cannot fail
- Location: /Users/indo/code/project/omon-gateway/src/discord/buttons.rs:710-715 (`failed_acknowledgement_does_not_claim_interaction_for_execution`)
- Trigger: The test builds a fresh `InteractionLru`, asserts that it does not contain id 42, and asserts that a string constant contains a substring. It never calls `handle_button_click`, so it passes even if the ack-failure path inserts the interaction id into `EXECUTED_INTERACTIONS`.
- Impact: There is no regression coverage for "ack failed means the action did not run and the interaction was not marked executed".
- Fix: Factor the claim, ack and record sequence into a function that takes an ack closure. Test that a failing ack leaves `IN_FLIGHT` and the LRU untouched, and that a successful ack records the id.

### [P2] A grandchild holding stdout or stderr makes a successful action look timed out, so the button is re-armed
- Location: /Users/indo/code/project/omon-gateway/src/discord/buttons.rs:194 (`child.wait_with_output()` inside `timeout`)
- Trigger: The configured command exits 0 but leaves a background process that inherited the pipes. `wait_with_output` blocks until pipe EOF. After the 600s default the code kills the process group and reports `timed_out` with `success=false`, and the failure branch restores the original, clickable button.
- Impact: An irreversible action that actually succeeded can be re-run by another click. Output is also collected unbounded in memory. This was recorded as "SKIPPED" last round; it is not documented in code.
- Fix: Wait on the child's exit status separately from reading the pipes, and read the pipes with a size cap. Treat exit 0 as success even if the pipes stay open.

### [P2] The failed-attempt counter never decays, so scattered operator typos eventually trigger the 15-minute lockout
- Location: /Users/indo/code/project/omon-gateway/src/discord/pairing.rs:173-190 (`record_failed_attempt_tx`)
- Trigger: An operator mistypes a code once a week. The counter only resets on a successful pairing, or when `attempts >= MAX` and the lock has expired. The fifth typo overall locks pairing for 15 minutes.
- Impact: Spurious lockouts for legitimate operators.
- Fix: Reset the counter when the last failure is older than a window, for example the lockout duration.
- Related: the check `row.attempts >= MAX_FAILED_ATTEMPTS` at pairing.rs:473 is now dead code. Nothing increments `pairing_codes.attempts`, so remove it or document it.

### [P2] Attachment pruning is based on mtime, so a file referenced by a live session can disappear
- Location: /Users/indo/code/project/omon-gateway/src/discord/attachments.rs (`prune_attachment_cache`, called from `with_client`)
- Trigger: A cached attachment is older than 7 days at gateway restart and its `local_path` is still referenced in a long-lived session transcript. A cache hit in `cached_path` never refreshes the mtime.
- Impact: The agent later fails to read a path that existed before the restart.
- Fix: Refresh the mtime on a cache hit, or prune by last-access time.

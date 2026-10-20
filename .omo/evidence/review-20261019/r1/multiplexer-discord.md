# Lane multiplexer-discord

### [P1] Discord replies drop the `[[measure]]` claims they strip, so wrong facts are never re-measured or corrected
- Location: src/discord/adapter.rs:3408
- Trigger: `OMON_REPORT_CLAIMS_ENABLED` is unset (the default is on). The owner asks the Discord lead bot (wawabot 1465631383862120451, the only profile with the autonomy block applied, per .omo/plans/ux-sessions-board.md:39) a factual question. The bot follows bot_profiles/blocks/autonomy.md:18 and appends `[[measure ["wc","-l",...] expect~"..."]]`. The final `OutboundAction::Stream` chunk goes through `PlatformRouter` (main.rs:1464) to `DiscordEgress::stream`.
- Impact: The marker is removed, but the extracted claims are thrown away. No `report_claims` row is written, no re-measure runs and no "정정:" correction is ever posted on Discord. The persona tells the bot that the gateway re-measures and corrects, so a wrong number stays uncorrected without any signal. Slack does this correctly in `spawn_claim_checks` (src/slack/egress.rs:192-196, 270-274).
- Evidence:
  ```rust
  let (claim_stripped, _extracted_claims) = if crate::report::claims::report_claims_enabled()
  {
      crate::report::claims::strip_and_extract_claims(content)
  } else {
      (content.to_string(), Vec::new())
  };
  let filtered_content = filter_reasoning(&claim_stripped);
  ```
- Fix: Keep `extracted_claims`. After `active.throttler.update(&processed_content, true)` succeeds on the final chunk, take the throttler's message id and, if `self.pool` is set, call `crate::report::claims::record_and_schedule_claims(pool, <dispatcher that posts a SendMessage reply_to that id>, session, message_id.to_string(), claims, workspace_root, DEFAULT_REMEASURE_DELAY)`, the same way `SlackEgress::spawn_claim_checks` does.

### [P2] Discord `SendMessage` and `EditMessage` do not strip `[[measure]]` markers
- Location: src/discord/adapter.rs:3622
- Trigger: A lead sends a mid-turn status through `OmoBackend::relay_status` (omo_backend.rs:2025, a `SendMessage`) whose text contains a measure marker. Another path is a recovered obligation or cron text that contains one.
- Impact: Raw `[[measure ...]]` text could reach the Discord user. Slack's `send_message` strips it (src/slack/egress.rs:161-163). The trigger needs the model to put a marker in interim narration rather than in the final reply, so I rated it P2.
- Evidence: `let filtered_content = filter_reasoning(&content);` (the same at 3836 for `EditMessage`), with no claim stripping.
- Fix: Apply `strip_and_extract_claims` before `filter_reasoning` in both arms when `report_claims_enabled()`.

### [P2] Approval-button owner gate uses a hard-coded owner id instead of the configured one
- Location: src/discord/adapter.rs:1388
- Trigger: `OMON_APPROVAL_LEDGER_ENABLED=1` with a `DISCORD_ALLOWED_USERS` first id that differs from `inbox::proposals::OWNER_ID`.
- Impact: The configured owner is refused on every approval button, so each ask times out. `/sessions` uses the env-derived `status_reply::owner_id()` (session_commands.rs:10-12), so the two owner checks disagree. Production uses the matching id today, which is why this is P2.
- Evidence: `let is_discord_owner = user_id.to_string() == crate::inbox::proposals::OWNER_ID;` (the same constant is used at adapter.rs:1618).
- Fix: Compare against `crate::sessions::status_reply::owner_id()` in both places.

### [P2] Status message shows the raw error that `owner_failure_text` deliberately hides
- Location: src/multiplexer/actor.rs:809
- Trigger: An owner-lead turn runs for more than 180 s, so the status message gets posted, and then fails (for example with a quota or timeout error).
- Impact: The status message is edited to `실패 · <raw error, up to 200 chars>`, while the reply posted next to it says the raw error stays in the logs (actor.rs:1446). Internal error text is shown to the owner. The failure reason itself is shown correctly, so this is P2.
- Evidence: `driver.finish(Utc::now(), Err(error.to_string())).await`
- Fix: Pass a short owner-facing reason instead, for example by splitting the reason-mapping out of `owner_failure_text` and passing that.

### [P2] Publish button token makes the custom_id too long for Discord
- Location: src/discord/buttons.rs:106
- Trigger: content-intel builds `omon:btn:content-intel:publish:96-417-435:4.<64 hex>.2`, the form used in the test at buttons.rs:513-517.
- Impact: That id is about 110 characters. Discord rejects component custom_ids over 100 characters, so this token form only works for Slack buttons. Creating the button is outside this repo, so this is P2.
- Evidence: `(1..=128).contains(&segment.len())`
- Fix: Document that the token is Slack-only, or shorten it (for example a truncated sha) so the full id stays at or under 100 characters on Discord.

### [P2] Slack strangers can evict pending Discord pairing codes
- Location: src/discord/pairing.rs:342
- Trigger: Many unpaired Slack users DM the bot. `SlackPairingStore` issues codes into the shared `pairing_codes` table.
- Impact: The global cap (`MAX_PENDING_CODES = 100`) evicts the oldest codes regardless of platform, so a pending Discord user's code can disappear. Lockout is split by platform, but capacity is not.
- Evidence: `sqlx::query_scalar("SELECT COUNT(*) FROM pairing_codes WHERE expires_at > ?")` followed by an eviction `ORDER BY created_at ASC` with no platform or `slack:` prefix filter.
- Fix: Scope the count and eviction by user-key prefix, using `user_id LIKE 'slack:%'` for Slack and `NOT LIKE 'slack:%'` for Discord, passed in from `platform_key`.

The other hunks in scope had no findings: platform_router, the Slack cursor rewind, the deny-preserving `approve_always` and revoke re-check, `request_for_proposal` and the guard resolve helpers, the queued notice, `busy_sessions`, and the `/sessions` commands.

<results>
<files>
- /Users/indo/code/project/omon-gateway/src/discord/adapter.rs - P1 claims discarded on Discord (3408); P2 marker not stripped in SendMessage/EditMessage (3622, 3836); P2 hard-coded owner id (1388, 1618)
- /Users/indo/code/project/omon-gateway/src/multiplexer/actor.rs - P2 raw error in the status message (809)
- /Users/indo/code/project/omon-gateway/src/discord/buttons.rs - P2 token length vs Discord's 100-char custom_id limit (106)
- /Users/indo/code/project/omon-gateway/src/discord/pairing.rs - P2 shared pending-code cap across platforms (342)
- /Users/indo/code/project/omon-gateway/src/slack/egress.rs - reference implementation of claim scheduling (161-196, 290-334)
</files>
<answer>One P1: Discord egress strips measure markers but drops the extracted claims, so on Discord no fact is re-measured and no correction is posted, while Slack does both. Five P2s, listed above.</answer>
<next_steps>Port Slack's spawn_claim_checks into the DiscordEgress::stream final-chunk path. The P2s are optional.</next_steps>
</results>
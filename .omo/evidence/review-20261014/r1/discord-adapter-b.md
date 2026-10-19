# Lane discord-adapter-b

### [P1] Final replies sent through the stream path never clear a dead-target mark, so later replies in a recovered channel are dropped
- Location: src/discord/adapter.rs:3445
- Trigger:
  1. A channel gets marked dead for a bot. For example, a mid-turn `SendMessage` or an `ApprovalRequest` gets 403 50013 while the bot briefly lacks Send Messages. The permission is then restored. The mark is also saved to `dead_targets` and reloaded at boot (main.rs:1047-1052, probe interval 60s).
  2. Production final replies go out as `OutboundAction::Stream` (omo_backend.rs:739 `emit_chunk`, ledger/service.rs:733 recovery). They go through `DiscordEgress::stream`.
  3. At :3370 `is_dead_for_bot` lets one probe through every 60s. That probe sends the reply successfully, but `stream()` never calls `clear_for_bot`. The `clear_for_bot` calls are only in the `dispatch` arms (:3589, :3619, :3718, :3794, :3840, :3883, :4055). The entry therefore stays dead, with `probed_at` set to now.
  4. Any other reply to that bot and channel within the next 60s gets `Err("dead target short-circuited")`.
- Impact:
  - The channel never heals through the main reply path, and the DB row survives restarts.
  - Most replies in a busy recovered channel are lost. omo_backend marks their obligations failed with "dead target short-circuited".
  - `sweep_failed_for_runtime` (ledger/service.rs:637-650) only retries transport-error text, so these replies are never re-sent.
  - Self-healing only happens if some non-stream `SendMessage` or edit happens to succeed.
- Evidence: `active.throttler.update(&processed_content, true).await?;` followed by `*last_sequence = Some(chunk.sequence);`. Nothing on the success path touches `self.dead_targets`. The registry's own doc says "Self-healing: a successful send, probe, or explicit clear removes the channel." Failures in `stream()` also never mark a channel dead.
- Fix: After `throttler.update(..)` succeeds in `stream()`, call `self.dead_targets.clear_for_bot(&identity, channel_id)`. Optionally also mark dead on placeholder or update errors with `is_discord_send_dead_target_error`, the same way the `SendMessage` arm does.

### [P2] Approval content with mentions can exceed Discord's 2000-character limit, so the approval is cancelled
- Location: src/discord/adapter.rs:4175 (budget computed at :4149)
- Trigger: `DISCORD_APPROVAL_MENTIONS=true` with a non-empty `DISCORD_ALLOWED_USERS`, and a dangerous command long enough to be truncated (roughly over 1650 characters, for example a heredoc script).
  - `build_approval_content` fills the message up to `APPROVAL_CONTENT_LIMIT` (2000). It caps the command at `2000 - bytes(prefix+suffix)` characters, so the plain content is about 1996 characters.
  - `build_approval_content_with_mentions` then prepends `"<@id>... \n\n"` (about 23 or more characters per user) with no new budget.
- Impact: Discord rejects the send with 400 Bad Request. The `ApprovalRequest` arm returns Err, and `request_approval` (approval.rs:365-371) returns `ApprovalError::Cancelled`. The command can never be approved, and the user is not shown why.
- Evidence: `let budget = APPROVAL_CONTENT_LIMIT.saturating_sub(prefix.len() + suffix.len());` in `build_approval_content`. Then `format!("{mentions}\n\n{plain_content}")` with no re-truncation.
- Fix: Build the mention prefix first and subtract its length plus 2 from the budget. For example, pass a `reserved` length into `build_approval_content`.

Two smaller things I checked and did not report:
- `replay_failed_transport_obligations` (:3218) has no production caller; only tests/test_discord_adapter.rs:4300 calls it. The missing call is outside this scope.
- The stream path re-uploading media when `throttler.update` fails is already recorded in the earlier rounds.
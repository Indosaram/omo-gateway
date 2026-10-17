# L-commands implementation report

Scope: `src/slack/commands.rs`, `src/discord/pairing.rs`, `docs/slack-app-manifest.json`. No cargo was run, per the lane rules.

## Rows closed

| Audit row | What changed | Where |
|---|---|---|
| Command registry: add `pair` | `COMMANDS` now has 19 entries including `/omo-pair <code>`. Added `SlackCommand::Pair` and a parser that requires the code. | commands.rs:35, :54, :285 |
| Manifest entry `/omo-pair` | Appended to `slash_commands`; the file still parses as JSON (19 commands, re-parsed after the write). The existing `help_lists_every_manifest_command` test now expects 19. | docs/slack-app-manifest.json:127 |
| `/pair <code>` | Operator gate: owner, `allowed_users` or `allow_all_users`; otherwise "not authorized to approve" (ephemeral). Outcomes match Discord: Success is public with `<@U…>`; Invalid, Expired and LockedOut are ephemeral. The owner bypasses admission for `/omo-pair`, as in `command_check`. | commands.rs:484-489, :691 |
| `SlackPairingStore` (contract 11) | `SlackPairingStore::new(pool)` mirrors the `PairingStore` API with `&str` ids: `init_cache`, `is_user_paired(_sync)`, `get_paired_user_ids(_sync)`, `is_platform_locked(_at)`, `record_failed_attempt_at`, `request_pairing_code(_at)`, `check_and_record_notification(_at)`, `record_confirmed_delivery_at`, `approve_code(_at)`. It returns `SlackPairingOutcome` with a `String` user id. Approvals go to `slack_paired_users`, with the operator recorded. Lockout uses platform key `slack`. Pending codes and notification throttling reuse `pairing_codes` / `pairing_notifications`, with user ids stored as `slack:<U…>`. Because of that prefix, Discord `/pair` cannot consume a Slack code (it fails `parse::<u64>`), and Slack cannot consume a Discord code (no prefix, counted as invalid). | pairing.rs:586-830 |
| Code generation, rate-limit and lockout reuse | The private `PairingStore` tx helpers now take the platform key / string user key: `is_platform_locked_tx`, `record_failed_attempt_tx`, `request_pairing_code_tx`. New shared helpers are `reset_failed_attempts_tx`, `claim_notification_tx` and `prune_notifications_tx`. Discord calls them with `DEFAULT_PLATFORM_KEY` and `u64.to_string()`, so the SQL and behavior are the same as before. | pairing.rs:130-275 |
| User admission: allowed OR paired OR `allow_all_users` | `admission_error` + `is_operator`. | commands.rs:484-506 |
| Channel allow/ignore lists (DMs exempt) | `is_channel_admitted`. Ignored channels always reject; an allowlist rejects non-listed channels but not DMs. Rejection text: "This command cannot be used in this channel." | commands.rs:206, :492-500 |
| Skill pending-write operator gate | Paired users get "❌ Only operators can manage pending skill writes." | commands.rs:718 |
| `destructive_slash_confirm` passed through | `SlackCommandDeps.destructive_slash_confirm` (defaults to `true` in `new`) is copied to `poise.destructive_slash_confirm`. L-main sets it from `SlackConfig`. | commands.rs:72, :468 |
| `SlackCommandDeps` gains `pairing` | `pairing: SlackPairingStore` (defaults to `SlackPairingStore::new(pool)` in `new`). | commands.rs:73 |
| `/omo-thread` always persists the thread session | `ensure_session` inserts the same row as `discord::commands::ensure_session`, keyed with `thread_id = root ts`. It runs before any starter message, so ingress `session_exists` accepts unmentioned replies. | commands.rs:419, :928 |
| `/omo-thread` reply links the thread | Uses `api.permalink`: `🧵 Created thread <url|name>.`. If the lookup fails it logs a warning and falls back to `*name*`. | commands.rs:929 |
| `/omo-thread` rejects DMs | Decision: match Discord. Replies with an ephemeral "Threads can only be created in channels, not in DMs." | commands.rs:915 |
| `/omo-title` | With `thread_ts`: reads the root with `conversations_replies(…, 1)`. It renames via `chat.update` (root text `🧵 *<title>*`) only if the root is the bot's `🧵 *` message, i.e. one created by `/omo-thread`; otherwise it replies with an ephemeral refusal. Without `thread_ts`: an ephemeral explanation. | commands.rs:952 |
| Pending-write payload delivery | `pending-writes.json` is sent as a file to the invoking user's DM (`open_im` + `upload_bytes`). The ephemeral reply carries only the count and a pointer, with no inline JSON. | commands.rs:1174-1215 |

## Decision parity notes

- `/omo-thread` in a DM is rejected, as on Discord, even though Slack DMs can technically hold threads.
- `/omo-title` edits the bot's root message rather than a thread object, because Slack threads have no title.
- Pairing operators are the owner, `allowed_users`, and anyone when `allow_all_users` is on. Paired users can use commands but cannot approve codes or manage skill writes. This matches `is_pairing_operator`, `is_user_authorized` and `can_manage_skill_writes`.
- Slack has no roles, so there is no `allowed_roles` equivalent.

## Tests added (not run; the lead's integration build runs them)

- commands.rs:
  - `thread_posts_root_persists_session_and_rejects_nested_and_dm`
  - `title_renames_bot_thread_root_and_explains_otherwise`
  - `admission_honours_pairing_allow_all_and_channel_lists`
  - `pair_rejects_non_operators_and_reports_bad_codes_privately`
  - `destructive_slash_confirm_comes_from_deps`
  - `pending_writes_payload_is_uploaded_to_invoker_dm`
  - `paired_users_cannot_manage_skill_writes`
  - `channel_admission_exempts_dms_and_blocks_ignored`
  - pair parsing assertions
  - The fake Slack server now answers `conversations.open`, `files.getUploadURLExternal`, the upload URL, `chat.getPermalink` and `conversations.replies`.
- pairing.rs: `slack_store_pairs_string_ids_into_its_own_table` and `slack_lockout_is_separate_from_discord`.

## Not closed

- **Ephemeral overflow keeps thread context.** Chunks after the 5 `response_url` posts go through `SlackApi::post_ephemeral(channel, user, text)`, which has no `thread_ts` parameter. `src/slack/api.rs` is outside every lane's write scope. Fix for the lead: add a `thread_ts: Option<&str>` to `post_ephemeral`, then pass `invocation.thread_ts.as_deref()` at commands.rs:1138-1141.
- **Pairing prompt to unknown DM users, and text `/stop` clearing YOLO.** Both live in `ingress.rs` (L-ingress). `SlackPairingStore::check_and_record_notification` is ready for L-ingress to call through `with_pairing`.

## Dependencies on other lanes (compile-time)

- `commands.rs` reads `SlackConfig.allowed_channels`, `ignored_channels` and `allow_all_users`, and the tests use `SlackConfig::test_default()`. These are contract 1, owned by L-main. Until L-main lands them, rust-analyzer reports "no such field".
- `main.rs:1490` builds `SlackCommandDeps` as a struct literal, so L-main must add `destructive_slash_confirm` and `pairing` (contract 11) and call `pairing.init_cache()` at boot.

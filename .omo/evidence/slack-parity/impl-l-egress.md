# L-egress implementation report

Scope: src/slack/egress.rs, src/slack/mrkdwn.rs, src/slack/buttons.rs (new), src/slack/interactive.rs. No compiler was run, per the lane rules; everything here is unverified until the lead's integration build.

## Closed gaps (from adapter-egress.md)

| Gap | Where |
|---|---|
| Mention safety: `<!here>`, `<!channel>`, `<!everyone>` and `<!subteam^…>` are escaped and `<@U…>` is kept. This also covers code spans and the button result replies. | mrkdwn.rs:85 `neutralize_broadcast_mentions`, called at the end of `to_mrkdwn`; buttons.rs `escape` on the reply text; tests mrkdwn.rs `neutralizes_broadcast_mentions_everywhere_but_keeps_user_mentions`, egress.rs:1241 |
| `MEDIA:` directives on SendMessage and on the stream final, using `extract_media_directives` and `validate_media_path_in` with the workspace root and `upload_file`. A Slack failure note `_⚠️ 파일 첨부 실패: …_` is added. Text goes out first and the error is reported after. A stream final is uploaded only once. | egress.rs:158 `send_message`, `stream` final branch, :303 `deliver_media`, :711 `append_slack_media_failure_note` |
| Upload guards: `media_path_is_denied` on the canonical path, a stat-before-read size cap of 100 MB, and a regular-file check | egress.rs:364 `upload_path` (`MAX_UPLOAD_BYTES`) |
| Markdown tables become PNGs via `transform_markdown_tables_to_images` and are uploaded with `api.upload_bytes` in the same thread. If an upload fails, the remaining tables are posted as code blocks via `fallback_text`. | egress.rs:340 `deliver_tables` |
| Runtime footer (`append_runtime_footer`, off by default) | egress.rs:172 |
| Chunk cap: at most 8 messages plus a truncation notice; open fences are closed | mrkdwn.rs:13 `MAX_MESSAGES`, :95 `bound_chunks`; egress.rs:723 `render` |
| reply_to threading: `thread_ts = reply_to` when the session has no thread and the channel is not a DM (applies to SendMessage and Stream) | egress.rs:692 `reply_thread` |
| Approval mentions: `<@U…>` in the fallback text and the first block when enabled | egress.rs:731 `approval_mentions`, `request_approval` |
| Decision label shows `by <@user>` | interactive.rs:83 `decision_label` |
| Dead targets: `channel_not_found`, `is_archived`, `not_in_channel` and `account_inactive` are classified, persisted in `slack_dead_targets`, skipped for the 60s probe interval, and cleared on success | egress.rs:703 `dead_target_reason`, :439 `skip_dead`, :478 `record_outcome` |
| `owner_thread_is_private`: `conversations.info` must report `is_private`, and `conversations.members` must be a subset of {owner, bot} and include the owner. 2s timeout. | egress.rs:622 |
| Generic `omon:btn:` buttons. Reuses `parse_button_custom_id`, `load_button_action`, `run_button_action`, `button_actions_path` and `reply_text`. Has an in-flight lock per channel:ts, dedupe by `action_ts` string (LRU of 500), fail-closed auth via `config.is_allowed`, a pending label, done labels, non-terminal actions, a threaded result reply, a cleared env plus `OMON_BUTTON_PLATFORM=slack`, and a restore on failure. | buttons.rs:94 `handle_button_actions`, :137 `start_click`, :290 `rebuild_blocks` |
| interactive.rs routes non-approval actions to the buttons handler | interactive.rs:20 |
| Contract 8 builders: `with_workspace_root`, `with_runtime_footer`, `with_default_model`, `with_approval_mentions`, `with_pool` | egress.rs:103-128 |

Block Kit has no disabled buttons. A terminal or pending button is therefore removed (link buttons stay) and its label is shown in a context block. A non-terminal done relabels the clicked button in place.

## Not closed (with reason)

- **Typing indicator.** The Slack Web API has no bot typing indicator outside assistant threads (`assistant.threads.setStatus` needs the assistant scope and an AI-app thread). The existing 👀 processing reaction and the live stream preview substitute. `Typing` stays a no-op.
- **Multiple bot identities** (one client per `session.bot_id`). Only one Slack app exists, so this is not needed.
- **Receive watchdog.** This belongs in socket.rs, which is in L-ingress scope.
- **Voice and forum posts.** Slack has no equivalent (N/A in the audit).

## Integration notes for the lead

- `src/slack/mod.rs` (L-main) must declare `pub mod buttons;`.
- The tests use `SlackConfig::test_default()` (contract 1).
- No cargo build or test was run in this lane.

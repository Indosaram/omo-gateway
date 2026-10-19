# Lane discord-commands

### [P1] `/thread` creates a private thread the invoking user cannot open
- Location: src/discord/commands.rs:1247
- Trigger: any authorized user runs `/thread name:foo` (with or without `message`) in a guild text channel.
- Impact: Discord defaults standalone threads to PRIVATE_THREAD when no type is sent. serenity 0.12.5 documents this on `CreateThread::kind`: "defaults to ChannelType::PrivateThread for standalone threads". Only the bot is a member of a private thread created this way. The user is told "🧵 Created thread <#id>", but the link is inaccessible to them unless they hold Manage Threads. The starter message is routed into that invisible session. If the bot lacks the separate Create Private Threads permission, the command fails with 403 instead.
- Evidence: `let builder = serenity::CreateThread::new(name.trim());` followed by `channel_id.create_thread(...)`, with no `.kind(...)` call anywhere in the file.
- Fix: use `serenity::CreateThread::new(name.trim()).kind(serenity::ChannelType::PublicThread)`. A private thread would instead need `add_thread_member` for the invoker.

### [P2] `/steer` and `/deny` echo unbounded user text after the side effect, so the reply fails and the user retries
- Location: src/discord/commands.rs:1051
- Trigger: `/steer text:<more than about 1950 characters>`. Slash string options are not length-limited by the command registration and Discord allows up to 6000. `/deny reason:<long>` has the same shape at :1311.
- Impact: the guidance is already applied or queued. The reply then embeds the full text, Discord rejects content over 2000 characters, and `ctx.send(...).await?` returns Err. The user sees a failed command, resubmits, and the agent gets duplicate steering. For `/deny` the approval is already resolved.
- Evidence: `format!("🎯 Steering guidance applied to the running turn: `{text}`")` is sent after `multiplexer.steer(...)` or `route(...)` has succeeded.
- Fix: pass `text` through `preview_text(&text, 300)` in the reply, and do the same for `reason`.

### [P2] `yolo_toggle` rewrites the whole `state_json` from a stale read and can clobber a concurrent update
- Location: src/discord/commands.rs:1347
- Trigger: `/yolo` while a turn is starting or running. The omo backend writes `$.metadata.omo_thread_id` with an atomic `json_set` (src/agent/omo_backend.rs:2265). That write lands between this function's SELECT and its UPDATE.
- Impact: the UPDATE writes back the pre-image and drops the new `omo_thread_id` binding, so the next turn loses its remote conversation. The window is small. `stop_session` at :896-902 has the same read-modify-write shape, but is less exposed because it only writes when yolo was on.
- Evidence: `let mut state: crate::SessionState = serde_json::from_str(&state_json)?;` then `state.yolo = new_yolo;` then `.bind(serde_json::to_string(&state)?)` (:1347, :1354, :1358). `SessionMultiplexer::set_model` (src/multiplexer/router.rs:599) patches one key with `json_set` for exactly this reason.
- Fix: use `UPDATE sessions SET state_json = json_set(COALESCE(NULLIF(state_json,''),'{}'), '$.yolo', json(?)) ...`, and the same for `$.yolo` in `stop_session`.

### [P2] A skipped button-state edit leaves a terminal button clickable once `IN_FLIGHT` is released
- Location: src/discord/buttons.rs:316
- Trigger: the clicked message has a non-button component (such as a select-menu row), so `rebuild_buttons` returns `None`. It does so at :285 and :287. The same happens if all three edit attempts fail.
- Impact: `set_buttons` returns silently, so neither the pending nor the done state is applied and the original button stays enabled. After the action succeeds, `IN_FLIGHT` is removed at :452, and a second click re-runs the irreversible action (for example publishing twice). The user gets "완료" while the button is still live.
- Evidence: `let Some(rows) = rows else { return };`. There is no refusal before the command is spawned and no persistent "done" record for the message.
- Fix: when `rebuild_buttons` returns `None` at click time, refuse before executing. Alternatively, record terminal-done message ids so the claim step rejects them.
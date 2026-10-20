# Lane sessions-b

I reviewed all six files in the diff (`cli.rs`, `control.rs`, `external.rs`, `mod.rs`, `probe.py`, `probe.rs`) and checked each finding against the current files. The WIP in `src/agent/omo_backend.rs` and `src/slack/egress.rs` is outside this lane and does not affect these paths. Line numbers below are from the current files.

The one P1 finding is about the shape of `omo thread read`. I confirmed that shape from the installed omo-ai CLI source (`~/.bun/install/global/node_modules/omo-ai/bin/lib/thread.js` and `plugin/runtime/thread-sdk/sdk.js`) and from QA output in `.omo/evidence/discord-omo-session-control/qa/`.

### [P1] Adopted sessions stop capturing output once the transcript passes ~60 entries, so `[CLAIM_DONE]` is never seen
- Location: src/sessions/external.rs:262
- Trigger:
  1. Run `omo-gateway sessions adopt omarchy:<id> ...` on a normal live session with 60 or more transcript entries. Real sessions in the QA overview have `message_count` of 150 to 1033.
  2. `adopt` reads the baseline with `read(target, 1, None)`, and `next_cursor` stores the highest `seq` it sees.
  3. In omo, `seq` is not a stable transcript position. `omo thread read` without a cursor runs in tail mode with the default `tail_items` of 60 (the `--limit` flag only slices that window). Each item gets `seq: t+1`, its index inside the returned window. QA shows this: `read local:01a13ef3-c94a --limit 2` on a 564-message session returned seq 59 and 60.
  4. The stored cursor is therefore "60". Every later `read_new` call (`run_lane` -> `drive` -> `read_new`) returns items with seq 1..60, and the `seq > last_seq` filter drops all of them.
  5. A session adopted while small has the same problem as soon as it grows past 60 entries, because the window slides and new items keep seq values of 60 or less. A single working turn with tool results easily adds 60 entries.
- Impact:
  - New assistant messages are never captured.
  - `[CLAIM_DONE] <key> | ...` is never detected, so the external lane stays `working` indefinitely and `lane_transcript` stops updating.
  - Item identity has the same flaw. `assistant_items` uses `seq` as `item_id`, so `external_session_items` and the `activity_item` source (`gen:seq`) treat different messages that land on the same window index as already seen.
  - When the 60-item tail exceeds the default 131072-byte `max_bytes`, the newest items are cut from the page entirely.
- Evidence:
  - external.rs:259-263: `let items: Vec<(String, String)> = if let Some(last_seq) = is_numeric_cursor { items.into_iter().filter(|(id, _)| id.parse::<i64>().map(|seq| seq > last_seq).unwrap_or(true)).collect()`
  - external.rs:554-556: `if let Some(max) = seqs.iter().max() { return Some(max.to_string()); }`
  - external.rs:594 (inside `assistant_items`): `entry.get("seq").and_then(|s| { s.as_i64().map(|seq| seq.to_string())`
  - omo `sdk.js`: `items:d.items.map((e,t)=>({seq:t+1,role:...,content:JSON.stringify(e.content??e)}))`, and the reader's default is `s=t.tail_items??60`.
- Fix:
  - Stop treating `seq` as a cursor or an identity.
  - Minimal change: set the item id to a hash of the item's `content` string (plus role). Remove the numeric `last_seq` filter and rely on the existing `external_session_items` dedup.
  - Store the hash of the newest assistant item as the anchor, and only capture items after that anchor in each tail read.
  - Alternatively, read through `Probe` / `get_messages`, which returns stable message ids.

### [P2] Assistant entries with only a tool call are captured as raw JSON text
- Location: src/sessions/external.rs:626
- Trigger: An external lane's session emits an assistant entry whose content is only `[{"type":"toolCall",...}]`. QA `is3-read-remote-after.txt` seq 7 is an example. `extract_blocks_text` returns `None`, and the code falls through to the raw string.
- Impact: The raw tool-call JSON is stored as an `assistant` row in `lane_transcript`, which the tracker displays, and it is fed to `captured_item`. The comment at external.rs:584 says these entries should be skipped.
- Evidence: external.rs:620-626: `if let Ok(Value::Array(blocks)) = serde_json::from_str::<Value>(text) { if let Some(extracted) = extract_blocks_text(&blocks) { return Some(extracted); } } Some(text.clone())`
- Fix: When the string parses as a JSON array, return `extract_blocks_text(&blocks)` directly, so `None` means skip. Also decode a JSON-encoded string (`Value::String`) before returning it.

### [P2] Probe falls back to running locally when the host name does not parse as a `Target`
- Location: src/sessions/probe.rs:109
- Trigger: A configured host passes `valid_host` (non-empty, no whitespace, no leading `-`) but contains a character outside `[A-Za-z0-9._-@]`, for example `box+1` from `~/.ssh/config`. `overview::collect` then calls `snapshot_all` for that machine.
- Impact: Python runs on the local Mac, and the local sessions are reported under the remote host's label in overview, the watcher and the tracker.
- Evidence: probe.rs:106-110: `let target_str = format!("{machine}:session"); match target_str.parse::<Target>() { Ok(target) => control::transport_argv(&target, &inner_words), Err(_) => inner_words, }`
- Fix: Use local argv only when `machine == "local"`. Otherwise build the ssh argv directly, or return `ProbeError::Transport("invalid machine")`.

### [P2] `probe.py` reports an ambiguous session as "not found", which surfaces as "not running"
- Location: src/sessions/probe.py:202
- Trigger: For interrupt or answer, two endpoints report the same `session_id`, `routing_id` or `name`.
- Impact: The script exits with code 3, `Probe` maps that to `NotFound`, and `ControlError::NotRunning` is shown ("not running - kept 24 h") for a session that is running.
- Evidence: `emit({"ok": False, "error": "session not found" if not matches else "ambiguous session"}, 3)`
- Fix: Exit with code 5 (or a dedicated code) for the ambiguous case, and match only `session_id` / `routing_id`, not `name`.

### [P2] `answers_for` sends the same text to every question in a multi-question request
- Location: src/sessions/probe.rs:264
- Trigger: `sessions answer <t> --text Yes` against an `ask_user_question` call that has two or more questions.
- Impact: Every question gets the same selection or free text. Questions whose options do not include the text get it as free-form input.
- Evidence: `for item in &question.questions { ... map.insert(item.id.clone(), val); }`
- Fix: Reject the call (`Ambiguous`) when `question.questions.len() > 1` unless per-question answers are supplied.

### [P2] Release after the session has ended needs the full id; adopt prints the unresolved target
- Location: src/sessions/cli.rs:221
- Trigger:
  1. `sessions adopt omarchy:01a142d8-fad9 ...` stores the resolved full id (QA `is10-db-row.txt`).
  2. The session ends.
  3. `sessions release omarchy:01a142d8-fad9`: `resolve` fails, the code falls back to the prefix, and the release query `WHERE external_target=?` finds nothing.
- Impact: "session is not adopted", and the user has to type the full id. In the same way, the adopt output at cli.rs:214 prints the prefix rather than the stored target.
- Evidence: cli.rs:219-222: `let target = match control.resolve(&target).await { Ok((resolved, _)) => resolved, Err(_) => target, };`, and cli.rs:214: `json!({"key":result?,"target":target.to_string(),...})`
- Fix:
  - In `release`, fall back to `SELECT key FROM lanes WHERE external_target LIKE ?||'%'` when there is exactly one match.
  - Return the resolved target from `adopt` and print that.

### [P2] Every `read` and `send` now runs an extra uncached `omo thread list`
- Location: src/sessions/control.rs:391
- Trigger: Each external lane step (`send_goal`, `read_new`, `send_pending`) and each Discord read.
- Impact: One extra `omo thread list --all-scope` (ssh for remote hosts) per call, on a 20 s budget. `adopt` lists twice: `resolve`, then `session()`. This adds latency but nothing breaks.
- Evidence: `let (resolved_target, _session) = self.resolve(target).await?;`
- Fix: Skip `resolve` when the target is already a canonical id (for example, stored lane targets), or reuse the `Session` from `resolve` in `adopt` instead of calling `session()`.

<results>
<files>
- /Users/indo/code/project/omon-gateway/src/sessions/external.rs - P1: seq is used as cursor and identity (lines 243-263, 535-566, 594); P2: tool-call-only entries captured as JSON text (626)
- /Users/indo/code/project/omon-gateway/src/sessions/probe.rs - P2: local fallback in `argv` (106-110); P2: `answers_for` with several questions (248-264)
- /Users/indo/code/project/omon-gateway/src/sessions/probe.py - P2: ambiguous session reported as not found, exit 3 (198-203)
- /Users/indo/code/project/omon-gateway/src/sessions/cli.rs - P2: release fallback and adopt output (214, 219-226)
- /Users/indo/code/project/omon-gateway/src/sessions/control.rs - P2: extra listing on every read/send (391, 417)
- /Users/indo/.bun/install/global/node_modules/omo-ai/plugin/runtime/thread-sdk/sdk.js - shows that `seq` is `t+1` within a 60-item tail window
- /Users/indo/code/project/omon-gateway/.omo/evidence/discord-omo-session-control/qa/is2-read-other-workspace.txt - QA output: `--limit 2` on a 564-message session returns seq 59 and 60
</files>
<answer>The report above covers every hunk in sessions-b.diff. There is one P1: external-lane capture silently stops once a transcript passes about 60 entries, so `[CLAIM_DONE]` is never seen and the lane stays `working`. The cause is that omo's `seq` is an index within a 60-item tail window, not a stable position. There are six P2s. There are no P0s, and `mod.rs` is fine.</answer>
<next_steps>Fix the P1 in `external.rs` by deduplicating on a content-hash id with an anchor instead of the numeric seq filter. The P2s are optional.</next_steps>
</results>
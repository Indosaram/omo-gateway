# lanes-signals review

### [P1] done_diff drops the whole deliverable when one early file exceeds the 256 KiB budget
- Location: /Users/indo/code/project/omon-gateway/src/lanes/settle.rs:315
- Trigger: A lane changes a large file whose old+new size exceeds `MAX_DIFF` (256 KiB). An example is `package-lock.json` after `npm install`, or a large `Cargo.lock`. `git diff --cached --name-only -z` lists paths bytewise, so `Cargo.lock` and `package-lock.json` come before `src/...`. The loop `break`s on the first over-budget file, so every later file is dropped as well.
- Impact: `changes` is empty or missing the real deliverable. The temp repo then has an empty or partial diff. For an empty diff, jev returns `fulfills=0.0, risk=1.0`, which trips `fulfills < 0.65`. The lane is bounced to `not_done` with a `done_diff fulfills=0.000...` objection that the agent cannot fix by working harder. The `break` also drops small later files in the same listing.
- Fix: Use `continue` instead of `break`, skipping only the oversize file. Note truncation in the detail. Stat the file size before `tokio::fs::read`. Consider treating "diff truncated" as `skipped`, not as a rejection.

### [P1] Kibitzer's judge input is cut to 4000 chars, so the Home DM section (placed last) is usually lost
- Location: /Users/indo/code/project/omon-gateway/src/lanes/kibitzer.rs:245
- Trigger: `state_document` (kibitzer.rs:356-395) builds a document of up to 6000 chars: open lanes, 10 events, memories, then "Home DM messages" last. `evaluate` passes it through `scrub_for_external` (kibitzer.rs:250), which caps at 4000 chars (/Users/indo/code/project/omon-gateway/src/judge/mod.rs:20-22). `JevCli::noul` (judge/mod.rs:1189) and the fallback (judge/mod.rs:512) cap again. Any state where lanes, events and memories take more than about 3900 chars loses the owner's recent DMs. Events can be about 2000 chars each, so this is reachable.
- Impact: The wake judge scores only lane and event text and never sees the owner's messages. The generation is already consumed (kibitzer.rs:238), so the wake is silently lost.
- Fix: Build the document within the judge's 4000-char cap. Put the DM section first, or budget it separately. Or pass the judge a purpose-built shorter state and keep the 6000-char document only for the model turn.

### [P2] `arm_owner` has no production caller
- Location: /Users/indo/code/project/omon-gateway/src/lanes/kibitzer.rs:443
- Trigger: A repo-wide grep finds `arm_owner` only at its definition and in `tests/test_lane_kibitzer.rs`.
- Impact: Owner DMs never arm a generation, so only `inbox_items` and `lane_events` triggers fire. If it is wired later while the kibitzer is disabled, `kibitzer_state` won't exist (it is created only in `initialize`, which `main.rs` skips when disabled). The UPDATE would then fail inside the owner's transcript transaction.
- Fix: Wire it into the owner-message commit path, or make it a no-op when the table is absent. Create the table unconditionally.

### [P2] A failed evaluation after reservation loses the generation, and the error stops the whole lane service
- Location: /Users/indo/code/project/omon-gateway/src/lanes/kibitzer.rs:238
- Trigger: After `consumed = generation` is committed, `state_document().await?` (line 245) or any `restore_admitted(..).await?` can return Err. SQLITE_BUSY after the 5s busy timeout is one cause. A corrupt `metadata_json` row makes `MemoryStore::search` fail and is another.
- Impact: Consumed is never restored, so that wake is lost. `serve` propagates the Err (kibitzer.rs:410). The JoinSet then reports `lane_service_exit`, which tears down the gateway (`main.rs` `lanes.join_next()` and `lane_tasks.join_next()` arms).
- Fix: In `evaluate`, handle errors after reservation by calling `restore_admitted`. Log and continue in `serve` for non-fatal DB errors.

### [P2] Failure detail is stored unscrubbed in `objection`, `lane_history` and `lane_inbox`
- Location: /Users/indo/code/project/omon-gateway/src/lanes/settle.rs:141
- Trigger: A `command` signal fails. Its detail includes up to 4 KiB each of stdout and stderr (signals.rs:1195-1201), for example env dumps or auth errors.
- Impact: Only the `lane_events` body is scrubbed (settle.rs:154). `lanes.objection` (line 141), the `lane_history.reason` rows and the `lane_inbox` text (line 160) hold raw output. The history and objection rows persist, so a leaked secret is retained in the DB.
- Fix: Scrub `detail` once and use that value for all sinks. Cap the length, as the other lane writers do (`.chars().take(2000)`).

### [P2] `now` is read once per `settle_due` pass and reused after long signal runs
- Location: /Users/indo/code/project/omon-gateway/src/lanes/settle.rs:49
- Trigger: Each lane's signals can take up to 120s each, run sequentially (signals.rs:153-158). By the time a later lane commits, `now` can be minutes stale.
- Impact: `updated_at` and history `at` are written with a stale time. They can move `updated_at` backwards relative to concurrent activity, which `guard.rs` uses for change detection.
- Fix: Re-read `self.clock.now()` before the final transaction.

### [P2] Worktree verification fails closed on stale registrations and attributes sibling lanes' worktrees to this lane
- Location: /Users/indo/code/project/omon-gateway/src/lanes/settle.rs:468
- Trigger 1: The repo has a registered but deleted (prunable) worktree. `external_creation_time` fails on metadata (line 523), or `git status` fails for a path under cwd. `worktree_reasons` returns Err, which becomes a not_done reason, so the lane cannot verify until someone runs `git worktree prune`.
- Trigger 2: Lane A's cwd is the main checkout and lane B creates a worktree later in the same repo. B's worktree has a creation time at or after A's start (line 518). A's verification is then blocked by B's uncommitted or unpushed state.
- Impact: The lane is stuck at not_done until the stale registration is pruned or the other lane finishes.
- Fix: Skip worktrees whose path is missing. Scope included worktrees through `lane_owned_worktrees` rather than a creation-time heuristic.

### [P2] done_diff reads whole files into memory before applying the budget
- Location: /Users/indo/code/project/omon-gateway/src/lanes/settle.rs:310
- Trigger: A lane leaves a large untracked, non-ignored artifact in cwd, for example a multi-GB build output or core dump. `git add -A` hashes it within 30s, then `tokio::fs::read` loads it fully. `git show` for the old side does the same.
- Impact: High memory use or OOM in the gateway process.
- Fix: Check `metadata().len()` and the `git cat-file -s` size against the budget first.

### [P2] `deliverable()` substring filter can drop real requirements
- Location: /Users/indo/code/project/omon-gateway/src/lanes/settle.rs:371
- Trigger: A brief sentence contains "model", "service", "report", "claim", "commit" or "production" as a substring. Examples are "Add a service module", "Fix the reporting bug" and "Update the models".
- Impact: The judge scores the diff against a truncated deliverable. This can cause a false rejection.
- Fix: Match whole words or anchor to known operating-rule phrases. Better, have the lane brief mark the deliverable explicitly.

### [P2] Command signals run with the full gateway environment, and their output is unbounded in memory
- Location: /Users/indo/code/project/omon-gateway/src/lanes/signals.rs:1136
- Trigger: A `command` signal such as `cargo test` or `make` runs lane-authored code (build scripts, tests) with the gateway's env and privileges. `wait_with_output` (line 1170) buffers all stdout and stderr until exit or timeout. A command printing GBs within 120s can exhaust memory.
- Impact: Gateway secrets (Discord and judge tokens) are visible to lane-authored code. Failure output can echo them into detail (see the scrub finding above). Large output can cause OOM.
- Fix: Use `env_clear` plus an allowlist, and read stdout and stderr with a byte cap. Likewise `tokio::fs::read` in the File signal (signals.rs:1262-ish) is unbounded.

### [P2] Semicolon splitter treats an unbalanced `'` or `[` outside quotes as opening a span
- Location: /Users/indo/code/project/omon-gateway/src/lanes/signals.rs:225
- Trigger: `file it's.txt; command [...]` or a path containing an unbalanced `[`. The splitter treats `'` as a quote opener and `[` as increasing depth, so the later `;` is swallowed.
- Impact: Multiple signals merge into one segment. The result is a confusing parse error or a mis-parse.
- Fix: Treat only `"` as a quote in the splitter, since the grammar uses JSON strings. Tolerate unbalanced brackets in file paths.

### [P2] Kibitzer reply parsing has no fence tolerance, so retries burn model turns
- Location: /Users/indo/code/project/omon-gateway/src/lanes/kibitzer.rs:55
- Trigger: The model wraps its JSON in a ```json fence. `parse_decision` fails and `restore_admitted` re-arms the generation. A turn is retried every 30 minutes (SPACING) with no cap on failed turns.
- Impact: Repeated model spend and no decision. `judge/mod.rs` already has `parse_json_loose` for this case.
- Fix: Strip the fence before parsing, and cap consecutive parse failures per day.

### [P2] scrub gaps: unterminated private key, `file://` paths, dotted usernames
- Location: /Users/indo/code/project/omon-gateway/src/lanes/scrub.rs:41
- Trigger 1: The private-key regex needs both BEGIN and END markers. Output truncated by `output_tail` (4 KiB) or a 2000-char cap, applied before scrubbing, can leave a partial key unredacted.
- Trigger 2: The home-dir regex's leading-char class (scrub.rs:9) excludes `/`. A path like `file:///Users/bob/x` is not rewritten. `[a-zA-Z0-9_\-]+` also excludes `.`, so `/Users/first.last/x` becomes `~.last/x`.
- Impact: Partial key material, or another user's path or name, can reach lane events, Discord and judge input. The gateway user's own HOME is covered by the exact replacement (scrub.rs:102-106).
- Fix: Redact from a BEGIN marker to end of text when no END marker follows. Include `/` and `.` handling in the home-dir regexes.

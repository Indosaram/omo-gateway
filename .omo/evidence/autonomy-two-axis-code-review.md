# Code review: autonomy-two-axis (`git diff f7e6c41..cb388f0`)

Reviewer: omo-native-code-reviewer (st_01a14f7c). This was a read-only review: no cargo commands were run and no source files were edited.
Plan: /Users/indo/code/project/omon-gateway/.omo/plans/autonomy-two-axis.md
Line numbers refer to `cb388f0`. HEAD has since moved to 8e888ae (3 more commits). Where a later commit changes a finding, the finding says so.
Report path: `agentToolkit.status().currentAttemptDir` returned undefined, so this report uses the `.omo/evidence/<goal>-code-review.md` fallback.

Skill-perspective check: the `remove-ai-slops` and `programming` skills are not installed in this session, so I applied the criteria given in the task prompt. Two findings come from that check (M4: tests that exercise an unwired path; M10: a spike-only path hardcoded in production code).

## CRITICAL

**C1. The classifier reads only the first token, so compound commands auto-allow as `read`.**
- Where: src/autonomy/effect.rs:275-306 (`classify_command` tokenizes the whole body, and `classify_tokens` at :308-312 switches on `tokens[0]`), together with src/autonomy/ledger.rs (~:340, `Effect::Read` returns Allow with no proposal row).
- How it fails:
  - With OMON_APPROVAL_LEDGER_ENABLED, the engine sends `ls && rm -rf ~/code/x` (or `git status; git push --force origin main`, or `cat f | sh`). It is classified `read` from `ls`/`git status`/`cat`, auto-allowed with no card, and leaves no ledger row.
  - The same hole applies to `bash -c "ls; rm ..."`, because the code recurses into `classify_command` on the same first-token logic.
  - Single-command bypasses also exist. `git -c core.fsmonitor='rm -rf ~' status` is read, because `-c` values are skipped (:495). `rg --pre sh x` is also read.
- Fix: split on `;`, `&&`, `||`, `|`, newlines, `$(..)` and backticks, classify every segment, and take the most consequential effect (HEAD's `Effect::rank` can do this). Treat any unparsed shell metacharacter as `unknown`. Treat `git -c` / `--config-env` / `--pre` as unknown.

**C2. Bare `Tool: eval` frames are auto-allowed before the ledger runs.**
- Where: src/agent/omo_backend.rs:1557.
- How it fails: the spike evidence (task-0.md, fixture `agents-auto-eval_write.json`) shows that an eval cell writes files with only the bare frame. Commands run through Bun's `$` shell, and JS `fs` / `fetch` / `Bun.spawn`, raise no nested frame. So any eval cell runs destructive, external-send or spend effects with no owner and no ledger row. This breaks the plan's "No model-written command runs outside the ledger."
- After `cb388f0`: HEAD 8e888ae only partly fixes this. It regex-scans `$\`..\`` and `command: "..."` strings, and still auto-allows when neither appears (`Bun.spawn`, `fs.rmSync`, `fetch(POST)`, or a string built from variables).
- Fix: treat a bare eval frame with no positively-read classification as `unknown`, which asks or uses an exact precedent. Do not auto-allow it.

## HIGH

**H1. A ledger failure fails open to the legacy auto-approve.**
- Where: src/agent/omo_backend.rs:1572 (the `if let Some(pool)` path has no else) and :1729 (`Err(error) => warn`), both falling through to :1747 `auto_approve`.
- How it fails: an SQLite error on the single-connection pool (busy or acquire timeout), or a backend that has the ledger on but no pool, falls through to the old path. That path allows every frame unless APPROVAL_MODE=always, so a `vcs_protected` / `delete` frame runs without the owner.
- Fix: when the ledger is enabled, deny any ledger error or missing pool with a reason, and `continue`. Never reach the legacy auto-approve.

**H2. Non-owners and the model can answer a pending approval with a short reply.**
- Where:
  - src/multiplexer/actor.rs:626-647 passes `next.author_id` as the `bot_name` argument of `is_owner_lead_session`, so the author is never checked.
  - src/agent/omo_backend.rs:2573 (`steer` calls `try_resolve_short_reply` with no author at all), reached from slack/ingress.rs:221 and discord/adapter.rs:1618.
  - src/lanes/runner.rs:1762, which steers lane_inbox text into the same path.
- How it fails:
  - In a Slack home channel or a Discord guild session, the session key has no user component, so the session is shared. Any allowed or paired user (or an allowed bot) who types `항상` resolves the open card as Always and creates an exact allow precedent.
  - The lead model's `omo-gateway lane send <key> 응` writes to lane_inbox. The runner steers it, and `try_resolve_short_reply` resolves the lane's DB-pending proposal through `decide_proposal`, which calls `guard.resolve_proposal_id`. That is a self-approval.
- Fix: pass the event author into the short-reply path and require author == owner (Slack `owner_id()` / Discord `OWNER_ID`). Never resolve from `LaneRunner` steers or any non-`EventOrigin::User` text.

**H3. Exact precedents are far broader than the case the owner approved.**
- Where: src/autonomy/effect.rs:92 (signature = effect:program:target, with flags and args dropped), :323-328 (the `rm` target is a directory), and :603-635 (all protected pushes share the signature `vcs_protected:git push:<repo>`).
- How it fails:
  - The owner clicks Always on `rm notes.txt` in a repo. A later `rm -rf .` gets the same signature (`delete:rm:<cwd>`) and is auto-allowed.
  - Always on `git push origin main` auto-allows `git push --force origin main`, `--delete` and `--mirror`.
  - Always on one `gh pr merge` auto-allows merging any PR in that repo.
  - The plan says "deletes and merges never become automatic beyond the exact case you approved."
- Fix: for `delete`, `vcs_protected` and `external_send`, include the normalized full argv (sorted flags plus positionals or refspec) in the signature. Keep `program:target` only for the lower-risk effects.

**H4. The owner gate on `autonomy decide` / `revoke` does nothing.**
- Where: src/autonomy/mod.rs:52-65.
- How it fails:
  - `OMON_AUTONOMY_CALLER_SESSION` is never set anywhere in src, so every invocation counts as the "local CLI", including lane, cron, and prompt-injected lead turns.
  - Unknown programs sign as `unknown:<basename>:<cwd>`. So once the owner clicks Always on any `omo-gateway ...` call (for example `omo-gateway lane pause`, which persona rule 7 asks for routinely), the engine can run `omo-gateway autonomy decide <id> allow-always` and approve itself.
  - At `cb388f0` this also works through C2 with no card at all.
- Fix: set `OMON_AUTONOMY_CALLER_SESSION` in the engine's process environment for every turn and treat an unset value as "not owner" when called under the gateway. Also include the subcommand in signatures for `omo-gateway`.

**H5. The "tainted" fixed owner rule is never applied to engine actions.**
- Where: src/agent/omo_backend.rs:1596 (`tainted: false` hardcoded).
- How it fails: an `[ACTION_LOOP]` or inbox-driven turn carries other people's text. An injected instruction then reaches `external_send` / `delete` / `vcs_protected` frames that match an exact precedent (made broad by H3) and auto-allows them. The plan says "Tainted + (external_send | delete | spend | vcs_protected) always asks" and "messages from other people never turn into actions without your answer."
- Fix: mark the turn tainted when its event is an action_loop, inbox or GitHub delivery, or when the context pack has other-author content, and pass that into `PreAskInput`.

**H6. With the flag on, cron and lane turns deny every non-read action without posting a card.**
- Where: src/agent/omo_backend.rs:1711 (the no-requester branch expires and declines), src/lanes/runner.rs:1307 and src/main.rs:1548 (both backends are built without `with_approval_requester`), and `is_ledger_enabled()`, which applies process-wide.
- How it fails: after todo 14 sets OMON_APPROVAL_LEDGER_ENABLED=true, every lane or cron frame that is not `read` (including `draft` writes) is declined with "owner has not answered yet". The owner never sees it. Lanes stall, and plan todo 7(i) ("engine approval frames in lead and lane turns") is not met.
- Fix: give the lane backend the shared requester, with ask_session = lead, as the code already computes. Exempt cron and other headless backends, or route them the same way. Do not silently decline.

## MEDIUM

- **M1 Quiet-hour proposals stay `pending` forever** (src/autonomy/ledger.rs:420-466). A row is inserted as pending, then Deny is returned, and the row is never moved to `expired`.
  - Two of these rows make `db_pending.len() > 1`, which disables short replies for that session until restart.
  - A morning "응" marks the stale row `approved` (and counts it toward the candidate) for an action that never ran.
  - Fix: set state `expired` (or a held state) when declining for quiet hours.
- **M2 Restart recovery expires inbox-origin rows and tells the engine to re-issue them** (src/autonomy/ledger.rs:615-670; main.rs:1255-1258 backfills first, then recovers). Every pending inbox draft becomes `expired` on the first start, and the lead gets "re-issue" instructions for other people's drafts.
  - Fix: limit recovery to `origin IN ('engine_approval','tool_approval')`.
- **M3 The kill switch no longer gates inbox auto-lookup** (src/inbox/triage.rs:444). The OMON_INBOX_AUTO_LOOKUP opt-in was removed with no replacement gate. With the ledger flag off (the default), the gateway now auto-fetches links from other people's messages.
  - Fix: gate this on `approval_ledger_enabled()`.
- **M4 `Proposals::evaluate_with_ledger` has no production caller** (src/inbox/proposals.rs:601). Only tests/test_inbox_ledger.rs calls it. Todo 11(b)/(c) (Draft card, precedent auto-run) is not wired, and those tests exercise dead code (slop check).
- **M5 The short-reply steer runs inline on the actor loop** (src/multiplexer/actor.rs:637). This contradicts the off-loop steer rule at :710. While it runs, the turn stream is not drained, which adds a DB round trip on the 1-connection pool plus a socket round trip of up to 5 s.
  - Fix: spawn it, as the Steer command does.
- **M6 The ask wait is not deducted from deadlines while it is pending** (src/agent/omo_backend.rs:1346 only adds the wait after the answer). A lane turn with a fixed deadline is interrupted mid-ask.
  - Separately, the ask timeout is APPROVAL_TIMEOUT_SECS, default 900 s (src/main.rs:3454), not the 30 minutes the plan specifies.
- **M7 Revoking a precedent does not invalidate the guard's in-memory `always_cache`** (src/discord/approval.rs:527). `approve_always`'s upsert (:606-620) also overwrites a `deny` precedent with the same signature to `allow`.
- **M8 Report claims can run more than read-only commands, and Discord leaks markers.**
  - Claims run anything the "read" classifier accepts (src/report/claims.rs:351): `cat ~/.ssh/id_rsa`, `rg --pre`, `git -c ...`, `cargo test` (which runs build scripts). On a mismatch, up to 300 chars of stdout are posted.
  - The Discord stream strips markers per chunk without the buffered filter, and it drops the claims (src/discord/adapter.rs:3397). A marker split across chunks reaches the user.
- **M10 Spike-only path hardcoded in production code** (src/autonomy/effect.rs:786: `/tmp/omon-approval-spike`). Fixture knowledge leaked into the classifier; remove it.

## LOW

- **L1** The candidate check uses `count != 3` (src/autonomy/ledger.rs, `record_once_and_check_candidate`). Yolo rows (state `approved`, precedent NULL) inflate the count, so the candidate can be skipped forever.
- **L2** Slack approval clicks are now owner-only (src/slack/interactive.rs:40), and the actor short-reply intercept is active even with the ledger flag off. Both changes leak behavior outside the kill switch.
- **L3** Yolo allows `spend` (ledger.rs:318), which conflicts with IS-9 "payments always ask" (although the plan's own ordering puts yolo first).
- **L4** Rollback: the old binary's sqlx migrator will fail with VersionMissing once 0046-0052 are applied (src/storage/db.rs:190 has no `ignore_missing`). This is a pre-existing pattern, but "approval_allowlist kept for rollback" does not help.

## Migrations against the prod DB

- 0046-0050 and 0052 are additive.
- 0049 adds a column with ALTER and copies the legacy allowlist with INSERT OR IGNORE (safe; the legacy rows get effect `unknown` and never match new signatures).
- 0051 copies the rebuild pattern of 0045, which already ran in production (`defer_foreign_keys`, temp-table copy, same columns). I found no data-loss risk.
- No existing migration file was modified.

## Verdict

codeQualityStatus: BLOCK
recommendation: REQUEST_CHANGES
Blockers: C1, C2, H1, H2, H3, H4, H5, H6.

VERDICT: REJECT

# Lane inbox-migrate-voice

### [P2] `migrate` preflight ignores `OMON_HERMES_PROFILES`, so it can abort after config import already ran
- Location: src/migrate/mod.rs:187
- Trigger: `OMON_HERMES_PROFILES=work` is set, and a store outside the selection (for example `default`) still has jobs in its `jobs.json`. The user runs a real `migrate` without `--no-cutover`.
- Impact: The import step syncs only the selected profiles (mod.rs:149-150), and config import has already written its output by then. The cutover preflight then scans every Hermes store (cron_cutover.rs:616). It finds the unimported jobs in the non-selected store and fails with "has not been imported". The user is left with a partial migration, and the Hermes gateway stays up. The dry-run path (`project_migration`) reports non-selected stores as `cron_nonselected` and never errors. So the dry-run says the migration will succeed and the real run fails.
- Evidence:
  - src/migrate/mod.rs:149 `let stores = discover_hermes_stores(env, &paths.hermes_root)?;` is filtered by `selected_profiles()`.
  - src/migrate/mod.rs:187 `let preflight = cutover_cron_stores(env, &paths.hermes_root, &pool, true)` goes through `discover_stores` (src/migrate/cron_cutover.rs:616), which always starts at src/migrate/cron_cutover.rs:617 `let mut stores = vec![(` with `default` plus every directory under `profiles/`. It has no selection filter.
  - I traced this by reading the code; nothing was run.
  - tests/test_migrate.rs only covers the dry-run and `no_cutover:true` cases.
- Fix: Pass the selection into `cutover_cron_stores`/`discover_stores` and filter with `is_profile_selected`, so the preflight and the real cutover cover the same stores the import step synced. Alternatively, make the dry-run apply the same check and report the failure.

### [P2] Triage re-runs every jev judge call when the writer step fails, and an Owner/Bot item the writer omits is re-judged forever
- Location: src/inbox/triage.rs:290
- Trigger:
  - (a) For a batch containing Owner/Bot items, `self.writer.write` (triage.rs:290) times out, errors, or returns invalid JSON. `classify` returns `Err`.
  - (b) The writer returns a valid array that omits an id.
- Impact:
  - (a) Owner/Bot jev results exist only in memory. Fyi/Ignore results are committed before the writer runs, but Owner/Bot results are not. After `Err`, `flush` leaves those items without a decision, so the next flush calls `judge.choice` again for every undecided item. `recovery_remaining` resets to 3 on every new insert (triage.rs:176). Repeated writer failures therefore burn the jev daily cap. Once the cap is hit, `is_capped` makes every later classification return Err (triage.rs:229), so classification stops until the cap resets.
  - (b) Unknown ids only get a warning (triage.rs:314), and nothing checks that every Owner/Bot item got a decision. At triage.rs:327 `let Some(row) = row else {` skips the item with no decision row. It stays undecided and is judged again on every flush. The writer step never fails in this case, so the loop has no end.
- Evidence:
  - triage.rs:225 `.choice(Purpose::InboxClass, &text, INSTRUCTIONS, OPTIONS)` runs per item before any Owner/Bot row is persisted.
  - triage.rs:229 `if choice.is_capped || choice.is_fallback {` returns `Err`.
  - triage.rs:290 `let raw = self.writer.write(&writing).await?;` aborts the whole function.
  - triage.rs:314 `"unknown inbox triage response id ignored"` is the only handling of an id mismatch.
  - The cap size (300 per day) comes from my earlier read of `production_judge`, which I did not re-open for this report.
- Fix: Persist the Owner/Bot class (with a placeholder reason and `proposed_action = NULL`) in the same early transaction as Fyi/Ignore, and fill in reason and action only when the writer succeeds. Also fail or store a fallback decision for any writing item whose id is absent from the writer response.

### [P2] A mail thread refresh leaves a stale `inbox_proposals` row, so the re-triaged bot item can never be approved
- Location: src/inbox/mod.rs:249
- Trigger:
  1. A mail thread is triaged as class `bot` with a proposed action, and a proposal row is created.
  2. The owner runs `inbox set <id> --status done` (or dismisses it).
  3. A new message arrives in the same thread. `commit_source` refreshes the item and deletes its decision, because status is `done` or `dismissed`.
  4. Triage re-classifies it and the writer produces a different `proposed_action`.
- Impact: The old `inbox_proposals` row is never deleted or updated. `propose_in` uses `ON CONFLICT(item_id) DO NOTHING` (proposals.rs:360), so the stale proposal row (old `proposed_action`/`title`/`link`) is kept. `pending()` joins on `d.proposed_action=p.proposed_action` (proposals.rs:288), and the approval claim has the same check (proposals.rs:447). With a different action, neither matches. The new proposal is never listed. `inbox set --status approved` takes the `AlreadyClaimed` path, which returns Ok.
- Evidence:
  - src/inbox/mod.rs:249 `"DELETE FROM inbox_decisions WHERE item_id = ? \` followed by `status IN ('open', 'done', 'dismissed')` removes the decision.
  - src/inbox/mod.rs only touches `inbox_decisions` there, so no `inbox_proposals` row is removed.
  - proposals.rs:360 `ON CONFLICT(item_id) DO NOTHING` keeps the old proposal.
  - proposals.rs:447 `AND d.proposed_action=inbox_proposals.proposed_action AND i.link IS inbox_proposals.link)` fails on the changed action.
- Fix: In the refresh transaction, also delete `inbox_proposals WHERE item_id=? AND state='pending'` when the decision is deleted. Or make `propose_in` update `proposed_action`/`link`/`title`/`action_json` on conflict while the proposal is still `pending`.

### [P2] `inbox set --status approved` exits successfully when nothing was executed
- Location: src/inbox/set.rs:62
- Trigger: A proposal exists but its claim fails. That means it is already `running`, `completed` or `failed`, or the stale-proposal case above. The owner runs `inbox set <id> --status approved`.
- Impact: `approve_item` returns `AlreadyClaimed` without running the action. The CLI treats that as success and exits 0 with no output. The caller cannot tell that the approval did not execute. The `NotMatched` branch of the same match returns an error ("approval did not execute"), so the two no-op outcomes are handled inconsistently.
- Evidence:
  - src/inbox/set.rs:60 `match proposals.approve_item(id, actions).await? {`
  - src/inbox/set.rs:62 `| super::proposals::ApprovalResult::AlreadyClaimed => return Ok(()),` is grouped with `Executed`.
  - proposals.rs:449-450 returns `AlreadyClaimed` when `rows_affected() != 1`.
- Fix: Return a `SetError::Approval` (for example "proposal already claimed or no longer matches; nothing executed") for `AlreadyClaimed`, and keep `Ok(())` only for `Executed`.
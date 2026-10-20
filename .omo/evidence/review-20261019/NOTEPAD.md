# review-20261019 notepad (durable copy; tmp notepad was truncated by restore)
## Criterion 1 (R1 reports + CONFIRMED)
- r1/: 26/26 harvested, including the four recovered lanes. r1-citations.md resolves 135 Location references to current file excerpts; semantic verification is tracked separately.
- CONFIRMED-r1.md: F1-F13 (wave 1), F14-F36 + regrades R-A/R-B (wave 2).
## Criterion 2 (fixes)
- All confirmed P1 fixed in working tree (lead + quick lanes). F29/F30 tracker lanes SYNC_LOCK landed (st_01a154b3), lead-reviewed.
- Delta re-review: four reports returned; cron/deploy, web/tracker and Slack/sessions report VERIFIED for scoped fixes. Runtime initially reported VERIFIED, but the lead rejected F39's cap scheduling claim: reinsertion into pending_batch prevents added > 0 and timer rearming. Removed reinsertion and added same-worker collection assertion; runtime follow-up remains pending.
## Criterion 3 (gate): baseline run0 PASS (web-test 4 preexisting '@' alias failures, identical on e57b472). run1 pending.
## Criterion 4 (push): pending. ## Criterion 5 (deploy): pending.
## Concurrent workspace changes observed before shipping
- `scripts/gateway_watchdog.sh` and `tests/test_gateway_watchdog.sh` acquired changes after the review baseline: removal of the subscription-patch check. These were not authored in this review and must not be reverted or silently included without resolving ownership.
- `.omo/permissions-approved.jsonl` was already deleted at baseline. `.senpi/` and `wt/` are untracked local paths. Deployment records all porcelain paths, so the dirty_paths=0 requirement remains unresolved until these are handled without discarding other work.
- Generated tracked `web/dist/` assets changed during gate builds and belong with the matching web source build.

## Shipping reconciliation
- The ownership question timed out; no other changes were reverted. Under the requested full-WIP commit scope, preserve the watchdog patch-check removal and existing permission-record deletion. Watchdog's isolated test exited 0 (`test_gateway_watchdog: ok`).
- Local-only `.senpi/` HTML output and the 4.1 GB nested `wt/` worktrees are preserved and excluded only through `.git/info/exclude`; no source changes are hidden.
- Latest browser run against `target/debug/omo-gateway` and latest web/dist: 21/21 PASS, including original missing conditions (same-column manual reorder, multi-label source identity, same-project milestone retention, non-alphabetical filter-key save). QA fixture DB and owned browser profile cleaned by harness.
## Gate run1 (GATE-FAIL, fixed): fmt (scheduler.rs formatting -> cargo fmt), clippy (claims.rs dead field/imports after stateless filter -> removed), test (backfill test pinned old hold-cursor-on-failure behavior that F14 deliberately changed -> test updated to new contract: failed turn counts handled, cursor 103). web-test 4 preexisting alias fails; web-tsc 0; web-build 0.
## Gate run2: GATE-PASS on tree before F37-F41 (fmt 0, clippy 0, test 0, web-test PREEXISTING(4), web-tsc 0, web-build 0). Run3 needed for F37-F41 + inbox.
## Gate run3: GATE-FAIL, exit 1. fmt failed on two new test wrapping sites (patched); clippy, cargo test, web-tsc and web-build each exited 0. web-test had PREEXISTING(4). This run does not certify the later F39 scheduling amendment; final gate still required.
## Gate run4: GATE-PASS
- Current amended tree: fmt exit 0 (4s), clippy exit 0 (37s), cargo test exit 0 (302s), web-tsc exit 0 (5s), web-build exit 0 (3s). Raw web tests classified PREEXISTING(4), matching the baseline alias failures. Verified directly in gate/run4/SUMMARY.txt.
- Runtime consolidated and amended F39 follow-up reports are preserved in r2/runtime-consolidated.md and r2/runtime-followup.md. Remaining shipping requirements are commit/push, clean-source deployment and live health/Slack verification; none is yet complete.

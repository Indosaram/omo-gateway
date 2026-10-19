# Final blocker source re-review

Reviewed 2026-10-12 in `/Users/indo/code/project/omon-gateway-wt/dori-work`, against the original blocker text in `final-batch-review.md` and todos 24, 25, 26, 30, 31 and 35 of `/Users/indo/code/project/omon-gateway/.omo/plans/dori-grade-agent-env.md`.

Verdict: **4 CONFIRMED-SOURCE, 2 BLOCKER**. The six original defects have source fixes; two current acceptance/safety defects remain below. CONFIRMED-SOURCE is a source-and-test-code verdict, not execution or deployment approval. No commands, cargo, tests, QA, database mutations, network requests, deployment or source edits were performed. This report is the only written artifact. Test references establish that assertions exist, not that they passed on this source.

Inputs read include `final-fixes.md` and `final-qa-fixes.md`. The review-blocker worker notes were not discoverable under the supplied evidence directory by filename/content searches; their claimed fixes were checked directly in source and tests instead. Parent changes to the 180-second writing timeout and missing-log handling were inspected directly.

## Todo 24 - BLOCKER

**Original pending-work blocker resolved in source.** `src/inbox/triage.rs:181-196` re-arms undecided items after a partial response or failure, consuming three recovery deadlines; `:387-396` restores pending work from SQLite once at service startup even when collection inserts nothing. FYI/ignore rows are committed before fallible writing at `:247-263`. Unknown IDs do not insert decisions; decision insertion and urgent event insertion remain transactionally coupled at `:319-363`.

**Remaining acceptance defect:** `src/inbox/triage.rs:17,265` sets writing chunks to 10 inside classification chunks of 20. With 25 new owner/bot items, the immediate 20-item flush needs two writer turns and the five-item deadline flush needs a third. The acceptance criterion is 25 jev calls and at most two small-model turns. `tests/test_inbox_triage.rs:162` uses mixed classes (only 13 actionable items) and therefore misses this case. The parent timeout at `src/inbox/triage.rs:552` is bounded at 180 seconds, but does not fix the turn count.

Concrete fix: pack up to 20 actionable items in each writer call while enforcing the existing serialized 6000-character cap, or batch actionable writing across classification chunks within the same admitted burst. Add a 25-item all-owner/all-bot regression asserting exactly 25 judge calls and no more than two writer calls, with every decision stored.

Acceptance code exists for empty collection/kill switch (`tests/test_inbox_triage.rs:208`), 25-item mixed burst, FYI/ignore zero writing (`:191`), valid JSON and urgent event exactly once (`:228`), unknown/malformed/missing writing (`:255`), recovery without new insertion (`:293`), informational preservation (`:323`) and bounded persistent failure (`:356`). Startup restoration and the production 180-second timeout are source-only coverage, not directly asserted by those tests. Recovery exhausts its budget rather than polling models forever; durable undecided rows remain for a later new collection/startup. No inbox records are deleted.

Risk assessment: no unbounded retry was found in this recovery path. Writing disables tools and captures output internally (`src/inbox/triage.rs:531-586`). Proposal preparation now writes snapshots but does not execute them by default; the explicitly enabled lookup path belongs to todo 26. Warning/error logging can include model-supplied IDs/parser errors, so these references do not prove universal secret-free logs.

Deployment-only evidence to collect in todo 36: installed gateway/new-session configuration really collects live source items and invokes triage; a deduplicated second collection produces no new model turn after pending work has settled; real small-model turns complete within the new timeout; restart recovers durable pending work. Fake-runner tests do not establish these operational properties.

## Todo 25 - CONFIRMED-SOURCE

**Original empty-digest blocker resolved.** `src/inbox/digest.rs:86-93` returns None unless owner/bot work, source auth problems or a cron section warrants a report; only afterward does it query and append judge/inbox statistics. Nonzero statistics cannot independently cause an empty task report. `tests/test_inbox_cli_digest.rs:284` seeds FYI collection, judge_calls and filter statistics and asserts None.

Acceptance code exists: seeded filters/order at `tests/test_inbox_cli_digest.rs:53,105`; done/dismissed transitions at `:135`; unknown-item exit 2 at `:159`; empty digest at `:276`; normal cron schedule/DM destination and no private snippets at `:177-273`; authentication once per KST day at `:338`. Approval now requires a prepared proposal rather than silently changing status (`src/inbox/set.rs:36-54`), with rejection/no status mutation at `tests/test_inbox_cli_digest.rs:316` and successful linked execution under todo 26.

Risk assessment: no new statistics-triggered message, source mutation or retry loop was found. `src/inbox/digest.rs:17-30,132-153` checks owner-home-DM routing. The list builder now initializes proposal schema (`src/inbox/list.rs:51`); runtime listing is therefore not a purely read-only database operation, although this reviewer never invoked it. Discord mention safety is not established by DM-only routing; the proposal-specific ping defect is graded under todo 26.

Deployment-only evidence to collect in todo 36: release `inbox list --class owner --json` exits 0 on the deployed schema; the applied persona actually calls both lists; the 09:00 KST digest reaches the actual owner home DM through native delivery and suppresses empty/statistics-only output and same-day repeated auth notices. Local seeded CLI tests do not prove live delivery.

## Todo 26 - BLOCKER

**Both original wiring blockers resolved in source.** Bot decisions with nonempty proposed_action create a prepared proposal in the decision transaction (`src/inbox/triage.rs:343-353`); trusted configuration supplies the route, defaulting to Draft (`:147,151-155,377-386`). `src/inbox/list.rs:29,53-55,81-83` exposes stable numbers. `src/inbox/set.rs:36-83` executes prepared proposals through approval and rejects status-only approval without preparation. `src/inbox/proposals.rs:474-500` implements enabled lookup-only execution; triage service calls it at `src/inbox/triage.rs:401-431`.

**Remaining ping defect:** `src/discord/adapter.rs:843,899` sends proposal/source text and direct lookup/draft results using `safe_allowed_mentions()`. That helper at `src/discord/adapter.rs:144-150` enables all user mentions and reply mentions. `src/inbox/proposals.rs:72-80` passes source title/action into display, and `:116-149` can return arbitrary HTTP response text as owner_text. A source title/action or lookup body containing `<@USER_ID>` can therefore produce a real user ping when these outputs are posted. Owner-DM confinement prevents a shared-channel broadcast but does not neutralize user mentions. No inspected proposal acceptance test asserts a no-mentions Discord payload.

Concrete fix: use proposal-specific allowed_mentions with users, roles, everyone and reply mentions all disabled at both sends, rather than changing the global helper. Add transport/payload coverage using mention-shaped source title/action and lookup/draft output, asserting the mention policy is empty while owner text is still delivered.

Acceptance code exists: `tests/test_inbox_triage.rs:380-473` starts from ordinary bot triage, obtains the numbered ordinary list, approves twice, and asserts one linked approved lane. `tests/test_inbox_proposals.rs:92,136` covers local/Discord approval, link-bearing brief and persisted outcomes; `:179,208` covers unauthenticated/unapproved and disabled-auto zero work; `:272` permits only enabled Lookup and deduplicates it; `:242,302` covers concurrent/failed execution without replay. CAS commits before effects (`src/inbox/proposals.rs:425-443`) and no source send/reply/react interface exists. The all-source default Draft route is safe but does not automatically infer a repo route; actual repo execution needs trusted preparation/configuration. Lookup is a bounded HTTP GET, not a general-purpose external-action runner.

Deployment-only evidence to collect in todo 36: actual owner-DM list presentation/reference binding, one authenticated reply starting the intended scratch lane/direct draft, persisted lane/outcome and zero execution for unapproved items; production configuration supplies the correct trusted route. Enabled lookup should be checked only when that opt-in mode is deployed. Verify no user ping on the real transport after the source fix; this review did not contact Discord.

## Todo 30 - CONFIRMED-SOURCE

**Original recurrence blocker resolved.** `src/cron/health.rs:81-91` reopens a fixed/ignored incident only when the incoming run_id differs, clears the old lane key and replaces serialized detail with the new raw cause, resetting rerun/approval state on Detail::parse. Event insertion is in the same transaction (`:102-118`). Repeated callbacks for the same recorded run do not reopen it. Scheduler calls the hook after outcome commit and isolates hook errors (`src/cron/scheduler.rs:1400-1429`).

Acceptance code exists in `tests/test_cron_health.rs`: real-scheduler failure (`:124`), expect mismatch and clean recovery (`:139`), no-expect (`:182`), delivery failure (`:196,348`), hook DB fault isolation (`:212,232`), kill switch (`:252`), stale lease (`:307`), startup missed occurrence (`:449`), no timer source assertion (`:174`). Recurrence test `:510` launches the isolated helper `:541`, which performs failure -> clean success -> identical failure, replays old/same-run callbacks, asserts one fresh event and reset remediation detail/lane. These helper tests are intentionally invoked by parent tests, not missing coverage because of their helper ignore attributes.

Risk assessment: no model invocation, periodic checker, unbounded rerun or data deletion was introduced by the upsert. Existing source detail/event contents can contain raw cron error text; this upsert does not add log-content emission. The guard only remembers the latest incident run ID, not a complete historical callback ledger; arbitrary older callbacks after multiple recurrences are a residual out-of-order coverage limit, not claimed tested here.

Deployment-only evidence to collect in todo 36: live terminal commit and problem/event timestamps agree within the intended same-second boundary; the deployed hook is enabled/routed to the lead; healthy real jobs invoke no model; fail/recover/recur is actually consumed once by remediation. Seeded scheduler tests do not prove daemon wiring/lead delivery.

## Todo 31 - CONFIRMED-SOURCE

**Original stranded-error and missing-assertion blockers resolved.** `src/cron/remediate.rs:207-244` wraps diagnosis and records needs_owner with structured stage/error evidence, clearing lane_key if no lane exists. `:335-342` wraps verification errors; the rerun reservation is committed before execution (`:371-377`) and retained when transport fails. Errors require explicit owner intervention rather than blind retry. `tests/test_cron_remediation.rs:261` injects log, lane, runs-table and invalid-choice failures and asserts a durable owner task/no phantom lane; `:302` covers transient, settlement and approval rerun failures with one reserved attempt/no second execution.

The parent missing-log fix is present at `src/cron/remediate.rs:554-573`: NotFound returns an explicit no-excerpt marker; other I/O errors still propagate through the recovery wrapper. There is no direct production GatewayRuntime missing-file regression in the inspected tests; the fake log-error test proves recovery, not the new NotFound branch.

Acceptance code exists: lane brief contains fixture error and runtime log at `tests/test_cron_remediation.rs:121-137`; bounded Unicode diagnostic values at `:403`; publishing ready-file/owner task/no rerun at `:140`; fourth attempt refusal at `:160`; natural success marks its lane verified_done without rerun at `:180`; verified transient receipt at `:216`; failed receipt and invalid verification regex at `:349,372`. The missing error/log assertion is now present as data-value assertions, not fixed template wording.

Risk assessment: reserved counter is preserved, maximum remains three (`src/cron/remediate.rs:13,363-377`), publishing requires approval, no dashboard trigger or job disable/delete is added. Natural success takes precedence over error recovery. Failure evidence and owner action text retain raw errors in SQLite; consolidated outcome transport scrubs messages at `src/cron/report.rs:171`. Existing scheduler raw cron-error logging and raw diagnostic brief content are not newly introduced by these fixes, and universal secret-free log output is not certified. Recovery depends on healthy storage; simultaneous failure of the evidence/task transaction is outside the tested single-fault recovery.

Deployment-only evidence to collect in todo 36: actual scratch script fix/marker creation, one native rerun, verified output/destination receipt and fixed problem/lane; publishing path creates only the ready-file/owner task and no unauthorized rerun; fresh install/rotated missing gateway.log does not abort diagnosis. Release `cron health --json` must exit 0 and reflect these real outcomes.

## Todo 35 - CONFIRMED-SOURCE

**Original retained-token blocker resolved under the owner's reduced scope.** Source search across src and tests found no `thread/tokenUsage/updated`, `model_usage` or `input_tokens` implementation/tests. `src/usage.rs:36-99,119-125` reports judge/fallback and inbox counts without token tiers; `src/inbox/digest.rs:93-100` appends that non-token line only after useful work qualifies. Historical migration tables are not active token accounting and should not be dropped destructively. Lane-start revision capture remains at `src/usage.rs:151-180` for done_diff.

Gate remains after signal/worktree checks (`src/lanes/settle.rs:123-133`), with rejection at fulfills < 0.65 or risk > 1.5 (`:208-216`), numeric history/objection and durable not_done event/inbox (`:147-172`), and signal-only skip for unavailable judge (`:218-221`). Parent QA fix now passes full title/brief and includes nonignored untracked files using a private Git index (`:199,227-287`). Collection is capped at 256 KiB and commands have bounded timeouts. The lane's actual index is not overwritten; temporary input cleanup is RAII at `:225-232`. Outbound judge prompt/diff scrubbing is in `src/judge/mod.rs:1309,1427`; persistent judge_calls insertion is at `:787-818`.

Acceptance code exists: seeded judge/fallback counts and absent tiers at `tests/test_usage_done_gate.rs:22`; KST/filter counts at `:60`; fake judge observes a >1500-character brief and untracked done.txt at `:114-128`; low fulfills/high risk/threshold/unavailable judge at `:212-226`; disabled/capped no-model-fallback at `:230`. Settlement cases assert numeric objection/history and a follow-up lane inbox row. Removed token assertions are not requirements.

Risk assessment: no retained fallible token-storage step in backend turns, no token estimations/reporting, no live send or new retry loop. Private-index collection can write Git objects through git add, but does not change the worktree/index or delete lane files. Truncation is bounded judge input rather than deletion of user data. Existing signal-only skip policy intentionally permits closure if diff preparation fails; this is not silently graded as stronger verification.

Deployment-only evidence to collect in todo 36: real configured jev rejects an unrelated README-only diff with persisted numeric reason and judge_calls; the deployed release `usage --since 1h --json` exits 0 and matches persisted judge rows without token fields. Actual jev availability/cap behavior, Developer ID signature, launchd TYPESAFE variable names, applied profile and live lane lifecycle remain todo 36 evidence. No live token evidence is required.

## Summary and boundaries

- 24: BLOCKER - original recovery defect fixed; all-actionable 25-item burst still needs three writer turns.
- 25: CONFIRMED-SOURCE - statistics-only empty digest suppressed with regression code.
- 26: BLOCKER - proposal/approval/auto-lookup wiring fixed; owner-directed source/lookup text still enables user pings.
- 30: CONFIRMED-SOURCE - recovered identical failure reopens once with reset remediation state and regression code.
- 31: CONFIRMED-SOURCE - diagnostic/rerun errors become durable owner work; brief data assertions exist; missing-log branch is source-only.
- 35: CONFIRMED-SOURCE - active token accounting removed; done_diff gate and judge_calls retained with acceptance code.

The report does not infer current test pass status from earlier binaries, worker notes or recorded clippy output. Both BLOCKER fixes are proposals only; no source changes were made. Every deployment item above remains unverified by this read-only re-review.

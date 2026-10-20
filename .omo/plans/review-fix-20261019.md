# review-fix-20261019 - Work Plan

## TL;DR (For humans)

**Who this is for and what changes for them:** For the owner, who talks to the bot on Slack and runs the dashboard. Every defect the review confirms in the five days of changes since the last full review gets fixed, so replies arrive once, on time, and never get edited, replayed, or lost. Approvals stay owner-only, and the dashboard pages load.

**What you'll get:** A full read-only review of every changed file. Every confirmed serious defect gets fixed and re-reviewed until none are left, followed by one green build and test gate, committed and pushed changes, and a verified production redeploy from main.

**Why this approach:** Many small parallel review and fix lanes with strictly separate file sets, so they finish fast and fail in isolation. The lead checks every reviewer claim against the real code and runs one integration gate at the end, which avoids repeated rebuilds.

**What it will NOT do:** No new features, no edits to the global senpi/omo install, and no .env edits. Nothing ships without a green gate.

**Effort:** Large
**Risk:** High - security-sensitive approval code and live Slack delivery are in scope; a bad deploy is mitigated by deploy.sh auto-rollback.
**Decisions I made for you:** Review base is e57b472 (the last full review). Only P0/P1 defects block, and P2s are recorded but not fixed. The in-progress final-only Slack reply and steer-removal changes in the main tree are kept and committed with this work. Workers are review lanes on explore (opus-5-5) and fix lanes on quick.

Your next move: nothing - approval was granted up front; execution runs in this session.

---

> TL;DR (machine): Large, High risk; R1 fan-out review (26 lanes) -> CONFIRMED regrade -> quick fix DAG -> gate.sh -> delta re-review loop -> commit/push/deploy.

## Scope
### Affected user and ideal state
**Affected user:** (a) The owner, Indo, on Slack DM/threads and the dashboard tracker, all day every day. (b) The omo engine and its agents, which consume approvals, lanes and cron. (c) The next programmer or agent, who deploys from main.

| Row | Statement | Reason |
| --- | --- | --- |
| IS-1 | Every owner message gets exactly one final reply: no duplicate, no replay, no silent drop, and no visible edit or delete of posted text | The owner explicitly hates silence and churn (memory: 45 s fail-fast, final-only) |
| IS-2 | No "[시스템 알림: ...]" steer text is injected into the owner's agent turns | It polluted turns and was the reason the owner was angry today |
| IS-3 | Approval / autonomy decisions can only be made by the owner, ledger errors fail closed, and the classifier cannot be bypassed by compound commands | A security invariant |
| IS-4 | Cron, lane and inbox automation behave as before or better (no silent deny, no stuck queue) | Regression guard |
| IS-5 | Dashboard tracker pages load with no API 404 and show correct session state | Linear parity commitment |
| IS-6 | fmt / clippy -D warnings / all tests are green, and main == origin/main == the deployed rev with dirty_paths=0 | Reproducible deploys (memory: deploy only from main) |
| GAP-1 | 176 files (+52k lines) have not had a full review since e57b472 | Unknown defects may be live |
| GAP-2 | The steer-removal and Slack final-only WIP is uncommitted, and its deploy failed on the build lock | IS-1/IS-2 are not live |
| GAP-3 | main is 5 commits ahead of origin, and DEPLOYED has dirty_paths=3 | IS-6 |
| GAP-4..N | One row per CONFIRMED P0/P1 (CONFIRMED-r1.md) | IS-1..IS-5 |

### Must have
- Every CONFIRMED P0/P1 fixed, each with a regression test where the module keeps tests.
- A delta re-review VERIFIED for each fix.
- gate.sh GATE-PASS.
- push, then deploy from main.

### Must NOT have (guardrails, anti-slop, scope boundaries)
- No edits to the global senpi install, and no .env edits.
- No refactors beyond the fix, and no P2 fixes unless they are one-liners inside a touched hunk.
- Fix workers must NOT run cargo, bun or tests (owner rule: one gate at the end).
- Never skip, ignore or delete tests to get green.

## Verification strategy
> Zero human intervention - all verification is agent-executed.
- Test decision: tests-after (cargo test, bun test) with a regression test per behavior fix.
- Evidence: .omo/evidence/review-20261019/ (r1/, CONFIRMED-r1.md, fix/, gate/<run>/SUMMARY.txt, r2/, deploy.log)

## Execution strategy
### Parallel execution waves
- W1: R1 review - 26 explore lanes (opus-5-5), disjoint scopes, plus a settle marker.
- W2: The lead falsifies every Location citation, regrades severity, and writes CONFIRMED-r1.md.
- W3: Fix DAG - one quick node per disjoint file set (write-only).
- W4: gate.sh once (lead).
- W5: Delta re-review DAG - explore, one lane per fix group.
- W3-W5 loop until there are 0 P0/P1.
- W6: commit, push, deploy.sh from main, verify.

### Dependency matrix
| Todo | Depends on | Blocks | Can parallelize with |
| --- | --- | --- | --- |
| 1 R1 review | - | 2 | - |
| 2 CONFIRMED | 1 | 3 | - |
| 3 fix nodes | 2 | 4 | each other (disjoint files) |
| 4 gate | 3 | 5 | - |
| 5 delta review | 4 | 6 or back to 3 | - |
| 6 ship | 5 | - | - |

## Todos
> Implementation + Test = ONE todo. Never separate.
<!-- APPEND TASK BATCHES BELOW THIS LINE WITH edit/apply_patch - never rewrite the headers above. -->
- [ ] 1. R1 fan-out review of e57b472..HEAD+WIP
  What to do / Must NOT do: Run DAG review-20261019-r1 with 26 explore lanes. Read-only.
  Closes: GAP-1
  Parallelization: Wave 1 | Blocked by: - | Blocks: 2
  References: .omo/evidence/review-20261019/diffs/*.diff
  Acceptance criteria: Every lane has a non-empty report harvested to r1/<lane>.md, containing either `### [P` blocks or the line NO FINDINGS.
  QA scenarios: the lead counts the r1/*.md files against the lanes; a missing lane is relaunched.
  Recommended task executor category: quick
  Commit: N
- [ ] 2. Falsify citations and write CONFIRMED-r1.md
  What to do / Must NOT do: Resolve every Location with sed, re-derive every P0/P1 at the source, and regrade.
  Closes: GAP-1
  Parallelization: Wave 2 | Blocked by: 1 | Blocks: 3
  References: r1/*.md
  Acceptance criteria: CONFIRMED-r1.md lists each finding as CONFIRMED / DOWNGRADED / REJECTED, with the reason.
  QA scenarios: the citation resolver reports 0 unresolved citations.
  Recommended task executor category: quick
  Commit: N
- [ ] 3. Fix every CONFIRMED P0/P1 (quick DAG, disjoint file sets)
  What to do / Must NOT do: Make the minimal fix plus a regression test. No cargo.
  Closes: GAP-2, GAP-4..N
  Parallelization: Wave 3 | Blocked by: 2 | Blocks: 4
  References: CONFIRMED-r1.md
  Acceptance criteria: Each fix lands in its owned files only (git diff --name-only matches its scope).
  QA scenarios: gate (todo 4) and delta review (todo 5).
  Recommended task executor category: quick
  Commit: Y | fix(<area>): <summary>
- [ ] 4. Integration gate
  What to do / Must NOT do: Run bash .omo/evidence/review-20261019/gate.sh runN.
  Closes: IS-6
  Parallelization: Wave 4 | Blocked by: 3 | Blocks: 5
  References: gate.sh
  Acceptance criteria: SUMMARY.txt ends with GATE-PASS.
  QA scenarios: on a failure, fix it and rerun the gate.
  Recommended task executor category: quick
  Commit: N
- [ ] 5. Delta re-review
  What to do / Must NOT do: Run explore lanes on the fix diff, checking each fix is VERIFIED and no new P0/P1 appears.
  Closes: IS-1..IS-5
  Parallelization: Wave 5 | Blocked by: 4 | Blocks: 6
  References: r2/
  Acceptance criteria: All fixes are VERIFIED and there are 0 new P0/P1.
  QA scenarios: the lead falsifies the re-review claims.
  Recommended task executor category: quick
  Commit: N
- [ ] 6. Commit, push, deploy
  What to do / Must NOT do: Make atomic commits, git push origin main, then run bash scripts/deploy.sh from main.
  Closes: GAP-2, GAP-3, IS-6
  Parallelization: Wave 6 | Blocked by: 5 | Blocks: -
  References: scripts/deploy.sh
  Acceptance criteria: deploy exits 0; DEPLOYED rev=HEAD with dirty_paths=0; /api/health returns 200; Slack ready.
  QA scenarios: curl -i http://127.0.0.1:9119/api/health and grep "listening on Slack" in ~/.omon/gateway.log.
  Recommended task executor category: quick
  Commit: Y

## Final verification wave
> Runs in parallel after ALL todos. ALL must APPROVE. Surface results and wait for the user's explicit okay before declaring complete.
- [ ] F1. Plan compliance audit
- [ ] F2. Code quality review
- [ ] F3. Real manual QA
- [ ] F4. Ideal-state fidelity

## Commit strategy
- One commit for the pre-existing WIP (steer removal + Slack final-only) after the gate.
- One fix commit per area group (autonomy, slack, agent, tracker/web, cron/inbox/lanes/sessions).
- Follow the history style `fix(<scope>): <imperative>`, with the footer `Plan: .omo/plans/review-fix-20261019.md`.

## Success criteria
> One row per IS row. The plan is complete only when every IS row has a delivering todo and a proving QA scenario; F4 checks the delivered behavior against these rows 1:1, and a shortfall becomes new `- [ ] N.` rows, never a note.
| IS | Delivering todo(s) | Proving QA scenario | Evidence |
| --- | --- | --- | --- |
| IS-1 | 3,6 | slack tests + live DM reply after deploy | gate/, deploy.log |
| IS-2 | 6 | `strings ~/.omon/bin/omo-gateway \| grep -c "사용자가 답변을 기다리고"` == 0 | deploy.log |
| IS-3 | 3,5 | autonomy tests + delta review VERIFIED | r2/ |
| IS-4 | 3,5 | cron/lanes/inbox tests | gate/ |
| IS-5 | 3,6 | /api/tracker/bootstrap 200 (deploy probe) | deploy.log |
| IS-6 | 4,6 | GATE-PASS; origin/main==HEAD; DEPLOYED dirty_paths=0 | gate/, deploy.log |

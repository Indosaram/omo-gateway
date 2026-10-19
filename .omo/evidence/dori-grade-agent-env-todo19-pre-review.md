# SOURCEONLY19 pre-review gate

recommendation: APPROVE (bounded source review only)
status: source-ready-tests-QAglobalpending
blockers: []
confirmation: NOT confirmed done; no shipped-crate, sink QA or global-gate approval

## originalIntent and desiredOutcome
Task 19 in `/Users/indo/code/project/omon-gateway/.omo/plans/dori-grade-agent-env.md:367-375` requires scrubbing home paths, secrets using existing security helpers, and configured private hosts from agent/gateway lane posts and event bodies while retaining owner-authored input. Owner instructions must remain exact and outbound agent text must be redacted.

## userOutcomeReview
The initial review rejected two concrete source gaps: raw agent claim event bodies (B19-1), and absent literal `$HOME` replacement (B19-2). The parent requested a bounded re-review of those two findings only. Both corrections are now present in inspected source. Neither correction was assumed from success prose. This source recommendation closes those two findings, not the outstanding task-19 runtime acceptance criteria or the parent's ownership investigation.

## B19-1 resolved in source
violatedCriterion previously: T19-ALL-EVENT-BODIES
current evidencePointer: `src/lanes/cli.rs:485-505`; `tests/test_lane_events.rs:44-68,72-95`; `.omo/evidence/dori-grade-agent-env/task-19-claim-boundary.md`

The event branch now computes `scrub(text).chars().take(2000).collect::<String>()` only for `claim == true`, then binds that body to lane_events. The owner objection branch retains `text.to_owned()`. Raw lane history still binds text; blocked-answer inbox guidance remains text.to_owned(). The existing transactional update/fence occurs before this branch. The inspected claim receipt describes the same boundary and explicitly labels the regression UNRUN.

The added `agent_claim_event_scrubs_and_bounds_without_changing_internal_evidence` calls the real change_claim storage path with a GitHub token having 36 suffix characters, Korean text and 2,100 additional Korean characters. It reads done_claimed event body, asserts redaction and 2,000 characters, then checks original claim_evidence and history. The owner regression still calls change_claim with false and compares inbox text exactly. These are meaningful source assertions, not executed proof.

## B19-2 resolved in source
violatedCriterion previously: T19-HOME
current evidencePointer: `src/lanes/scrub.rs:13-17,96-113,290-306`

The literal HOME patch was already present when inspected. LITERAL_HOME_SLASH_RE matches `$HOME/` and `${HOME}/`; LITERAL_HOME_BARE_RE matches bare `$HOME` and `${HOME}`. They replace with `~/` and `~` respectively before the existing home-directory regex replacements. Expanded HOME handling remains. The new test covers literal slash and bare forms, Korean surrounding text, and unchanged `$HOMESICK`/`$HOME_DIR`. No helper completion receipt was required to infer this state: the parent explicitly allowed inspection if the patch was present. No helper test execution is credited.

## Earlier source observations retained; not re-audited in this bounded pass

| Class | Initial source observation | Execution status |
| --- | --- | --- |
| Exact GitHub token | Regex and fixtures cover ghp_ plus 36 suffix characters | UNRUN |
| Security helper credentials | scrub calls approval_display and maps its sentinel to <redacted> | UNRUN |
| Home paths/Korean | Surrounding Korean and home path examples present; literal gap now fixed as above | UNRUN |
| Configured private hosts/default | Comma split, trim, default empty, escaped case-insensitive matching | UNRUN |
| Benign/empty/idempotent | Source unit assertions present | UNRUN |
| Length/UTF-8 bounds | Changed posts/activity/runner boundaries scrub before chars().take(2000); claim now does too | UNRUN |
| Agent/gateway event coverage | Activity and runner boundaries covered in initial pass; claim bypass now fixed | UNRUN |
| Owner exactness | Actual CLI storage function preserves blocked owner answer; generic storage enqueue remains raw intentionally | UNRUN |
| Sink/restart/concurrency | Added activity/SQLite/post-drain fake-sink regression is real component-path source, not a delivery receipt | QA UNRUN; live Discord/network N/A to source-only review |

Initial checked paths, relative to `/Users/indo/code/project/omon-gateway-wt/dori-work` unless absolute:
- `/Users/indo/code/project/omon-gateway/.omo/plans/dori-grade-agent-env.md` (task 19, task 8 and verification requirements).
- `.omo/evidence/dori-grade-agent-env/task-19-source.md` and `task-19-boundary-wiring.md`.
- `src/lanes/scrub.rs`, `mod.rs` (export search), `posts.rs`, `activity.rs`, `runner.rs` (failure boundary), `cli.rs` (dispatch and claim/owner boundaries), `discord_threads.rs` (thread creation).
- `src/security/approval_display.rs`, `src/storage/lanes.rs` (generic enqueue).
- `tests/test_lane_scrub.rs`, `tests/test_lane_events.rs` (fake sink and named regression).

Re-review checked only current `src/lanes/cli.rs:480-509`, current `src/lanes/scrub.rs` literal definitions/replacements/test, the two named claim/owner tests in `tests/test_lane_events.rs`, and `task-19-claim-boundary.md`. No future generic audit was performed.

## Skill-perspective and overfit/slop coverage
No matching remove-ai-slops/programming skill files were available in supplied skills or searches under `/Users/indo/.omo` during the initial pass; prompt criteria were applied directly. The initial task files were untracked; git diff gave no historical patch, so the production source and named additions were reviewed directly.

Initial nonblocking notes: helper integration tests repeat unit examples; host regex-error fallback adds complexity. Neither was tied to a failed task-19 criterion. No framework/style audit or redesign is requested. The claim regression now checks the actual event persistence boundary and retained internal input; the literal HOME test checks required external behavior and nearby nonmatches. Neither is deletion-only, a requested-removal test, tautological, implementation-mirroring, or prose-pinning. Neither uses sleep/poll timing. No unnecessary production extraction was introduced by these two fixes.

No separate code-review report explicitly documenting the same skill/overfit checklist was supplied. Initial receipts did not contain that checklist. This coverage gap is recorded, not promoted into an unrelated implementation blocker.

## Exact evidence gaps
- All tests in this review are UNRUN. No Cargo, compiler, runtime, network or live Discord check was executed.
- The reported 11 isolated harness passes in task-19-source.md are 9 scrub tests plus 2 approval-display tests. They are historical claims, not reproduced evidence; they do not cover these corrections, shipped-crate compilation or integrated sink QA.
- Plan-required cargo test output, captured task-19 sink payload and global gate results remain pending. No standalone manual-QA evidence, notepad path or historical task-specific diff was verified.
- The old source receipt's frozen-post statement and classification of every claim CLI input as owner-authored were superseded by inspected wiring and the corrected claim boundary; old receipt prose is not current proof.
- The parent ownership-guard investigation is outside this two-finding re-review and has not been approved here.
- No JS eval tool is available for currentAttemptDir lookup. This report uses the explicit path requested by the parent.

## Cleanup
No product source writes, Cargo, tests, runtime processes, network, services, commits or scratch resources were started. Only this gate report was updated. No cleanup resources remain.

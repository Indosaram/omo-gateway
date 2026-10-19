# Todo19 focused QA and evidence reconciliation

Status: FOCUSED_ASSERTIONS_PASS_WITH_FINDINGS; READY_FOR_INDEPENDENT_REVIEW.
This is not an independent approval or a claim that every external lane surface is covered.

## Scope and execution

Read the complete todo19 entry at `/Users/indo/code/project/omon-gateway/.omo/plans/dori-grade-agent-env.md:367-376`: scrub gateway-posted lane Discord text and event bodies, preserve owner input, test patterns including Korean context, and demonstrate a fake `ghp_` plus 36-character suffix in the sink as `<redacted>`.
Worktree: `/Users/indo/code/project/omon-gateway-wt/dori-work`; HEAD `245a5876138acdd1f4a2c31416121f39e95bf416`. Product source was read-only. No Discord, backend, model, network, service, external DB, configuration, commit, or push action was performed.

`task-19.log` was absent (read returned ENOENT). Existing canonical evidence was adequate for unchanged integration assertions, so those tests were not rerun. To supply the required named helper receipt, exactly one smallest existing command ran:

```text
CARGO_TARGET_DIR=target-w4 CARGO_BUILD_JOBS=2 cargo test --offline -j2 --lib lanes::scrub -- --nocapture
```

Monitor `mon_RY2RB7X6JZYT070W`, session `bash_11`, deadline 1800000 ms; process and monitor exited 0. Ten passed, zero failed/ignored, 471 filtered out; compilation 22.79 s, tests 0.06 s. Raw observed output is transcribed in `task-19.log`, not presented as a historical redirected Cargo run. No suite or Clippy run was launched. Preflight process listing showed two other Cargo jobs; their cwd values were `/Volumes/T9-Mac/ferryx-release-builds/qa-full-aa522310` and `/Volumes/T9-Mac/project/mahoquot-wt/cliproxy-remediation-v3/mahoquot-proxy`, not dori-work/target-w4. Neither was touched. Post-run listing showed no focused Cargo/test child remaining.

## Source-linked existing evidence

All filenames below are in this evidence directory. Independently calculated current hashes before and after the focused run were identical:

| Path | SHA-256 |
| --- | --- |
| src/lanes/scrub.rs | 08562761c2b9183dcfde3efdcb3f586337e26ae0bfa8a7882e8bc94beee196ba |
| src/lanes/posts.rs | 7cc0a2caf6a8ccc288f9001737e2c59ddecc83993d5232ffc00028ea2ddd9230 |
| src/lanes/activity.rs | 8ded771250c7c1f6a095b7e3a27522bd30683805d0b4732030dd81f872c0780c |
| src/lanes/cli.rs | cf5fa7d73c08f9677b890bf39b9059d0c5a67be4ff50f27ad46c8bdd7c46102d |
| src/lanes/discord_threads.rs | ff18725ac0118c700870a6de2ad9c10cd7d023a9d0451327ca72ab9c637bd6fa |
| src/security/approval_display.rs | 10569bf43fb5423a07841266e44509808eb2a591aef32fff4d1472bdfbed2b0c |
| tests/test_lane_scrub.rs | 198b2582d15286382f31733f4414daf280417f877164f76ddf354e6676b3ebc3 |
| tests/test_lane_events.rs | 92bb24b9bdd2ceb057ae613000dba5b5f37fd267d3bcb007c3c414c8f21e43d2 |
| src/lanes/runner.rs | 83c41285631ebe74eb63d91f77ccf6c967464139cd1622b43f8a616783d7136c |

The first eight hashes match `batch-20261011T135508Z-pre-manifest.sha256` and its post manifest; independently compared pre/post manifests are identical. `batch-20261011T135508Z-full.log` SHA-256 is `c5ae061e132fe9a8a07cc964a09e2f73021d404a13ca59d380b6942750d954d1`. Lines 230-249 show all ten helper tests passing; lines 851-879 show event target 24/24, including claim, fake sink, owner preservation, and scrubbed judge excerpt tests; lines 909-919 show scrub integration 6/6. Full Cargo exit is 101, NOT 0: the handoff records 15 baseline failures elsewhere, plus separate historical fmt exit 1 and Clippy exit 101. No global clean verdict is inferred.

Runner's canonical hash was `64a930e43459b12ebd291247ec6f1a876e9743798430d46600e719250374f5dc`. Current runner matches `race-port-20261011T145201Z-post-manifest.sha256`. The read `race-port-20261011T145201Z-runner.diff` only changes backend task/notification selection and trailing whitespace; it does not alter capture or failure scrubbing. Race-port focused log SHA-256 `c95724d2014fe2892a99ab35435b34132ca56e60c4dc8b486bd2c002ff65baee` records runner 11 passed/1 ignored and reopen 5 passed, command exit 0. It is current-runner behavior evidence, not a new scrub sink assertion.

Earlier `task-19-source.md`, `task-19-boundary-wiring.md`, and `task-19-claim-boundary.md` say UNRUN/source-settled and are historical implementation receipts. Canonical execution supersedes those UNRUN labels for the exact unchanged assertions above. Their isolated harness results are not credited as integrated sink QA. No todo19-specific independent review artifact was found in the evidence directory; unrelated gate reports do not approve todo19.

## Authorship and body-path trace

All source references are relative to dori-work.

| Path | Authorship and scrub boundary |
| --- | --- |
| Runner capture, `src/lanes/runner.rs:118-190` | SendMessage, EditMessage and finalized Stream content go to `LaneActivity::captured_item`; UploadFile becomes an internal `[file] path` record; approval returns an intervention error; control actions have no body. This dispatcher has no Discord transport. Raw agent transcript is intentionally internal. |
| Milestone, `src/lanes/activity.rs:169-181` | Agent report text is queued through posts::enqueue; raw ActivityDecision.thread_posts is only returned, not used as a production transport in current source. |
| Question/blocker, `src/lanes/activity.rs:183-193,324-354` | Agent report routes through block -> event; event scrubs then takes 2000 characters before SQLite binding. Internal history and last_report remain raw. |
| Done without claim, `src/lanes/activity.rs:194-205` | Agent report routes through the same scrubbed event boundary. |
| Unreported turn, `src/lanes/activity.rs:238-268` | Scrub then 2000-character excerpt goes to judge; finished/blocked outcomes route through event/block and scrub again. Progress emits no event. |
| Stale, `src/lanes/activity.rs:305-319` | Last agent report or static fallback routes through both event and posts::enqueue. |
| Durable Discord post, `src/lanes/posts.rs:37-55,67-87,103-166` | enqueue scrubs then bounds body; drain parses the stored Post and calls PostSink; Discord implementation uses post.body unchanged in CreateMessage. No blanket scrub at drain/send, so arbitrary legacy/raw inserted Post rows are not validated by this path. Current production enqueue is scrubbed. |
| Runner failure, `src/lanes/runner.rs:790-805` | Gateway reason scrubs then bounds before dead/failed event binding; internal history retains raw reason. |
| Agent claim, `src/lanes/cli.rs:430-514` | claim=true scrubs and bounds event body at 489-493; claim_evidence/history retain originals. |
| Owner object/answer, `src/lanes/cli.rs:179-186,488-512` | claim=false preserves event body text; blocked answer has no event and queues exact text; not_done wraps owner text in lead guidance without redaction. |
| Owner send/reopen, `src/lanes/cli.rs:155-168,245-249,422-429`; `src/lanes/runner.rs:46-80,494-531,576-596` | Raw owner input enters lane_inbox, prompt and steering without scrub. Reopen deletes stale pending guidance and inserts the supplied original. |
| Generic storage, `src/storage/lanes.rs:610-666` | enqueue_lane_event and read APIs preserve supplied bodies. Current production code has no caller of this generic enqueue; do not globally scrub it because it can carry owner text. No production consumer of take_undelivered_events/mark_events_delivered was found; event storage is not a Discord delivery receipt. |

## Assertions actually exercised

- Home: `src/lanes/scrub.rs:208-241,290-305` asserts exact macOS/Linux paths, bare home roots, literal `$HOME`/`${HOME}` paths and bare forms, Korean surrounding text, and preservation of `$HOMESICK`/`$HOME_DIR`. Dynamic actual HOME substitution exists at 96-103 but is not explicitly tested with a custom nonstandard HOME fixture. `tests/test_lane_scrub.rs:17-29,58-68` repeats paths/Korean/literals; no new probes are invented.
- Secrets: `src/lanes/scrub.rs:128-205` asserts fake GitHub token length 4+36, token absence and exact marker; other GitHub prefixes/fine-grained PAT, OpenAI, Anthropic, Slack, JWT, RSA PEM block, bearer/basic headers, URL credentials, CLI api-key and password assignment. Existing security helper is reused at 56-57. These are defined patterns, not an assertion that all possible secret formats are removed.
- Custom hosts: `src/lanes/scrub.rs:245-260` and `tests/test_lane_scrub.rs:32-40` use explicit host slices, assert exact marker replacement, uppercase matches and empty-list preservation. Environment comma parsing/trimming is source-inspected at 41-48; tests do not mutate OMON_SCRUB_HOSTS or demonstrate configured hosts at the sink.
- Fake sink and owner preservation: `tests/test_lane_events.rs:76-138` runs actual capture -> SQLite Post -> posts::drain -> in-process Sink. Sink clones the Post into mpsc and returns synthetic `receipt-123` (`:14-32`). Assertions require exact Korean text plus `<redacted>` in delivered/stored body, Sent state, exact redacted question event, then exact original token-bearing owner text in pending lane_inbox. It is fake sink evidence, not Discord API evidence. The owner preservation assertion covers a blocked answer, not every owner command with every pattern.
- Claim: `tests/test_lane_events.rs:47-73` asserts token absence, Korean marker prefix, 2000 characters, exact internal evidence/history. Path sink check at `:142-172` asserts private path absence and no resend after recovery. Judge excerpt at `:716-745` asserts 2000 characters/private-path absence and event kinds, not event body equality.

## Findings for parent; no product fixes

1. Direct lane thread APIs bypass scrub: `src/lanes/discord_threads.rs:185-207` sends a starter containing raw title; `:304-326` post/edit sends raw text. `src/lanes/cli.rs:393-401` calls create_thread with raw lane.title. Title comes from CLI input, so owner-authored title preservation and gateway-generated title policy need explicit attribution; this is not a demonstrated agent-secret leak. The public post/edit helpers have no production caller in current source beyond their wrappers, but no regression proves they are safe for gateway-authored text. These APIs cannot support an unconditional every-lane-text-scrubbed claim.
2. Shared event table bypass outside lane-keyed bodies: `src/cron/health.rs:103-109,218-224` serializes unsanitized cron detail into lane_events (lane_key NULL). Failed-run detail comes from run_error at `:49-61`. This is a concrete gateway-authored shared-event body bypass, though cron_problem is outside narrowly lane-keyed todo19 scope. Parent must decide scope; no external probe or unauthorized fix was made.
3. Coverage gaps: no explicit env-list-to-sink host test, dynamic nonstandard HOME test, token-bearing stale/done/runner-failure event body test, or owner pattern matrix for send/reopen/nonblocked objection. Shared helper wiring supports those paths by inspection; existing runner failure test checks dead event count, not scrubbed body. Direct Discord transport and live event delivery remain UNRUN by instruction. No pane-id redaction pattern/assertion exists although IS-10 mentions pane ids; todo19's explicit pattern list does not specify pane-id syntax.

## Fixtures and cleanup

New helper-only run uses strings/explicit host slices and LazyLock regexes, reads existing environment, and creates no DB, scratch repo, listener, child backend, or service fixture. Cleanup: focused test/Cargo and monitor exited 0; no owned test process remains; no scratch cleanup was needed. Build cache in target-w4 is intentionally retained. Product fingerprints remained unchanged.

Historical sink fixtures use `Database::connect("sqlite::memory:")`, a MockClock fixed at 2026-10-11 00:00:00 UTC and lane qa-1 (`tests/test_lane_events.rs:502-520`); `/tmp` and `/tmp/brief` are lane metadata, not a real throwaway repository or file DB. Those tests use mpsc/try_recv after awaited drain, not sleeps, and do not create network listeners. Fixture resources are local values dropped at test end; no explicit pool.close or separately observed cleanup receipt is asserted. The same historical 24-test target also includes a separate local WebSocket backend fixture (`:405-500`) with TempDir and an awaited peer; it was not rerun here. Race-port handoff reports its unique TMPDIR empty, rmdir 0, absence check 0, and socket/shutdown fixture pass; that is a historical receipt, not cleanup performed in this QA turn.

Disposition: requested focused evidence reconciliation is complete. Named helper log now exists, source-linked fake sink/owner assertions are credited without rerun, and the findings above are ready for independent review. Overall todo19 approval remains with the parent/reviewer.

# Todo 10 evidence reconciliation

Read-only reconciliation on 2026-10-11. No Discord, network, service, model, Cargo, or product-edit operations were performed. This is historical evidence reconciliation, not new execution or a completion verdict.

## Named artifacts and provenance

The requested named artifact EXISTS: `/Users/indo/code/project/omon-gateway-wt/dori-work/.omo/evidence/dori-grade-agent-env/task-10.log`. It is a Markdown executor receipt containing embedded command output, not a standalone untouched process transcript. Lines 60-79 record 7 unit passes / 1 ignored; lines 82-114 record an explicit historical live invocation, exit 0, lifecycle output, and 1 pass. Lines 134-158 contain its DoneClaim. No task-10.log exists in the inventoried main or dori-next evidence roots.

The independent review exists in main `.omo/evidence/task10-independent-review.md` and dori-work `.omo/evidence/dori-grade-agent-env/task10-independent-review.md`, with identical hashes below. It identifies reviewer `st_01a129a0` and worker spelled `st01a1298a`, reports APPROVE / scope-ready/globalpending, and says it inspected task-10.log without initiating new live external actions. These copies are duplicate receipts, not two live runs. Dori-work `.omo/evidence/dori-grade-agent-env-todo10-gate-review.md` repeats the same IDs/output. Executor/final receipt report all-target/all-feature clippy; independent review records lib-only clippy and 0.07s unit tests versus executor 0.00s. Preserve those as separately reported gates.

Inventory and paginated text search covered all three evidence roots. Dori-next evidence contained only task-21-qa-current.md and no todo10 receipt. Other references in manifests, batch logs, downstream preflights, diagnostic ownership reports, and main pre-reviews add no lifecycle execution. Text search skipped two binary evidence files; their contents are not asserted.

## Requirement map

| Requirement | Existing proof | Limit / gap |
| --- | --- | --- |
| Prefix replacement | dori-work src/lanes/discord_threads.rs:436-473: test_prefix_replace_all_statuses, test_prefix_variations_and_whitespace; :534 test_strip_status_prefix | Pure deterministic name tests, not REST lifecycle calls; historical executor records all as ok. |
| 100-character cap / Korean boundaries | Same source :476-504: test_ascii_100_char_cap, test_korean_unicode_char_boundary_cap; 150 characters become 100 total / 98 suffix; mixed Korean also capped | Source inspected, no rerun. |
| Other scoped tests | :507-572: test_thread_status_conversion_from_lane_status, test_token_parsing_logic | Token test changes process-global environment. Prior blanket claim that all seven tests are pure/deterministic is too broad; name tests are pure. |
| Create / Done rename / archive / GET archived / invalid channel Err and code | task-10.log:82-114, corroborated by prior reviews | Historical executor bot API evidence, not owner live proof or new execution. |
| Unarchive / successful post and edit / mention suppression | dori-work source :211-218, :278-283, :304-328; prior code reviews | No todo10 runtime receipt for unarchive or successful post/edit; specified happy QA does not exercise them. No exact HTTP payload capture proves mention suppression at runtime. |
| Current source identity | dori-work and dori-next hashes differ; main helper absent | Original live receipt does not prove modified dori-next execution. task-18-source.md:41-43 records downstream none_allowed_mentions / is_not_found changes. |

## Exact historical API lifecycle / failure receipt

Recorded command at task-10.log:85: `CARGO_TARGET_DIR=target-w6 cargo test -j 2 --lib lanes::discord_threads::tests::test_live_discord_thread_lifecycle_qa_scenario -- --ignored --nocapture`. Receipt reports exit 0, binary `target-w6/debug/deps/omon_gateway-e129d555675071bf`, 1 passed / 0 failed in 3.36s.

- Parent channel `1529738911377657967`; starter message and thread channel `1558726010181132309`.
- Initial name has Working prefix plus qa-lane-thread; renamed name has Done prefix plus qa-lane-thread.
- Archive response `is_archived: Some(true)`; subsequent GET `GET channel archived flag: true`.
- Invalid channel `999999999999999999`: POST `https://discord.com/api/v10/channels/999999999999999999/messages` returns typed `DiscordApi`, HTTP `404`, Discord code `10003`, message `Unknown Channel`. Code and status are both printed.
- dori-work source :575-598 calls create/set_status; :600-623 archives and asserts GET archived; :625-640 posts to invalid channel, asserts Err, conditionally prints code/status. The receipt logs the exact code; test source does not assert that exact code.
- Missing credentials/channel cause early return (:546-572), so a bare pass is insufficient. Concrete IDs and five step outputs avoid that ambiguity in this historical receipt. Its output has not been authenticated against Discord in this reconciliation.

## API-stub versus owner live proof

No todo10-specific deterministic HTTP/API-stub lifecycle or invalid-channel execution receipt was found. Name tests do not substitute for it. task-15-implementation-preflight.md:83 explicitly states name tests do not prove lifecycle API calls; proposed fake transport checks are future work, not execution. Downstream sink mocks do not prove the exact helper REST exchange.

No owner-performed live receipt/confirmation was found. Existing successful live output is historical executor bot API evidence. Parent decides whether to accept it for the plan live-QA requirement; it must not be relabeled owner QA or new execution. Direct Discord QA remains forbidden. No rerun or new task-10.log was created.

## Historical source corroboration

Dori-work helper hash matches :83 and module hash :85 in batch-20261011T135508Z-pre-manifest.sha256 / post-manifest.sha256; those manifests are byte-identical. batch-20261011T124242Z-full.log:205-213, batch-20261011T132937Z-full.log:206-214, and batch-20261011T135508Z-full.log:205-212 record seven scoped tests ok and live test ignored. These are historical unit corroboration, not extra live runs or blanket green batch verdicts. Executor live receipt lacks an execution-time source manifest; later manifests are corroboration, not a cryptographically bound live-run source freeze.

## Exact paths and SHA-256

Hashes computed locally from existing bytes. MISSING means no source file at that path, not a hash.

- `/Users/indo/code/project/omon-gateway/.omo/plans/dori-grade-agent-env.md`
  - `5a782339ffb6c0129670803fd515c3d8cb6876dd7c44a51457cda51f22cb47b0`
- `/Users/indo/code/project/omon-gateway/.omo/evidence/task10-independent-review.md`
  - `3d14d7193702192298619747cbc9b753e373eba923647e1877ea9a95845dc79e`
- `/Users/indo/code/project/omon-gateway-wt/dori-work/.omo/evidence/dori-grade-agent-env/task-10.log`
  - `56712c2df3edef24edccda7c1ec2d127dcba1e47accb2ee087f2eb27b5a6ccbb`
- `/Users/indo/code/project/omon-gateway-wt/dori-work/.omo/evidence/dori-grade-agent-env/task10-independent-review.md`
  - `3d14d7193702192298619747cbc9b753e373eba923647e1877ea9a95845dc79e`
- `/Users/indo/code/project/omon-gateway-wt/dori-work/.omo/evidence/dori-grade-agent-env-todo10-gate-review.md`
  - `8be5971f6fdc0afa8422e198078d6e8ebba02be6f34bc37602ea43bcb073aca8`
- `/Users/indo/code/project/omon-gateway/src/lanes/discord_threads.rs`
  - `MISSING: ENOENT`
- `/Users/indo/code/project/omon-gateway-wt/dori-work/src/lanes/discord_threads.rs`
  - `ff18725ac0118c700870a6de2ad9c10cd7d023a9d0451327ca72ab9c637bd6fa`
- `/Users/indo/code/project/omon-gateway-wt/dori-next/src/lanes/discord_threads.rs`
  - `f879debcefec007c5364f6f07cf5df4ab3c1a98b0ac9854c2ef765d103551266`
- `/Users/indo/code/project/omon-gateway-wt/dori-work/src/lanes/mod.rs`
  - `e8c05cd90f6c8f26b288c8ec908de5658288527e929ec0d068224033a172cfae`
- `/Users/indo/code/project/omon-gateway-wt/dori-work/.omo/evidence/dori-grade-agent-env/batch-20261011T135508Z-full.log`
  - `c5ae061e132fe9a8a07cc964a09e2f73021d404a13ca59d380b6942750d954d1`
- `/Users/indo/code/project/omon-gateway-wt/dori-work/.omo/evidence/dori-grade-agent-env/batch-20261011T135508Z-pre-manifest.sha256`
  - `5525bd6e6b1eed943005db6584d4e5b1d353acf2d6bfd17cf38c78965dcf5777`
- `/Users/indo/code/project/omon-gateway-wt/dori-work/.omo/evidence/dori-grade-agent-env/batch-20261011T135508Z-post-manifest.sha256`
  - `5525bd6e6b1eed943005db6584d4e5b1d353acf2d6bfd17cf38c78965dcf5777`
- `/Users/indo/code/project/omon-gateway-wt/dori-work/.omo/evidence/dori-grade-agent-env/task-18-source.md`
  - `bd423ad1511a95b4731896dcfe35a93d98d51450ec1070834fe4369e8053ffc2`
- `/Users/indo/code/project/omon-gateway-wt/dori-work/.omo/evidence/dori-grade-agent-env/task-15-implementation-preflight.md`
  - `95d64cd3302809c1aec1979bb1eb0470cda7ade24e541fe3f3574cbfcd697c70`
- `/Users/indo/code/project/omon-gateway-wt/dori-work/.omo/evidence/dori-grade-agent-env/batch-20261011T124242Z-full.log`
  - `aa314c16711859b48199eda7b3195f5f80381b997d67fb967f7ae5b4cda61c3b`
- `/Users/indo/code/project/omon-gateway-wt/dori-work/.omo/evidence/dori-grade-agent-env/batch-20261011T132937Z-full.log`
  - `25a93e46e80b1de06e6b952334407c0121fb08ae39b3122464215ee9ee2e4b7f`


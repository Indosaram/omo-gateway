# Todo 23 current evidence map

Date: 2026-10-11. Verdict: concrete evidence reconciliation, NOT todo completion approval.

## Scope and read boundaries

Main = /Users/indo/code/project/omon-gateway; work = /Users/indo/code/project/omon-gateway-wt/dori-work; next = /Users/indo/code/project/omon-gateway-wt/dori-next. Evidence paths without a tree prefix below resolve under work/.omo/evidence/dori-grade-agent-env/. Source and fixture paths resolve under work.

Only this report is written. No commands, Cargo, network, live source fetches, model calls, database opens/writes, service or git changes, source/fixture edits, or delegation were performed for this reconciliation. Hashing used local file bytes, not a command. These hashes describe observed bytes during ongoing grouped edits, not a frozen complete tree.

Main plan todo23 was read completely at .omo/plans/dori-grade-agent-env.md:416-426 together with shared guardrails and verification strategy. The larger plan reads returned truncated output; unrelated remaining plan sections are not claimed as fully read. All five source-module files, the entire test_inbox_sources.rs, all seven inbox fixture files, task-23.log/json/wiring-request, tool-help.txt, main bounded review, and the named historical receipts below were read. Discord ingress/storage/event excerpts were read, not those three whole files. No inaccessible required todo23 file remained. Unrelated broad-scan output was truncated and is not evidence for absence or completeness.

Main holds the bounded review but not the four adapter/fixture paths. Next/.omo/evidence/dori-grade-agent-env contained only task-21-qa-current.md at inventory time; next test receipts are retained in work under next-focused-* names. Missing task-23-named evidence in next does not mean next tests never ran.

## Actual fixture provenance versus recorded-response requirement

Todo23 requires one real sanitized response per source. Placeholder text and successful parser tests do not independently establish capture provenance. Existing task-23.log says live response contents were not written to fixtures; task-23.json only asserts fixture_text_sanitized. Neither contains an original capture digest or a capture-to-sanitization manifest.

| Source | Fixture and actual test use | Recorded origin evidence | Grounded status / missing proof |
| --- | --- | --- | --- |
| Mail | mail.json: one placeholder thread with date/from/subject/snippet/categories; MailSource::parse in mail_parses_sanitized_thread_and_builds_read_only_cursor_command | task-23.log records one live list thread, YAML summary/items framing; tool-help records empty YAML shape | Observed-schema fixture, sanitized-looking text; exact populated response-to-fixture lineage unconfirmed. No captured raw YAML fixture is exercised through production byte decoder in this test. |
| Kakao | kakao-messages.json: id/logId/authorId/type/real_type plus createdAt/text; parse_messages is exercised. kakao-rooms.json: one OM room; fixture helper has a branch but no test calls fixture("kakao-rooms") | task-23.log records 263 rooms, 4 unread, types, one 50-message page, initial before_id=0 HTTP 400. tool-help later records message timestamp/text fields | Message schema has observation support, not authenticated capture lineage. Rooms fixture is not consumed by production fetch test; HTTP paging test serves handwritten synthetic rooms/pages. Real response fixture provenance still incomplete. |
| iMessage | imessage-chats.json and imessage-history.json: single objects with numeric IDs; history.ndjson: two lines, text "first message"/"second message", redacted sender. Parser tests consume JSON fixtures; NDJSON bytes pass through production decode_structured_output | task-23.log records one chats object and keys; tool-help claims observed history empty/single/NDJSON forms using sample values | Chats framing directly reported. History/NDJSON framing has a later narrative observation, but no capture invocation/exit/digest tying those exact fixture bytes to real history. NDJSON text is synthetic sample text, not explicit REDACTED text. Sanitization test does not include NDJSON or chats/rooms. |
| Discord | discord.json has redacted author/content and placeholder snowflakes; behavioral test uses its guild/channel/message seed but overrides author/content with synthetic cases | task-23.log records only a read-only owner-mention count of 0, no captured indexed document | Synthetic local DB regression seed; no recorded real sanitized Discord indexed row provenance. Builder-based producer regression is not a real serenity conversion/runtime receipt. |

The fixture safety test checks REDACTED presence and excludes two literal strings, not every personal field or capture authenticity. Kakao retains observed room/message/author IDs. No original message bodies are copied into this report.

## Actual read-only boundary and forbidden argv assertions

- Mail: tests/test_inbox_sources.rs:23-35 asserts command_args(Some(epoch)) starts [mail,list], contains after:epoch, has no --json, and enumerates send, reply, forward, archive, read-mark, trash. Production fetch uses the same private args function via run_json. This is helper-level argv proof, not an executed subprocess capture. The plan's --json example is deliberately superseded by recorded zele/0.8.0 help (no such flag); production decoder accepts YAML.
- iMessage: tests/test_inbox_sources.rs:94-103 tests both chats_args and history_args, requires chats/history, and forbids send, read, react. Both helpers feed production run_json. Owner-message exclusion is asserted. No actual CLI argv-recording stub/integration run is present in this target.
- Common runner: sources/mod.rs run_json uses tokio::process::Command::new(program).args(args), null stdin, no shell. Decoder accepts JSON, NDJSON, then YAML; therefore blanket "strict JSON only" is inaccurate. No fixture-backed subprocess/auth_expired failure test is present here.
- Kakao: production uses reqwest GET rooms/messages, no CLI argv exists. Paging fake accepts GET request patterns, but there is no general forbidden-endpoint/method enumeration assertion. No katok call appears in the inspected adapter. Treat endpoint read-only inspection separately from nonexistent CLI argv testing.
- Discord: production reads message_search_documents with SELECT and no Discord API/CLI. It has no argv to enumerate. In-memory tests write only synthetic test databases in prior runs. Local source SQLite reads are NOT Discord manipulation. Owner Discord UI/direct manipulation QA remains prohibited; a verified read-only SQLite copy is an allowed evidence surface, not a missing UI QA obligation.
- If acceptance is read literally as a union list on every CLI source: mail does not enumerate react (or read); iMessage does not enumerate reply/forward/archive/read-mark/trash. Read is forbidden specifically for imsg; mail read is separately excluded by the plan's no-body-fetch rule. Mail's positive [mail,list] assertion already restricts its selected subcommand. Do not label HTTP/SQL sources as failing an argv test for commands they never invoke.

## Recorded runs: passing source tests are not live adapter QA

1. next-focused-20261011T142517Z.log identifies compilation in next and test_inbox_sources: 13 passed, 0 failed, 4 ignored. It explicitly marks all four live_*_source_fetch_once ignored. Status TSV records overall exit 101 because the accompanying lane reopen target failed. That overall failure does not negate the inbox result or become an inbox failure.
2. repair-20261011T144053Z-inbox.log identifies compilation in work and 13 passed, 0 failed, 4 ignored. repair status TSV records inbox exit 0; provenance explicitly says no ignored live tests selected. Same ledger records clippy 101; this is historical evidence, not a current global-gate verdict.
3. task-23.log/json are narrative receipts of direct tool/HTTP/SQLite probes: mail list 1 thread exit 0; Kakao rooms/page probes; imsg chats 1 object exit 0; Discord read-only indexed owner mentions 0. These are real-source observations as recorded, not raw executions of the ignored fetch_since harnesses. task-23.json explicitly lists those harnesses as unrun.
4. The four ignored test bodies print counts AND next_cursor, not counts only. Kakao's cursor includes per-room IDs. No named receipt read here shows those tests run with --ignored or their QA EVIDENCE output. Thus "previously run ignored live count-only logs" is unsupported by these records. Preserve the actual direct-probe observations; do not invent harness runs.
5. Later next-focused-20261011T145256Z.log contains runner/reopen results only, not a new inbox/live run.

### Exact bad-endpoint isolation strength

kakao_bad_endpoint_fails_clearly_without_other_source_dependency (tests/test_inbox_sources.rs:135-142) constructs http://127.0.0.1:9, calls Kakao fetch_since, asserts "PirateTalk GET /api/rooms failed", then calls MailSource::parse on mail.json and checks one item. Both historical inbox logs mark it ok.

This proves a real failing loopback HTTP adapter call plus independent mail fixture parsing in that test. It does NOT call mail fetch_since, iMessage, Discord, or collect_all, and does NOT prove the other three sources returned after Kakao failed. task-23.log class 7 wording overstates the actual test. All-three runtime source isolation requested by todo23 remains missing. No rerun was performed.

## Current-source linkage and residual runtime gaps

Current work and next hashes match for all five source modules, all seven fixtures, and test_inbox_sources.rs. Main lacks these paths. All those current work hashes also equal entries in repair-20261011T144053Z-post-manifest.sha256, linking the historical work inbox pass to this bounded source/fixture set. This does not establish that the whole current gateway tree is tested/frozen.

Discord event fields and actual adapter assignments preserve author_id and referenced_message_id; storage prefers event.author_id and writes reference metadata. Read excerpts agree with main review. discord_producer_event_to_adapter_end_to_end exercises event builders -> index_inbound -> source SELECT, while discord_behavioral_db_cases_and_cursor covers synthetic DM, owner mention, reply, exclusions and cursor. No real conversion/runtime ingress receipt is supplied by these tests. Main review approves only bounded source fixes, explicitly not todo completion.

Genuinely missing: authenticated real sanitized fixture lineage per source (especially Discord; Kakao rooms fixture consumption); actual ignored per-adapter runtime fetch receipts including imsg history and Discord adapter query on an explicit verified read-only copy; all-three return/isolation with bad Kakao endpoint; executed CLI argv capture and broader forbidden-word enumeration if literal acceptance requires it; current whole-tree wiring/global verification. Mail auth_expired failure behavior is present by inspection, not fixture/subprocess tested here. Fixed mail=100 and imsg=50 ceilings are acknowledged in prior evidence, not backfill coverage. No live Discord UI test is requested or justified.

## SHA-256 source/fixture snapshot

Each row below is identical in work and next, absent at the same main path, and matches the historical repair post-manifest.

| Relative path | SHA-256 |
| --- | --- |
| src/inbox/sources/imessage.rs | 359c40b491d52c1d1050a2d54c53f2350ea8012fbf567f71dd13095772e4868a |
| src/inbox/sources/discord.rs | cac3e132b1c302c973e7f070bc797df7738c92a904d906add1e41b9c96593cde |
| src/inbox/sources/kakao.rs | 5f9f87a046ce8b8c56c63f3a2901e2ee4bf3adba2b924a16b830f77e22243f51 |
| src/inbox/sources/mail.rs | ad58b8787d5b11a9c37953ba9ed7cbe171f4e96828bcb1b6b2a3183a6ec6d413 |
| src/inbox/sources/mod.rs | 6ebaf534d85e42cfa0b8323c7b74c8e78bfc7c8729491bb280a63840fb69d237 |
| tests/fixtures/inbox/imessage-chats.json | e594f66625f9f0215f9add21a5f8211242b6b6c08f50972ea4d506014e32d163 |
| tests/fixtures/inbox/discord.json | 9e9c13235344b0d8a9656fdf72ebf3cab8bdb0806e96f8526f64535bcb51069f |
| tests/fixtures/inbox/kakao-messages.json | a9062f26d8b30abb8379fcaad85857846a8885febb021431df410594b0b11883 |
| tests/fixtures/inbox/kakao-rooms.json | 167fd625ece3b6104efd9fb245b15e8dcd58d372e0c03945182e7cbefaadca79 |
| tests/fixtures/inbox/mail.json | 28bac048372a597d2636707228cf7eb751b5d3832eff61aa5501216a35c7e6f3 |
| tests/fixtures/inbox/imessage-history.json | 96e4449d623eade9517b37e71ffb7765db597d342fb34adef797d2f1a3764384 |
| tests/fixtures/inbox/imessage-history.ndjson | 5293d3daf4159632af9d2bcbe6ec57e6808ce1a629fe3ef9c882cd56c425e921 |
| tests/test_inbox_sources.rs | 2aef0dd17a3182734bb6ba7cc88c40ad129d31d8cf95f92847a3c33037995a48 |

Discord seam excerpt files (whole-file hashes; whole-file contents not fully read):

| Work path | SHA-256 |
| --- | --- |
| src/models/events.rs | 92d4ca4ce1ee3614dbec8b2a36c557d91d26084e91d78754033cf8787cf578a1 |
| src/discord/adapter.rs | 9950923c6d20297ebafb2660d2dbbdbd11e78afe28d110eab1a427223c444a84 |
| src/storage/message_search.rs | 8e4429bda25e53586630bd90d3a68ef0c6e4b91a05abd13fbb80e264e47c04ac |

## Exact artifact hashes

| Absolute artifact path | SHA-256 |
| --- | --- |
| /Users/indo/code/project/omon-gateway/.omo/plans/dori-grade-agent-env.md | 44cfced2a04735a4b9b377aaae5eb0e277bcf190fecc2351a6403b5087ee710e |
| /Users/indo/code/project/omon-gateway/.omo/evidence/dori-grade-agent-env-todo23-pre-review.md | 3f11fca20c7d246a7d6fbf3a136111723a92a993ff08dc3a6f697cc8848e577a |
| /Users/indo/code/project/omon-gateway-wt/dori-work/.omo/evidence/dori-grade-agent-env/task-23.log | a527424d17ca13b56a9b67a44f44d5dbda808182a52024438d5ac9322b3a8d98 |
| /Users/indo/code/project/omon-gateway-wt/dori-work/.omo/evidence/dori-grade-agent-env/task-23.json | 50e916db61b2d2c7ad913546aebdd9d9b95c12a96c7beb07f4d2e5edd1acd47e |
| /Users/indo/code/project/omon-gateway-wt/dori-work/.omo/evidence/dori-grade-agent-env/task-23-wiring-request.txt | 40bc0e2a69a93175a4cdbf7ebb15920915b651de02dd1851e6728eed006aa714 |
| /Users/indo/code/project/omon-gateway-wt/dori-work/.omo/evidence/dori-grade-agent-env/tool-help.txt | f47b25d9defc524803a3ebad2e55e990f35712c486992cac8e3f53afb2b9c71a |
| /Users/indo/code/project/omon-gateway-wt/dori-work/.omo/evidence/dori-grade-agent-env/next-focused-20261011T142517Z.log | a4ea46f973af9a0ac1a0b670529d0c9dc9cfcea58eb84639bc97ca2ac7597e13 |
| /Users/indo/code/project/omon-gateway-wt/dori-work/.omo/evidence/dori-grade-agent-env/next-focused-20261011T142517Z-status.tsv | f93b50397c44096f77fc501f21ccb4d25c9855eeed3f8d832942cd6eb884a39a |
| /Users/indo/code/project/omon-gateway-wt/dori-work/.omo/evidence/dori-grade-agent-env/repair-20261011T144053Z-inbox.log | aadbac1bc6f1c34ab9a41609496b8b4f8898e2c87a740168f4e8a37e171bfd9f |
| /Users/indo/code/project/omon-gateway-wt/dori-work/.omo/evidence/dori-grade-agent-env/repair-20261011T144053Z-status.tsv | 920f20cc496f9a852e7a8346a85be88decd82166435e04b8e589e9ea2c16ac19 |
| /Users/indo/code/project/omon-gateway-wt/dori-work/.omo/evidence/dori-grade-agent-env/repair-20261011T144053Z-provenance.md | d5e7a483fb788893f1f04113a0cb6f6277ddc4c7d1a0a0ddbc5bc32c416b468c |
| /Users/indo/code/project/omon-gateway-wt/dori-work/.omo/evidence/dori-grade-agent-env/repair-20261011T144053Z-post-manifest.sha256 | 159ebd57c8c6105029bd48032a8c676e3dad46b9bcec0d89c039ba1b1547358e |

Cleanup: no runtime resources created. Only this evidence map was added. Findings are bounded to the named inspected evidence and observed source snapshot; no blanket confirmed status.


# Todo19 start-title authorship contract proposal

Status: METADATA_CONTRACT_ACCEPTED; IMPLEMENTATION_FROZEN. User accepted published-metadata scrubbing. Key-only output is withdrawn. No speculative quoted-title decision remains open. The release-ready hunks below are evidence, not applied product changes.
No product edit, Cargo command, runtime, DB, network, service or Git action ran.

## Release-ready scope and hunks

Accepted contract: raw task title stays exact; generated starter/name metadata is scrubbed; owner message/inbox/reopen/objection paths stay untouched. Preserve descriptive output and existing status semantics. No flags, public enums, schema/CLI/environment changes, key-only output, unused helper API expansion or provenance mechanism.

Exact product scope against the inspected source: `src/lanes/discord_threads.rs:185-216` creation payload assembly. Private helper insertion immediately before create_thread; existing test module begins at :417. The small extraction below makes the same payload pair consumed by network builders observable without live Discord. The helper scrubs title exactly once before both title renderings, and separately scrubs the starter key component because the accepted contract covers generated external metadata. It never mutates the input strings or persisted lane. No scrub occurs in format_thread_name or set_status.

```diff
@@
-pub async fn create_thread(
+fn creation_metadata(key: &str, title: &str) -> (String, String) {
+    let title = super::scrub::scrub(title);
+    let key = super::scrub::scrub(key);
+    let starter_text = if key.trim().is_empty() {
+        format!("lane: {}", title.trim())
+    } else {
+        format!("lane {}: {}", key.trim(), title.trim())
+    };
+    let thread_name = format_thread_name(ThreadStatus::Working, &title);
+    (starter_text, thread_name)
+}
+
+pub async fn create_thread(
     http: &Http,
     parent_channel: ChannelId,
     key: &str,
     title: &str,
 ) -> Result<CreatedLaneThread, DiscordThreadError> {
-    let starter_text = if key.trim().is_empty() {
-        format!("lane: {}", title.trim())
-    } else {
-        format!("lane {}: {}", key.trim(), title.trim())
-    };
+    let (starter_text, thread_name) = creation_metadata(key, title);
@@
-    let thread_name = format_thread_name(ThreadStatus::Working, title);
     let builder = CreateThread::new(thread_name.clone());
```

Both key and title are generated publication inputs here, not owner message bodies. Empty-key create_thread_with_title still produces the descriptive `lane: <title>` starter and title-based name, now with publication scrubbing through the same creation path. Signatures, allowed mentions, API error propagation and CreatedLaneThread remain unchanged. Existing benign live fixture expectations remain unchanged.

### Status-name inspection: no unintended scope

`src/lanes/discord_threads.rs:228-258` set_status fetches current Discord channel name, verifies thread kind and calls format_thread_name(status, current_name) at :254. `format_thread_name` at :78-88 strips known status prefix and caps Unicode characters; it does not retrieve Lane.title. Therefore a status change of a newly created scrubbed thread retains its scrubbed descriptive base without needing another scrub or raw title read. Do not add scrub to this formatter: that would broaden the change to remote/manually renamed/preexisting names and every caller. No retroactive migration/renaming is authorized. The unused set_status_with_title at :260-269 remains residual risk, not a new product patch. Literal caller search in src/lanes found only wrappers and the ignored live test. Current inspected lane source contains no active status_message_id publisher; implementing todo18 is outside this patch, though its future generated title display follows the accepted metadata contract.

### Exact creation regression hunk

Insert at `src/lanes/discord_threads.rs:420`, after `use super::*;` in the existing test module. Use an isolated child rather than mutating the parallel test runner environment. This checks the actual strings consumed by CreateMessage and CreateThread, not a duplicate renderer or prose contract. The parent selects only one ignored helper, so no recursion or unrelated test execution occurs.

```rust
    #[tokio::test]
    async fn creation_metadata_scrubs_configured_hosts_in_isolation() {
        let output = tokio::time::timeout(
            std::time::Duration::from_secs(10),
            tokio::process::Command::new(std::env::current_exe().unwrap())
                .args([
                    "--exact",
                    "lanes::discord_threads::tests::creation_metadata_subprocess_helper",
                    "--ignored",
                    "--nocapture",
                ])
                .env("OMON_SCRUB_HOSTS", " private.example , SECOND.EXAMPLE , ")
                .env("HOME", "/Users/qa-owner")
                .kill_on_drop(true)
                .output(),
        )
        .await
        .expect("metadata subprocess must finish")
        .expect("metadata subprocess must launch");
        assert!(
            output.status.success(),
            "stdout: {}\nstderr: {}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr),
        );
    }

    #[test]
    #[ignore = "invoked by creation_metadata_scrubs_configured_hosts_in_isolation"]
    fn creation_metadata_subprocess_helper() {
        let token = "ghp_0123456789abcdefghijklmnopqrstuvwxyz";
        let title = format!(
            "작업 /Users/qa-owner/repo {token} PRIVATE.EXAMPLE second.example public.example"
        );
        let original = title.clone();
        let published = "작업 ~/repo <redacted> <redacted> <redacted> public.example";
        let (starter, name) = creation_metadata("qa-1", &title);
        assert_eq!(starter, format!("lane qa-1: {published}"));
        assert_eq!(name, format!("🔄 {published}"));
        assert_eq!(title, original);
        for value in [&starter, &name] {
            assert!(!value.contains(token));
            assert!(!value.contains("/Users/qa-owner"));
            assert!(!value.to_lowercase().contains("private.example"));
            assert!(!value.to_lowercase().contains("second.example"));
        }
        let waiting = format_thread_name(ThreadStatus::Waiting, &name);
        let done = format_thread_name(ThreadStatus::Done, &waiting);
        assert_eq!(waiting, format!("⏸ {published}"));
        assert_eq!(done, format!("✅ {published}"));
        assert_eq!(
            creation_metadata("qa-1", "Fix login"),
            ("lane qa-1: Fix login".into(), "🔄 Fix login".into()),
        );
        assert_eq!(
            creation_metadata("", "Fix login"),
            ("lane: Fix login".into(), "🔄 Fix login".into()),
        );
        let (starter, _) = creation_metadata("private.example", "Fix login");
        assert_eq!(starter, "lane <redacted>: Fix login");
        let (_, name) = creation_metadata("qa-1", &"가".repeat(150));
        assert_eq!(name.chars().count(), 100);
        assert_eq!(name, format!("🔄 {}", "가".repeat(98)));
    }
```

The child HOME setting makes path expectation deterministic and is confined to the child process. No model, runtime service, sleep, live network or global env mutation. Status checks directly compose the actual pure formatting step used by set_status; they do not claim a network edit was exercised. Actual stored-title preservation is independently asserted in the integration seam below; passing String input equality alone would not establish storage preservation.

### Deterministic event stored/delivered and owner-exact regression design

Only other proposed file: `tests/test_lane_events.rs`, insert after the existing token/owner regression ends at :135. Reuse fixture, queued_post and Sink at :14-40; no new transport abstraction. Parent test uses the same 10-second bounded current_exe child pattern, selecting exact helper `configured_host_subprocess_helper`, --ignored --nocapture, child-only OMON_SCRUB_HOSTS/HOME and kill_on_drop; assert exit success with stdout and stderr in failure output. Do not use the module-qualified unit selector for this integration-test executable.

Child helper procedure, all sequential awaited state transitions:

1. Build fixture (SQLite memory, MockClock, qa-1 working lane). Choose text `작업 /Users/qa-owner/repo <fake ghp token with 36-char suffix> PRIVATE.EXAMPLE second.example public.example` and expected published body `작업 ~/repo <redacted> <redacted> <redacted> public.example`. Configure hosts exactly as the creation child.
2. Update qa-1 title to the original text and thread_channel_id to '123'. This is local test-fixture state, not a production mutation. Retain original for comparison.
3. Await captured_item for one milestone with deterministic source `configured-host-post`, then create recording Sink and await posts::drain. Assert count 1. try_recv is valid here: Sink::send synchronously places the cloned payload before awaited drain returns; it does not depend on scheduling/time luck.
4. Assert delivered.body equals expected; assert queued_post.body equals delivered.body; PostState is Sent and message_id is receipt-123. Explicitly reject both configured host spellings case-insensitively, raw path and token in both bodies. Retain public.example and nonsecret Korean description.
5. Await captured_item question with distinct deterministic source `configured-host-question`; query lane_events question body and assert expected. No prose/prompt pinning; these are actual machine-bound body values.
6. Await cli::change_claim(pool, qa-1, false, original) as the blocked-owner answer. Query unconsumed lane_inbox text and assert byte equality to original, including hosts/path/token. Query get_lane and assert lane.title equals original. This protects owner-exact and stored-title behavior while leaving the existing claim_evidence/internal-history regression untouched.
7. No sleeps, repeated polling, live Discord, source environment changes, chronology assertions or duplicate sink scrub. Existing fixture clock and awaited operations are sufficient.

Independent configured-host regression remains ready regardless of product freeze. It is not run or added to product/test sources in this preparation turn. Relevant future focused checks are the new isolated creation unit parent, new isolated lane-events parent, and existing name-format/token-owner regressions; repository lint/format checks wait until implementation is authorized. No checks have run here.

Final release allowlist: src/lanes/discord_threads.rs creation assembly/private builder/tests and tests/test_lane_events.rs new parent/helper only. Do not edit cli.rs, scrub.rs, owner ingress, storage APIs, posts/drain, raw helper API signatures, todo18 implementations, cron or environment files. Freeze release is still required; current8/9 QA completion is not inferred from this evidence.

## Historical semantic proposal -- superseded by accepted contract above

## Corrected recommendation: preserve descriptive metadata, scrub its publication

The previous key-only recommendation was incorrect: it failed to reconcile todo10 and todo18. All proposal text after the historical divider below is retained solely as a rejected historical record; none of its implementation instructions or approval claims remain operative.

### Combined requirements and semantic boundary

Main plan `.omo/plans/dori-grade-agent-env.md` in the main checkout:

- Todo10, line 269: starter message "lane <key>: <title>" and public thread named "🔄 <title>". Line 274 also explicitly permits name truncation to 100 characters on character boundaries. Descriptive title is required, rather than a key-only replacement; this is already a rendered metadata value rather than a promise of a byte-identical message echo.
- Todo18, line 357: status message "⏳ <title> · <elapsed> · turn <n>" and status-dependent thread prefixes. Title must remain part of that descriptive gateway status display.
- Todo19, lines 368-369: "Apply to every lane text the gateway posts to Discord or puts into event bodies" and "Must NOT do: scrub owner-authored messages."
- Todo9, line 258: first prompt after reopening is "the owner's reopening message"; its QA at line 264 uses `lane reopen qa-1 --text "add a line to done.txt"`.
- Todo15, line 324: an owner message arriving in the lane thread is passed to reopen as the first prompt and is not also routed to chat.
- Todo8, line 253: `lane start qa-1 --title "QA lane"` identifies a task label supplied as command metadata, separately from the brief and conversation.

Recommended explicit interpretation: a task label supplied in `StartArgs.title` is metadata regardless of whether a human or agent chooses it. It is not automatically the owner-authored inbound message itself. The protected message paths are inbound content and owner guidance/reopening/objection text; they must not be fed through scrub. Gateway starter, thread-name and status displays are generated publications of task metadata and are subject to todo19's outbound requirement. Their descriptive title is preserved, with only recognized sensitive values replaced or home prefixes shortened.

This is a semantic contract for metadata publication, not an inference that the caller is an agent. It applies identically to owner- and agent-supplied metadata. No trusted authorship discriminator is needed for this option, and no flag/environment/lead field is treated as authentication. If an owner chooses a title containing a secret, the raw title remains stored while its published metadata is redacted. That consequence is explicit, not hidden behind storage preservation.

The plan examples distinguish message text from structured title metadata but do not expressly define the edge case of an exact owner quotation reused as title. Under this proposal, copying message words into a title does not cause mutation of the original message; the copy is published as metadata and scrubbed. If the owner-text prohibition is intended to cover every downstream metadata copy as well, this proposal requires the decision described below instead. It must not be advertised as approved before that semantic interpretation is accepted.

### Minimal concrete repair under this interpretation

Product edit path: `src/lanes/discord_threads.rs` only for the current active creation bypass. Keep existing signatures and callers. At the generated creation payload boundary, scrub the original title once, retain the descriptive result, and use that same result for both the starter's title component and `format_thread_name(Working, ...)`. Scrub the key component independently when rendering the starter if needed; do not scrub the whole starter again. Stored Lane.title and CLI JSON remain raw and unchanged. Benign titles continue to produce the current starter and thread name. Preserve prefix replacement, character-safe 100-character cap, safe mentions and error propagation.

Use a small private pure payload builder in this module only if needed to test the two exact payload values used by create_thread without Discord. Its inputs are existing key/title metadata; its output is starter content and thread name. It introduces no public authorship enum, caller flag, policy registry or transport framework.

Todo18's status publisher must use the same raw-metadata-to-published-value rule when that publisher is present/implemented: scrub the raw title at its output boundary, not the persisted title or owner messages. This is a contract obligation, not authorization to implement missing todo18 behavior here. Current inspected source has no active per-lane status publisher to patch in this repair.

Unused raw post/edit and title-changing helper APIs remain residual risk, with no speculative public API expansion. Existing posts::enqueue and lane event body boundaries remain unchanged; do not add another scrub at drain/sink. Owner send/reopen/objection/answer paths stay untouched. Cron-originated events with lane_key NULL are out of todo19 scope, not todo19 findings.

### Affected callers, tests and compatibility

- Current `cli::start` -> free create_thread -> DiscordThreads wrapper signatures remain compatible; callers need no owner/agent selection. `create_thread_with_title` also publishes title metadata through the same creation builder, without assuming it is an owner message.
- Existing benign live fixture name remains `🔄 qa-lane-thread`; do not change its expected title or run it during freeze.
- Neighboring unit tests in `src/lanes/discord_threads.rs` should observe serialized/payload values from the real builder: benign descriptive title preserved; secret/path title retains non-sensitive descriptive parts; both starter and name exclude fake token/home prefix; status prefix/cap remains correct. Test shipped payload equality and sensitive marker values, not surrounding natural-language prompt prose.
- `tests/test_lane_events.rs` remains the independent configured-host regression path: isolate OMON_SCRUB_HOSTS in one bounded child test process using the existing cron-health test pattern; capture -> SQLite Post -> drain -> recording Sink; assert configured host absence and marker presence in both stored and delivered bodies, unrelated host preserved, question event scrubbed, and original owner answer exact. No process-wide environment mutation, sleeps, polling or live Discord.
- Compatibility: normal descriptive titles and status layouts unchanged; sensitive title metadata is redacted only on external display. Raw title, owner messages, DB schema, CLI flags and JSON stay unchanged. No migration or retrospective thread rename.

### Exact alternative decision if owner-message protection includes title copies

Choose between these two contracts, not a key-only display:

1. **Scrub published metadata (recommended):** preserve descriptive titles, redact sensitive values in starter/thread/status displays regardless of who supplied the title, and never alter the original owner message or stored title. Small output-boundary patch, no new authentication.
2. **Preserve trusted owner-title copies verbatim:** add trusted producer provenance for a verbatim exception. An owner admission path must bind the accepted platform author/message identifier and exact title value in a gateway-controlled record; the consumer must verify the binding, not accept a CLI authorship label. Agent-generated/unknown CLI titles remain scrubbed. Lead/environment fields cannot establish the binding. Such an authenticated producer path does not exist in the inspected standalone start flow and requires an expanded ingress/request contract plus tests for invalid, copied and spoofed provenance. It intentionally permits sensitive values in authenticated owner-title publications and therefore narrows todo19's blanket outbound rule.

The concrete owner decision, only if this edge interpretation remains disputed, is: "May the gateway redact sensitive values in published task-title metadata even when the owner supplied or quoted the title, while preserving the original owner message and stored title exactly; or must verified owner-title copies be published verbatim through a new trusted provenance path?"

No product edits or verification runs have occurred. Proposed minimal repair paths remain src/lanes/discord_threads.rs and tests/test_lane_events.rs, conditional on freeze release and acceptance of metadata-publication semantics. This file is NOT_APPROVED.

## Historical rejected key-only proposal -- not operative

### Former decision (withdrawn)

Recommend separating raw request title from gateway-authored Discord display, without attempting to authenticate authorship in the standalone CLI. Preserve StartArgs.title and Lane.title exactly under the existing storage contract. For the active keyed lane creation path, do not echo title into either the starter message or thread name. Generate both external values from the lane key, scrub that generated display once using the existing lanes::scrub::scrub, then use the existing status-name formatter for the thread name.

Concrete behavior in discord_threads::create_thread:

- If key is nonempty (the current production lane start path), build the gateway display from `lane {key.trim()}` only and scrub that generated string. Send it as starter content and pass the same value into format_thread_name(Working, ...). Do not read title for these two display values.
- If key is empty, preserve the existing title-only behavior of create_thread_with_title. That path is an unused production helper, not the active keyed lane bypass. It receives no new provenance assertion or extra API layer in this patch.
- Keep the persisted raw title, CLI JSON title, brief, owner input, inbox guidance, internal history and transcript untouched. No owner message is fed through scrub by this repair.

The lane key is an identifier, not proof of human or agent authorship. It still requires scrub at the generated display boundary: the CLI key character restriction alone does not exclude every recognizable token or configured host. This proposal prevents echo of the raw title; it does not guarantee that arbitrary semantic information encoded into an identifier is undetectable.

## Why the existing contracts cannot authenticate title authorship

`src/entry.rs:100-104` dispatches a parsed Lane command directly to lanes::cli::run before gateway ingress. `src/lanes/cli.rs:100-138` opens the configured database, constructs a judge and passes the command to start. No InboundEvent, authenticated request principal, approval receipt or admission context is passed.

`StartArgs` at `src/lanes/cli.rs:78-94` contains title, paths, destination thread, lead and model; it has no provenance field. `--lead`, latest matching session and OMON_LANE_DEFAULT_LEAD at `:347-363` identify where notifications belong, not who supplied the title. No inference of owner authorship from those fields is valid.

Trusted transport metadata exists elsewhere: Discord admission checks channel and message.author.id against pairing/user/role policy (`src/discord/adapter.rs:1602-1627`), and accepted events take author_id from the Discord message (`:1800`). `InboundEvent.author_id` is defined at `src/models/events.rs:183-185`. Authorized transport users are not automatically the singular owner, and an accepted owner message does not authenticate the authorship of a title later synthesized by an agent. None of that request metadata reaches the standalone start call.

`bin/omomeow-delegate` forwards caller-provided TITLE to `lane start --title`; its final exec carries DATABASE_URL, not trusted authorship. An agent and a human can invoke the same CLI with the same argv/environment. A caller-provided `--owner`/`--authorship owner` flag would be a label, not authentication; it cannot safely authorize raw outbound title text. A protected signed request/owner admission endpoint would solve a different, larger problem and is not needed to stop this echo.

## Exact plan interpretation and compatibility trade-off

Main plan `/Users/indo/code/project/omon-gateway/.omo/plans/dori-grade-agent-env.md:368` says: "Apply to every lane text the gateway posts to Discord or puts into event bodies."
Line 369 says: "Must NOT do: scrub owner-authored messages."

Those lines do not define title provenance, authorize a public-title exception, or explicitly guarantee that the supplied title must appear verbatim in Discord. Scrubbing a display copied from an owner title while retaining the stored original would still transform the owner-authored words on an outbound surface; storage preservation alone is not sufficient to claim compliance with the prohibition. This proposal therefore does not recommend that reinterpretation.

Instead, the gateway stops copying the ambiguous title into a new outbound message and supplies its own identifier-based display. Raw title/message values are neither redacted nor rewritten. This is an explicit user-visible compatibility change: newly created keyed lane starters and thread names show the lane identifier instead of the descriptive supplied title. Existing threads are not renamed or migrated. Lane list/start JSON and stored title remain descriptive and unchanged. If verbatim title echo is an additional requirement, that would require a trusted provenance contract; it is not established by the inspected plan or current start API.

This engineering choice sacrifices descriptive Discord naming to satisfy both existing constraints without inventing owner identity. No owner question is required to select this minimal repair recommendation; freeze release remains a separate prerequisite to implementation.

## Active path and exact patch scope

Current violating chain: agent-capable `lane start --title ... --thread ...` -> Lane::new(args.title) (`src/lanes/cli.rs:377-385`) -> create_thread(..., lane.key, lane.title) (`:393-401`) -> raw title starter content (`src/lanes/discord_threads.rs:191-202`) and raw title thread name (`:206-209`). Title containing a home path, fake token or configured host reaches both values with no scrub. This is source-reachable behavior, not a live leak receipt.

Proposed product edit path: `src/lanes/discord_threads.rs` only. Keep function signatures and current callers unchanged. Add a small private pure display-pair builder in that same module, used by create_thread to construct the actual starter content/thread name before network operations. The keyed branch ignores title, invokes scrub once on its generated identifier display, and reuses that value. The title-only branch preserves existing behavior. This extraction is the non-network test seam for two payload values, not a new transport/authorship framework. No public authorship enum, caller flag, CLI change, schema change or migration.

Proposed neighboring test edit paths:

- `src/lanes/discord_threads.rs` existing unit-test module: exercise the actual display-pair builder with a nonempty key and a title containing fake GitHub token, home paths and arbitrary descriptive text. Assert machine-bound payload values derive only from key and do not contain title values; assert status formatting and title-only compatibility separately. Tests consume the same pair that create_thread uses, not a parallel test-only renderer. Do not pin descriptive prose. Update only the existing ignored live thread-creation fixture's expected keyed name for the new behavior; do not run it.
- `tests/test_lane_events.rs`: add the required configured-host sink regression at the existing capture/SQLite/PostSink seam. Use the repository child-test pattern from `tests/test_cron_health.rs:253-275`: launch the current test executable with one exact ignored helper selector, set OMON_SCRUB_HOSTS only in that child, bound completion and use kill_on_drop. No shared-process set_var, sleep, polling or live Discord. Configure comma-separated hosts with whitespace and mixed case; trigger milestone capture, await drain, inspect stored and delivered Post body and assert hosts are absent and `<redacted>` present while an unrelated host remains. Exercise a question event body and exact original owner answer in the same isolated helper using the existing fixture. Assertions target payload values and owner preservation, not prompt prose.

All affected current callers: CLI start remains unchanged; free create_thread and its DiscordThreads wrapper retain signatures; the ignored live helper fixture remains compilable with its corrected expected name. The empty-key create_thread_with_title wrappers retain behavior. No production caller of raw post/edit or title-changing wrappers was found beyond their forwarding wrappers, so those helpers remain residual risk and receive no speculative API expansion.

## Existing events and exclusions

Current gateway lane event boundaries already scrub: activity::event (`src/lanes/activity.rs:349-354`), runner::fail (`src/lanes/runner.rs:791-805`), and claim=true (`src/lanes/cli.rs:489-497`). Owner objection/answer branches preserve input; generic storage enqueue is an untyped storage primitive with no current production caller, not the correct place for indiscriminate redaction. Durable posts scrub once at enqueue (`src/lanes/posts.rs:47`); this proposal adds no second scrub in drain or Discord sink.

Cron problem event bodies at `src/cron/health.rs:103-109,218-224` are cron-originated with lane_key NULL. They are out of todo19 scope and are not a requested repair or todo19 blocker here.

## Verification after release, not performed

Focused checks should cover the keyed display builder and configured-host stored/delivered/owner-value regression. Existing token/path/claim tests should remain unchanged and retain their prior evidence unless the implementation touches their behavior. No present recommendation claims test execution, exact-binary validation, global approval or a completed product repair. Final patch allowlist: src/lanes/discord_threads.rs and tests/test_lane_events.rs; all other product paths remain frozen/unmodified.

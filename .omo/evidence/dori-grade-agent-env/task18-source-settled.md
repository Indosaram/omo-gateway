# Task 18 source handoff

Status: blocked at focused compilation; not a verified source-settled claim.
Date: 2026-10-11
Worktree: `/Users/indo/code/project/omon-gateway-wt/dori-work`

## Requirements and inspected evidence

Read the complete todo 18 requirements, prohibitions, acceptance criteria and QA scenarios in the primary plan, plus todo 19's scrubbing contract. Inspected the current `src/lanes/posts.rs`, `src/lanes/discord_threads.rs`, `src/lanes/activity.rs`, runner delivery scheduling, lane storage fields, and `tests/test_lane_thread_transport.rs` / `tests/test_lane_post_transport.rs` before editing. The earlier `task-18-source.md` and primary-tree todo18 pre-review describe another implementation tree, not the current posts source, so they were not treated as verification of this change.

## Owned changes

- `src/lanes/posts.rs`: adds `drain_with_status`, status edit and rename sink operations, scrubbed elapsed/turn status text, persisted `lanes.status_message_id`, 15-minute non-milestone refresh gating, and working/waiting/verified-done prefixes. An edit returning HTTP 404 with Discord code 10008 creates a replacement; other failures do not cause replacement. Creation intent is persisted before sending; acknowledgement and the lane message ID are committed together. Uncertain creation and interrupted sending receipts suppress blind retries across restart. Existing milestone pending/sending/sent/uncertain delivery and enforced nonce behavior are retained. Sends and status edits disable user, role, everyone and reply mentions.
- `tests/test_lane_status_transport.rs`: new focused test using actual Serenity serialization against an owned loopback HTTP recording peer and a temporary SQLite database. Covers two captured milestones producing two milestone posts and two status edits, repeat-drain suppression, the 14/15-minute boundary, blocked rename, verified_done/closed prefixes, 404 replacement ID storage, path/token scrubbing, serialized zero-ping fields, and uncertain replacement suppression after database reopen. Cleanup aborts and joins the server and closes the database on panic/timeout as well as success. These assertions have not executed because compilation is blocked.
- `task18-runner-wiring.patch`: unapplied runner handoff. Uses status-aware drain at startup and post notifications, and refreshes statuses at startup and service-loop wake boundaries. The existing settlement interval provides cheap local wake-ups for the 15-minute gate, with no new model calls or services.

`src/lanes/runner.rs`, `src/lanes/cli.rs`, activity, storage, thread helpers, Cargo.toml, env files and other workers' files were not edited by this task. No live Discord, services, models, full suite, or git commits were used.

## Verification

- Scoped `rustfmt --edition 2021` and final `rustfmt --edition 2021 --check src/lanes/posts.rs tests/test_lane_status_transport.rs`: exit 0.
- `git apply --check .omo/evidence/dori-grade-agent-env/task18-runner-wiring.patch`: exit 0; patch was not applied.
- LSP was requested before cargo. An initial posts error query returned no diagnostics, but final fresh diagnostics for both changed Rust files repeatedly timed out after 3000 ms. This is unavailable verification, not a clean final LSP result.
- Exactly one focused cargo invocation was started:

```sh
CARGO_TARGET_DIR=/Users/indo/code/project/omon-gateway-wt/dori-work/target-w18 cargo test -j2 --offline --test test_lane_status_transport -- --nocapture
```

Monitor `mon_YC79YHJWQAZHGM0D`, terminal `bash_2`. It reported four blocking E0277 compile errors outside this task's ownership:

```text
src/inbox/triage.rs:235:50
src/inbox/triage.rs:245:70
src/inbox/triage.rs:257:77
src/inbox/triage.rs:454:41
`?` couldn't convert the error to `OmonError`
the trait `From<JsonError>` is not implemented for `OmonError`
```

Task 24 owner `st_01a12cf4` was notified. The focused test has not run; no passing result is claimed. The final monitor summary confirmed exit code 101: `could not compile omo-gateway (lib) due to 4 previous errors`. No cargo rerun was launched. This concrete out-of-ownership compiler blocker is the requested stop condition.

## Final source hashes

```text
e2562ae76e241c5d30096cbb834cf8a28c1ca8073df81bf08a47a94d9567e77c  src/lanes/posts.rs
f47d01812f30f59c927ee888d830d5086f15926dbed76038d4fb6c13900ecb7c  tests/test_lane_status_transport.rs
41e9bddefcf744d7709e7c9b6322cc8a5db3865fe4fc04a08c0a7cbd43edb117  .omo/evidence/dori-grade-agent-env/task18-runner-wiring.patch
```

## Integration boundary

Task 13 must integrate the supplied runner patch against its final runner source. After task 24's compiler errors are resolved, the owner integration gate must execute the focused test and obtain final fresh diagnostics. This handoff is local source work with a concrete blocker, not deployed/live acceptance.

## Verification continuation (2026-10-11)

The previous blocked attempt and hashes above are retained as history. This continuation waits for the task 24 owner's triage change before running the requested focused test.

- Registered the requested `stat -f %m` change monitor as `mon_YR5WJNH2HWRWSCZ9` (terminal `bash_1`), with `CHANGED` as its event sentinel. No triage edits were made by this continuation.
- Fresh LSP error diagnostics for `src/lanes/posts.rs` and `tests/test_lane_status_transport.rs` both returned `No diagnostics found`.
- Fresh `rustfmt --edition 2021 --check src/lanes/posts.rs tests/test_lane_status_transport.rs` completed with exit code 0 (monitor `mon_EK47K383RPJ4Z14N`, terminal `bash_2`).
- The focused cargo run is still pending the subscribed triage change; these diagnostics and formatting results are not a passing test claim.

## Verification attribution to completed Wave 4 gate

Status: task 18 transport verification passed in the combined Wave 4 gate; the earlier blocked attempt and pending-monitor entries above remain historical records.

At the user's instruction, cancelled `mon_YR5WJNH2HWRWSCZ9` / `bash_1` rather than waiting for a change that preceded monitor registration. The kill result confirmed cancellation. No new Cargo invocation was started: the existing gate already executed the requested target.

Read `wave4-integration-source-settled.md` in full. Its recorded command includes `--test test_lane_status_transport`, `--offline -j2 --all-features`, and `-- --nocapture`, with an explicit 1800000 ms budget. Monitor `mon_DD7SH4CQ8QQRERXT` / `bash_2` completed exit 0. The target's actual recorded result is **1 passed, 0 failed, 0 ignored**; the combined main-target result is **117 passed, 0 failed, 4 existing ignored**. Status transport used owned loopback fixtures, not live Discord.

Current SHA-256 values were obtained with `shasum -a 256`:

```text
e2562ae76e241c5d30096cbb834cf8a28c1ca8073df81bf08a47a94d9567e77c  src/lanes/posts.rs
f47d01812f30f59c927ee888d830d5086f15926dbed76038d4fb6c13900ecb7c  tests/test_lane_status_transport.rs
41e9bddefcf744d7709e7c9b6322cc8a5db3865fe4fc04a08c0a7cbd43edb117  .omo/evidence/dori-grade-agent-env/task18-runner-wiring.patch
```

The current posts hash exactly matches the Wave 4 gate-input hash. The Wave 4 receipt does not record a test-file hash, so equality against a test hash in that receipt cannot be claimed. The current test hash exactly matches the original task 18 source handoff hash above; the user's correction attributes the completed combined gate to the current source. This distinction is preserved rather than inventing a missing gate-input hash.

Completion audit: monitor cancelled; requested target present in the completed gate with a passing count and exit 0; current posts bytes match captured gate input; current test bytes match the prior task 18 handoff; results and final hashes appended without removing history. No transport failure required a source fix. This continuation did not edit triage.rs, runner.rs, cli.rs, posts.rs, or the test file, and performed no live Discord, services, models, env, or git action. Strict clippy/build and live runtime approval remain outside this requested verification attribution.

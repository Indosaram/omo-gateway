# Task23 source repair receipt

Status: SOURCE_SETTLED; focused invocation completed, integration BLOCKED by an out-of-scope binary compile error. No focused test pass or live-source QA claim.

Worktree: `/Users/indo/code/project/omon-gateway-wt/dori-work`. Implemented the released `task23-source-repair-ready.md` plan. Product edits are restricted to `src/inbox/sources/kakao.rs` and `tests/test_inbox_sources.rs`; this receipt is the requested evidence artifact.

## Changes and provenance

- Numeric `time` is primary, integral signed Unix seconds, range checked by chrono. Present invalid time fails even with a valid RFC3339 alias. Absent time retains createdAt/created_at/timestamp/date. Cursor-floor filtering remains before timestamp parsing. No sendAt/logId timestamp decoding, coercion, rounding, or unit guessing.
- Contract inspected directly in `/Users/indo/code/project/scripts/kakao-archive.ts`: Msg.time, month(time), and time * 1000 archive operations establish seconds. No script execution or external capture performed.
- Six parser regressions cover synthetic numeric seconds, precedence, all four RFC3339 aliases, invalid/null/overflow values, unsupported sendAt-only input, and cursor-floor skips. Synthetic values are explicitly labeled in test code, not authenticated captures. No fixture file or capture lineage was changed or added.
- Real collect_all receives actual failing Kakao at loopback port 9 first, fixture-parser Mail/iMessage sources, and actual DiscordSource backed by an owned migrated in-memory DB populated through InboundEvent/MessageSearchIndex. Assertions cover ordered results, per-source persisted rows/cursors, Kakao cursor/error preservation, and exactly one call per fixture source. This is not three live-source evidence.
- Four ignored live harnesses retain actual adapters, add bounded fetches, emit only QA_COUNT source/items, and use fixed failures without cursor, IDs, private fields, or underlying error output. Discord keeps explicit configured read-only copy and bounded connection. Ignored tests were not run.

## Source SHA-256

Before editing:

```text
5f9f87a046ce8b8c56c63f3a2901e2ee4bf3adba2b924a16b830f77e22243f51  src/inbox/sources/kakao.rs
2aef0dd17a3182734bb6ba7cc88c40ad129d31d8cf95f92847a3c33037995a48  tests/test_inbox_sources.rs
```

At focused launch and after completion (identical):

```text
b604647d15d32f97b0d63f88d86643252594882472f914354ce0b0c928b76250  src/inbox/sources/kakao.rs
5be5cde6bc6b3cc2a90d0291905b63c56e702e79e4ed28cd3fb5bb7e35c2710c  tests/test_inbox_sources.rs
```

## Validation result

`rustfmt --edition 2021 --check src/inbox/sources/kakao.rs tests/test_inbox_sources.rs`: completed successfully, no output. Formatting edits were applied through apply_patch, not rustfmt in-place.

File-level LSP diagnostics for both allowed files, severity all: no diagnostics found before the focused command. LSP launched its own Cargo check; no standalone global check/build/clippy/test was requested.

Exactly one focused invocation, preserving parent's target-w4 and using offline Cargo:

```text
CARGO_TARGET_DIR="$PWD/target-w4" TMPDIR="$S" cargo test --offline -j2 --test test_inbox_sources
```

Monitor `mon_329FSJW7TR970H65`, terminal `bash_8`; final terminal evidence:

```text
error[E0063]: missing field `origin` in initializer of `InboundEvent`
   --> src/main.rs:455:25
error: could not compile `omo-gateway` (bin "omo-gateway") due to 1 previous error
TASK23_TEST_EXIT 101
TASK23_CLEANUP_EXIT 0
```

No test result summary was produced: tests did not execute. Read-only inspection confirmed the unrelated initializer lacks origin; src/main.rs was not edited. Parent must resolve its combined source integration before the next integration gate. No retry or alternative test invocation was issued.

## Cleanup and boundaries

Owned scratch: `/var/folders/zh/7cc25lt91b1_dj577306nwdh0000gn/T/task23_repair_XXXXXX.pRvBIRy2mT`. The post-run find listed no remaining entries, rmdir exited 0, and independent absence check returned SCRATCH_ABSENT_EXIT 0. Test in-memory DB explicitly closes on success and is process-local; no owner/source DB access occurred.

No live ignored fetch, external network fetch, Discord operation, model change, env file edit, git command, service change, full clippy/build/global tests, or capture replacement. Only test loopback access was allowed. The broad initial local filename search was stopped once the prepared plan was located; its terminal bash_2 was killed and completed, with no source side effects.

Parent handoff: scoped source set is settled and hashes stable; focused result is compile-blocked, not green. Parent owns the combined next integration gate.

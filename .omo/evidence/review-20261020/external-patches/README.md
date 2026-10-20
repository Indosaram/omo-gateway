# e19-producer.patch — external patch record (content-intel-dashboard)

E19's producer half cannot ship inside the gateway commit: it lives in a different repository
(`/Users/indo/code/project/content-intel-dashboard`, branch `main`). This directory preserves it
as evidence so the gateway commit is self-contained.

## Contents

- `e19-producer.patch` — unified diff of **only** the E19 hunks, 9 hunks across 2 files.

## Files and source hashes

sha256 of the working-tree files the patch was cut from:

```
4e6f5004e86a388396f57760a9216576d701d6f87307c09be2c6de66b4777820  scripts/publish-ready-digest.ts
8f30dd86b32a3882d2053031cd2b1d19035afbad659b6d2adf1ec20312d692d7  tests/unit/publish-button-action.test.ts
```

Base blobs the hunks were diffed against: `scripts/publish-ready-digest.ts` `18982aa`,
`tests/unit/publish-button-action.test.ts` `d6a8ae5` (i.e. `git diff` against HEAD of that repo).

## Excluded on purpose

That repository has unrelated uncommitted work from another agent in the same file — the
snapshot re-freeze change (`LatestSnapshot`, `cardsRenderedAfter`, `latestSnapshot` now selecting
`created_at`, and the reordered `ensureReviewSnapshot` early return). Those hunks were read,
preserved untouched in the working tree, and deliberately left out of this patch. A grep for
`cardsRenderedAfter|LatestSnapshot|created_at` in `e19-producer.patch` returns nothing.

## Hunks included

`scripts/publish-ready-digest.ts`
- `@@ -371,6 +384,30 @@` — adds `DISCORD_CUSTOM_ID_LIMIT`, `COMPACT_VERSION`, `COMPACT_PART_MARKER`,
  `HEX_SHA256_CHARS`, `compactTokenPart` and the exported `discordCustomId`.
- `@@ -383,7 +420,12 @@` — `buttonComponents` builds the publish `custom_id` through `discordCustomId`.

`tests/unit/publish-button-action.test.ts`
- import of `discordCustomId` plus the `expandGatewayCustomId` mirror and `COMPACTED_SHA256_A`.
- the three expectation sites updated to the compacted form.
- the two `parseArgs(...)` call sites now expand the gateway id first.
- the new known-vector test asserting the id is <=100 characters, matches the exact expected
  compacted string, and round-trips to the original snapshot token.

## Verification performed

- `git apply --check --cached e19-producer.patch` in that repo: applies cleanly to HEAD.
- `bun test tests/unit/publish-button-action.test.ts`: 27 pass, 0 fail (includes the new test).
- No commits, no pushes, no Discord posts were made in that repository.

## Operational status (what is and is not verified)

Verified: the producer in the working tree is modified and tested. `publish-ready-digest.ts` now
emits compacted `custom_id`s, and `bun test tests/unit/publish-button-action.test.ts` passes 27/27,
including the known-vector test that asserts a <=100-character id and a round-trip to the original
snapshot token.

Verified (lead, read-only cron inspection): the cron job `omon-content-intel-publish-ready` runs
the producer from this checkout. Its `payload.script` changes into `content-intel-dashboard` and
runs `bun run scripts/publish-ready-digest.ts`. The producer invocation path is therefore proven;
no post was executed to confirm it.

Not implied by anything here: `/Users/indo/.omon/button-actions.json` points the `content-intel`
namespace at `scripts/publish-button-action.ts` — the **click handler** that runs when a button is
pressed. That is a different script from the digest producer and says nothing about how the digest
message gets posted.

Scope of the E19 fix at runtime: that job's digest is delivered to **Slack**, so the Slack button
value path is the one that runs in production. Discord's 100-character `custom_id` limit is what
E19 was about, and the Discord wire (compaction on the producer side, `expand_custom_id` +
`rebuild_buttons` on the gateway side) is covered by tests only — it is regression-tested, not
exercised by this job. Moving the digest to Discord would activate it.

Committing this repository is a separate, independent step owned by whoever owns it; this patch
exists so the change is not lost when the gateway commit is made.

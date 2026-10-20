# review-20261020 - requirement ledger (107 rows)

## Current shipping gate (supersedes historical gate states below)

`../review-20261019/gate/review-20261020-ship/SUMMARY.txt` records GATE-PASS:
fmt exit 0 (20 s), clippy exit 0 (267 s), Rust tests exit 0 (813 s),
web tests exit 0 (151 pass, 0 fail, 0 unresolved imports), TypeScript exit 0
(62 s), and web build exit 0 (46 s). The lead read the completed summary directly.
The source snapshot is `freeze/tracked-20261020T100754Z.diff`, SHA-256
`4da7066144d01c2b95b5482d0d04de3db745f1c881eefd70c7f3109d3fc0cf32`.
This establishes the integrated gate, not deployment or automatic calendar completion.
Commit, push, signed deployment and external-result verification remain pending.

Audit ledger written 2026-10-20 by senpi child `st_01a1582c` (hephaestus). Read-only: no source
edit, no build, no test run, no commit, no deploy. The only file written is this one.

**Bottom line.** All 107 requirements are mapped to code and to an evidence path. The last completed
gate, `review-20261020-final`, is **`GATE-PASS`** - but it passed a tree that has since changed, so
the authoritative run is now **`review-20261020-drift`** (clippy `exit=0`, tests running, fmt recheck
pending). **Nothing is deployed**: `DEPLOYED rev=5fcfc08` predates every fix here, and no deploy claim
is made anywhere in this file. Three rows - **A1, A5, A14** - have green unit tests and a real
calendar event on record, but the **automatic end-to-end has never run**, because engine adoption is
refused at the owner gate with exit 2; the calendar is **not** proven autonomous. Commit, push and
deploy remain pending. The `user_input_cancellation` edits are included under the owner's broad
fix-all authorization, with attribution left unknown.

Sources used, in authority order:
1. The actual working tree (`/Users/indo/code/project/omon-gateway`, HEAD `5fcfc08`, frozen and
   dirty). Every "implementation" cell below was checked to exist in that tree; every test name
   below was resolved with `grep -F "fn <name>"` to a real file.
2. The requirement statements themselves: `.omo/senpi-task/dag/skills/dag_74b2c346-*.json`
   (nodes 0-17, the `ITEMS (ID: defect)` blocks the fix lanes were given).
3. The 20 lane reports in `.omo/evidence/review-20261020/lanes/`.
4. The gate logs in `.omo/evidence/review-20261019/gate/review-20261020-integrated1/`.
5. Live read-only probes I ran here: `git log/status`, `stat`, and one read-only `sqlite3` query
   against the live DB (`_sqlx_migrations`, `sqlite_master`).

**Keying rule (matters for A1/A11/A14):** the "Requirement" column states the *original user
requirement*, never a lane's description of what it built - and never the node-1 `ITEMS` text as if
it were the source. That text is the **assistant's decomposition**: a useful index of intent, but
when it names an implementation means (a CLI subcommand, a migration, a log rename) that means is
evidence toward the requirement, not the requirement. A lane report is evidence for a row and never
redefines one; one artifact may serve several rows but cannot replace a requirement. Keyed as such:
**A1** = a calendar registration counts as done only when the agent records an external result id
(the automatic end-to-end); **A11** = the delivered-vs-completion bookkeeping; **A14** = autonomy
completion must *mean* an external user result (e.g. a specific calendar event), of which the
`outcome ... --external-id` CLI and the readback are the means.

## Status vocabulary (no row is called verified unless it was executed)

| Status | Meaning |
| --- | --- |
| `TEST-PASSED (gate2)` | Implementation and its named test exist, and `cargo test --all-features --no-fail-fast` **actually ran them green**: `integrated2` `test exit=0 secs=1410`, 2068 passed / **0 failed** / 20 ignored across 93 targets. This is real execution evidence, and it is the strongest Rust evidence in this ledger. It is **not** a gate pass and not a deploy: gate2 ended `GATE-FAIL` on clippy alone, and nothing is live. |
| `FIXED-UNRUN` | Implementation and its test exist in the tree (both checked) but no test run has covered it. After gate2 this applies to no Rust row; it survives only as history. |
| `FIXED-UNRUN (web, tsc-only)` | Web implementation exists; the only machine check is `tsc` exit 0 (type-check, not behaviour). No behavioural test executed. |
| `VERIFIED-EXECUTED (web)` | A web test or browser harness actually ran and passed; the artifact and its exact result are named. |
| `DIAGNOSED` | Investigation concluded with cited evidence; no gateway code change was required. |
| `PARTIAL` | One named part is done/verified and another named part is not. |
| `PENDING` | Nothing done, or the required final action has not happened. |

## Gate state this ledger is written against (measured, not claimed)

- HEAD `5fcfc08c5793f8f29f0b3f7f90e157665421d46f`, branch `main`, `origin/main..HEAD` = 0 and
  `HEAD..origin/main` = 0. The tree is **frozen and dirty**: 86 tracked source paths plus 11
  untracked source files (fingerprint in the freeze section below).
- `gate/review-20261020-integrated1/SUMMARY.txt`: `fmt exit=1 secs=13`, `clippy exit=101
  secs=576`. No `GATE-PASS`. **These are the old numbers for an old tree** - see the current
  checks below.
- `fmt.log` (15:57:32): 45 hunks in 21 files. **All 21 files were modified after that snapshot**
  (`find -newermt "15:57"` = 27 files), so the log describes a tree that no longer exists.

### Current lead-run checks (newer than `integrated1`; kept separate on purpose)

| Check | Command | Result | Provenance |
| --- | --- | --- | --- |
| fmt (current tree) | `cargo fmt --all -- --check` | **exit 0 - passed** | monitor `mon_DH2HJXD0DV6YFRT8` |
| web suite (current tree) | `cd web && bun test` | **151 pass / 0 fail, exit 0** (gate2 `web-test-raw`: `web-test-passes=151`, `web-test-fails=0`, `web-test-unresolved-imports=0`); the earlier 146/0 was monitor `mon_DZ0DM6R1P3MFTMNE` before `descriptionDraft.test.ts` grew from 9 to 14 cases |
| web types | `cd web && bunx tsc -b` | exit 0 | lead run, no stored log |

### gate2 - `review-20261020-integrated2` (17:31): tests green, gate still FAILED

Read verbatim from `gate/review-20261020-integrated2/SUMMARY.txt`:

| Step | Result |
| --- | --- |
| fmt | `exit=0 secs=6` |
| clippy | **`exit=101 secs=522`** |
| test | **`exit=0 secs=1410`** - 2068 passed / 0 failed / 20 ignored across 93 targets |
| web-test-raw | `exit=0` - `web-test-passes=151`, `web-test-fails=0`, `web-test-unresolved-imports=0` |
| web-tsc | `exit=0 secs=34` |
| web-build | `exit=0 secs=20` |
| **SUMMARY verdict** | **`GATE-FAIL`** - because of clippy, and nothing else |

**gate2 is NOT a pass.** Every step except clippy is green; `grep -q 'exit=[1-9]'` still matches the
clippy line, so `gate.sh` wrote `GATE-FAIL`. Anyone quoting gate2 must say "all steps green except
clippy", never "gate passed".

What gate2 proves: the six Rust tests that failed in gate1 all now pass -
`item_is_done_only_when_the_calendar_reads_the_event_back_for_it`,
`readback_matches_the_parsed_id_and_the_whole_item_key`,
`precedent_lookup_queries_lane_owned_worktrees`,
`unclosed_measure_marker_keeps_rest_of_reply`,
`mark_sent_failure_after_a_successful_send_does_not_send_again`, and
`test_default_lead_runtime_config_delegation` (which appears in two targets). The whole Rust suite
compiled and ran for the first time in this review.

gate2's clippy errors (8, at `src/autonomy/loop.rs:978,1013` type_complexity;
`src/context/pack.rs:667`; `src/inbox/sources/mail.rs:220` items-after-test-module;
`src/slack/backfill.rs:628,629`; `src/slack/commands.rs:504`; `src/tracker/issues.rs:1394`) are
being fixed under a **standalone clippy run, monitor `mon_XCHXNWVBXNR2SGEA`**, which has no result
yet. A standalone clippy pass will not retroactively turn gate2 green - only a fresh `gate.sh` run
does that.

The `integrated1` `fmt exit=1` / `clippy exit=101` above stay recorded as the state of the
pre-fix tree; they are not the current verdict and must not be quoted as one.

### final gate - `review-20261020-final` (18:02): **GATE-PASS**

Read verbatim from `gate/review-20261020-final/SUMMARY.txt`:

| Step | Result |
| --- | --- |
| fmt | `exit=0 secs=11` |
| clippy | `exit=0 secs=103` - `Finished dev profile ... in 1m 43s`, no lint emitted |
| test | **`exit=0 secs=1118`** - 2071 passed / **0 failed** / 20 ignored across 93 targets, every `test result:` line `ok` |
| web-test-raw | `exit=0` - `web-test-passes=151`, `web-test-fails=0`, `web-test-unresolved-imports=0` |
| web-tsc | `exit=0 secs=26` |
| web-build | `exit=0 secs=15` |
| **Verdict** | **`GATE-PASS`** |

**The gate is green.** This is the first `GATE-PASS` since `run4` (09:16), and the first one that
covers this review's fixes: the `integrated1` fmt/clippy/test failures and `integrated2`'s 8 clippy
lints are all cleared, and the suite grew from 2068 to **2071** passing tests (the three extra cases
landed with the later fixes).

Note for anyone reading an earlier draft: a mid-run snapshot of this same directory showed only
`fmt` and `clippy` with `test` still compiling. The run finished at 18:02; the numbers above are the
finished state. `GATE-PASS` here means the tree passed the gate - it does **not** mean deployed.

### Freeze snapshot - 20261020T100754Z (current; supersedes the 08:44:17Z fingerprint)

Captured on the lead's explicit snapshot signal, after the backend worker confirmed **exactly two
`assert` reflows patched** and no further source edits. Artifacts live in
`.omo/evidence/review-20261020/freeze/` (see its `README.md` for the full index and the drift-check
commands):

| Artifact | sha256 |
| --- | --- |
| full tracked diff (644978 B) | `4da7066144d01c2b95b5482d0d04de3db745f1c881eefd70c7f3109d3fc0cf32` |
| full tracked diff, `-w` (640181 B) | `82f3b7e9b10eabbd60f3fbdea7431cf4caa344a1732b34777c030afc989073e9` |
| `changed-paths` list | `3a0166d1beefcb1b2a7323add52ffe064eff101c779b94bae670a16f57d663cc` |
| per-file hashes, **all 611 tracked** source files | `10b6ff178d889707e054428175b9c5ed10e0b307cb400332e679959c849602ba` |
| per-file hashes, 11 untracked source files | `d02f2f9614bc8f59645a7913d1a8c349e665a177da2cfd764d4218ae23b30ffc` |

**Aggregate tripwire now**: `git diff` = `4da7066144d01c2b95b5482d0d04de3db745f1c881eefd70c7f3109d3fc0cf32`,
`git diff -w` = `82f3b7e9b10eabbd60f3fbdea7431cf4caa344a1732b34777c030afc989073e9`. The untracked
manifest hash is byte-identical to the previous capture (`d02f2f96…`), confirming those 11 files never
moved.

The two reflows are verifiably in this snapshot: `src/agent/omo_backend.rs` now reads
`assert!(backend(true).ensure_workspace_preset(&enabled_path).await);` and the `!backend(false)`
sibling on single lines, and its per-file diff hash moved `030a661c…` -> `d7d59c91…`.

### HISTORICAL - the previous freeze fingerprint (2026-10-20T08:44:17Z)

Declared frozen. Read-only commands only; reports, logs, `.omo/` evidence and `web/dist` artifacts
are **excluded** from the fingerprint so that evidence writes cannot masquerade as source drift.

Scope: `src web/src migrations tests scripts Cargo.toml Cargo.lock web/bunfig.toml
web/test-preload.ts web/package.json web/tsconfig.json web/vite.config.ts`.

| Component | Value |
| --- | --- |
| Tracked modified source files | **86** |
| Source-only `git diff` sha256 | `6b086892b8f0cf06cccaa4bc54d59d62f7a77473dcd4f067d06d8adfe16a3e8d` (639846 bytes) |
| Tracked path-list sha256 | `ef7358adc263b2e288ce4bafa4283435d7eb78086f6b159704fb881526ae86f1` |
| **Combined fingerprint** (tracked diff + untracked source contents) | `72af1e3bfd633b02846f9771786c3bfa99a25641e1a2a8a458bfc9b6affad17b` |
| Untracked source files | **11** (combined sha256 `d02f2f9614bc8f59645a7913d1a8c349e665a177da2cfd764d4218ae23b30ffc`) |

The 11 untracked source files and their sha256:

| File | sha256 |
| --- | --- |
| `migrations/0054_action_loop_outcomes.sql` | `39f75621033804e8512fed762189b3b1a2608956ca0e586e466a4f00631bb851` |
| `src/runtime_config.rs` | `747fe7f0caa4f6c6d0152bc956685799192cc7a2406e4320339c56c2d65cfbfd` |
| `src/storage/retention.rs` | `a7f50a50d75c1972738697cc25c39070cceec52755943958ab05b31cbb2fe5de` |
| `web/bunfig.toml` | `6a6960bdebdeed62fdb5e45a82722332d862ec00f3ea921f5bc5eab860608142` |
| `web/src/lib/chat-sidebar-session-params.ts` | `f39ce622ba3d21f03383f23417884d4be19ef7bb30d1af039ff0fee588f43d0d` |
| `web/src/lib/ws-url.test.ts` | `d49ea81557044451934257dd58263e37667bdb1b3d61ce54ab25c2dfcc9b34f6` |
| `web/src/lib/ws-url.ts` | `325dd196e89f9948e1ea1f4de58fec979988b0a8b7e5f3b3d26fe816132a29fb` |
| `web/src/tracker/detail/descriptionDraft.test.ts` | `355a733522d99df8369564aca065e589e74f891473b3a92c91f7f1ec606d5d70` |
| `web/src/tracker/detail/descriptionDraft.ts` | `7bca9b9736e679c06b449cab761b3b8016cb582205ac8dcf0239097b29372172` |
| `web/src/tracker/store.test.ts` | `a3744dfc77a4d538f69d73c98304fa4c5c078d78f946c376701407b73274edea` |
| `web/test-preload.ts` | `c0cb37864a6f9f03b24180bb227aa6d4adc4736684b04788d72f875d9a980857` |

### ACTIVE gate - `review-20261020-drift` (open; supersedes `final` as the current run)

| Fact | State |
| --- | --- |
| Gate run | **`review-20261020-drift` - ACTIVE**, the current authoritative run |
| Why it exists | `final` passed at 18:02:24 but 4 source files were touched 17:52-18:02, one of them 3 s after the pass; the tree now fails `fmt` (2 hunks, `src/agent/omo_backend.rs` ~3586/~3599) |
| `fmt` failure | **2 hunks**, both rustfmt reflows - **correction delegated and now applied** by the backend worker (see the freeze snapshot); the original failure stays recorded below and is not rewritten |
| Progress | `clippy exit=0 secs=182`; **tests running**; `fmt` recheck active under monitor `mon_79FK3TS7QX4RHE44` (no result yet) |
| What it must show | a `GATE-PASS` whose `fmt`/`clippy`/`test` steps all started *after* the last source edit, so the pass describes the committed tree |
| What it now covers that `final` did not | the post-freeze semantic edits: `src/inbox/health.rs` (restarted main-runtime worker) and `src/inbox/cli.rs` (an extra real-poll test, inbox recovery). Neither is fmt-only, so `final` cannot vouch for them |

`final`'s `GATE-PASS` stays on record as a real pass of a tree that no longer exists. It must not be
quoted as the current verdict, and the ship checklist's S7 stays **STALE** until this run finishes.
Nothing here is deployed; `DEPLOYED rev=5fcfc08` is unchanged.

### Freeze-artifact lesson (must be fixed in the next freeze)

The 2026-10-20T08:44:17Z freeze was **insufficient by construction**: it stored aggregate hashes
(`6b086892…`, path-list `ef7358ad…`, untracked combined `d02f2f96…`) and per-file hashes for the 11
untracked files only. That is enough to prove *that* drift happened and to name the 4 candidate files
by mtime, but **not** to reconstruct or diff the pre-drift content - which is exactly what was needed
when the drift was found. The next freeze artifact must therefore carry:

1. **The full tracked diff as a file** (e.g. `freeze/tracked-<stamp>.diff`), not only its hash - so any
   later state can be diffed against it, and `git diff -w` against the snapshot becomes possible.
2. **A per-file hash manifest for every tracked path in the source set** (path + sha256 of the
   working-tree content), not just an aggregate - so a single-file change is identifiable without
   relying on mtime.
3. The existing untracked manifest, kept as-is (that part worked).
4. Aggregate hashes retained as the cheap one-line drift tripwire.

Also recorded: **do not capture a freeze while a patch is in progress.** The next capture waits for
the lead's explicit snapshot signal; capturing mid-edit is what produced a fingerprint that could not
be reconciled with the tree minutes later.

### DRIFT DETECTED - the freeze no longer holds (checked 2026-10-20T09:07Z)

Re-running the identical fingerprint command gives
`e7f1d7a4def275384f68f0d4a4d5a6a2396c2164520c9dd48d8e3a94c2563868`, **not** the frozen
`6b086892…e3e8d`. What is and is not recoverable:

| Question | Answer | How |
| --- | --- | --- |
| Did the file *set* change? | **No** - still 86 tracked, path-list hash `ef7358ad…86f1` identical | `git diff --name-only` re-hashed |
| Did the 11 untracked source files change? | **No** - combined hash still `d02f2f96…0ffc`, all 11 byte-identical | per-file `shasum` vs manifest |
| Which tracked files changed? | **4** - see table below | per-file `stat` vs freeze time 17:44:17 |
| Is this a formatter run? | **No.** `rustfmt --check` is clean on three of them, and `omo_backend.rs` has **2 diffs** - a formatter run would have left it clean | `rustfmt --edition 2021 --check` per file |
| Which of the 4 provably changed *content*? | **`omo_backend.rs` by formatting state; `health.rs` and `cli.rs` by owner-confirmed semantic change** (see the attribution table below) | formatting-state flip + owner confirmation |
| Can the pre-drift content be recovered? | **No.** Only aggregate + untracked hashes were stored, never file contents; `git stash`/reflog hold nothing from this window | checked |

| File | mtime | `git diff` sha256 (vs HEAD) | magnitude vs HEAD |
| --- | --- | --- | --- |
| `src/autonomy/effect.rs` | 17:52:54 | `32258d8aae3e8a153c63aacd051d807b82ccfcfcc5a11ca2721858c135a8d012` | +980 -105 |
| `src/inbox/cli.rs` | 17:54:19 | `9d8df730f9fc56fa3b8da8c77f966e609dea968fc2266d69606cfc124b7a97ae` | +132 -1 |
| `src/inbox/health.rs` | 17:55:09 | `005310a6bb53ccd3c57107b48d6908d1dd2b4bc3e0ea9813c30544c96b70c211` | +88 -15 |
| `src/agent/omo_backend.rs` | **18:02:27** | `030a661cc0977dd295b6b3c73cb2abb1ecfc5a8fb9ba63a942947ea60205b973` | +561 -43 |

**What is provable without a content snapshot - and what is not.**

1. **`src/agent/omo_backend.rs` provably changed content after the gate.** The gate's `fmt` step
   exited 0 at 17:41:10, which entails the file was rustfmt-clean at that instant. It is not clean
   now: `rustfmt --edition 2021 --check` reports 2 diffs (~3586, ~3599), and
   `cargo fmt --all -- --check` exits 1 with exactly those 2. Formatting state is a function of
   content, so the content differs. mtime 18:02:27 (3 s after `SUMMARY.txt` at 18:02:24) corroborates.
2. **`omo_backend.rs`: content changed, and the *formatting* deviation is exactly those 2 hunks - which
   does NOT bound the semantic delta.** An earlier draft of this ledger said the drift was
   "layout-only, not semantic". **That was an overreach and is retracted.** `rustfmt` reports what it
   would reflow *now*; it says nothing about what the post-gate edit changed. A semantic edit can
   leave a file 2-hunks-unformatted just as easily as a reflow can. The honest statement: the two
   hunks are `assert!(backend(true).ensure_workspace_preset(&enabled_path).await);` collapsed onto one
   line and the same for `!backend(false)`, and **the semantic delta in this file is unmeasured**, not
   zero. Refusing "fmt-only" applies to my own claim first.
3. **`health.rs` and `cli.rs` carry confirmed semantic changes** - owner-reported, from separate
   workers, and therefore *not* fmt-only either (see the attribution table). `effect.rs` remains the
   only one of the four with no confirmed content change: `rustfmt` is clean on it and no per-file
   hash was captured at freeze, so its mtime (17:52:54) is suspicion, not proof.
4. **Whitespace vs semantic across the whole diff.** Raw `e7f1d7a4…` vs `-w` `974ef5d0…`; line totals
   raw `+11227 -953` vs `-w +11094 -820`. Only **133 lines** are whitespace-only (~1.1%), so this
   diff is semantically dense and `git diff -w` cannot isolate drift. Per file: `effect.rs` raw
   `+980 -105` vs `-w +975 -100`; `cli.rs` identical both ways (`+132 -1`); `health.rs` `+88 -15` vs
   `+78 -5`; `omo_backend.rs` `+561 -43` vs `+559 -41`.
5. **The freeze-instant dispute does not rescue anything.** My capture ran at
   `2026-10-20T08:44:17Z` (17:44:17 local) and is not re-labelled. Even under the owners' claimed
   17:52 freeze, `effect.rs` at 17:52:54 is still **54 s after** it; and `omo_backend.rs` at 18:02:27
   is after the gate's `SUMMARY.txt` at 18:02:24 under *any* claimed instant. Both conclusions hold
   whichever instant is chosen.
6. **No build compiled the drifted content.** `target/debug/.fingerprint/` shows no activity after
   18:02:24, so nothing has compiled, linted or tested these edits.
7. **Attribution: two files attributed to separate workers; exact sessions unknown and not hunted.**
   Owner-confirmed, recorded as claims about *which worker class* made the edit, with the specific
   session id deliberately left unresolved:

   | File | Confirmed change | Attributed to |
   | --- | --- | --- |
   | `src/inbox/health.rs` | **semantic** (not formatting) | a **separate restarted main-runtime worker** - see its `Restart audit` report |
   | `src/inbox/cli.rs` | **an extra real-poll test** added | a **separate inbox-recovery** worker |
   | `src/agent/omo_backend.rs` | content changed (formatting deviation proven); semantic delta unmeasured | unattributed |
   | `src/autonomy/effect.rs` | not confirmed | unattributed |

   No further hunt was performed, by instruction: the exact session id is **unknown and stays
   unknown**. What matters for shipping is that `review-20261020-drift` covers these edits.

   **Not mine**: every command my session ran was read-only (`git diff`, `git diff -w`, `stat`,
   `shasum`, `grep`, read-only `sqlite3`, `rustfmt --check`, `cargo fmt -- --check`); my only write is
   this evidence file. A search of every child session log and task log under `.omo/senpi-task/`
   found **zero** edit/write tool calls targeting these four files - consistent with the edits coming
   from restarted workers and shell-level writes rather than from the original fix lanes.

**Timing is decisive.** The gate's own steps: fmt 17:41:10, clippy 17:43:04, test step began
17:43:04 (1118 s, finished 18:01:42), `SUMMARY.txt` written 18:02:24. Every one of the four edits
landed **after** the gate's compile began, and `omo_backend.rs` landed **3 seconds after** the gate
wrote `GATE-PASS`. So **none of these four files' current content was formatted, linted, compiled or
tested by the gate that passed.**

Consequence, measured now: `cargo fmt --all -- --check` **exits 1** with 2 diffs, both in
`src/agent/omo_backend.rs` (lines ~3586, ~3599 - `assert!(...)` calls the formatter wants collapsed).
The tree that passed fmt at 17:41 cannot pass it now.

I did not cause this: my session ran only read-only commands (`git diff`, `stat`, `shasum`, `grep`,
read-only `sqlite3`, `rustfmt --check`, `cargo fmt -- --check`), and every one of those is
non-writing; my sole write is this evidence file. The edits are therefore **unattributed
concurrent work** - consistent with the release-preparation and `user_input_cancellation` activity
already flagged as not belonging to the fix workers - and ownership needs to come from the owners,
not from this ledger.

**How the previous freeze was checked** (kept for the record; superseded by the snapshot above):

```sh
git diff -- src web/src migrations tests scripts Cargo.toml Cargo.lock \
    web/bunfig.toml web/test-preload.ts web/package.json web/tsconfig.json \
    web/vite.config.ts | shasum -a 256
# must equal 6b086892b8f0cf06cccaa4bc54d59d62f7a77473dcd4f067d06d8adfe16a3e8d
```

If that hash differs, source changed after the freeze and the gate that was run no longer describes
the tree being committed.

### New binary vs. the deployed one - migration proof on a consistent production copy

The deployed binary and the new one behave differently on the same schema, and that is now measured
rather than inferred:

| Run | Database | `max(_sqlx_migrations.version)` | `action_loop_outcomes` |
| --- | --- | --- | --- |
| deployed `5fcfc08` (old binary) | live DB, read-only | **53** | absent |
| **new binary** | consistent copy of that same production DB | **53 -> 54** | **created** |

- `PRAGMA quick_check` on the copy: **ok**. No corruption, no partial migration.
- The migration was applied by the binary's own `MIGRATOR` (`src/storage/db.rs:15,190`) on connect -
  exactly the mechanism A11's row describes, now observed instead of argued.
- The old binary's zeros (`inbox_items where source='mail'` = 0, no outcome table) remain the
  pre-deploy baseline; this run shows the new binary migrates cleanly over that same data.

**Engine adoption attempt: refused, correctly.** Pointing the new binary at the current engine
environment ended with **exit 2 at the owner gate** - it refused and wrote no rows. No bypass was
attempted and none was added. So the adoption path is *exercised and correctly refusing*, not proven
working: **autonomous proof is still not complete** (A1/A5/A14 stay as they are - the automatic
end-to-end has still never run; see those rows).

### Live DB baseline (read-only, on the deployed `5fcfc08` binary)

Queried `file:omon_gateway.db?mode=ro` - no writes, no locks taken:

| Fact | Value | Reading |
| --- | --- | --- |
| `max(_sqlx_migrations.version)` | **53** | the running binary embeds no 0054; a new binary applies it on first startup (see A11) |
| `inbox_items` where `source='mail'` | **0** | the pre-deploy baseline, **not** a failure of the new binary |
| `action_loop_outcomes` table | **absent** (`count(*)` on `sqlite_master` = 0) | same reason: no 0054 in the deployed binary |
| `inbox_items` total rows | **3094** | collection is alive for other sources; only mail is at zero |

These three zeros are a **baseline of the old binary**, and must not be read as evidence that the
mail fix or the outcome table fail. The mail fix and 0054 are in the frozen tree, not in the
running process; only a deploy can move these numbers.

### Source inclusion for the single commit - RESOLVED

The commit is to be **one** commit for the whole fix set. Concurrent `user_input_cancellation` edits
appeared in the tree during this review; their inclusion question was asked and is now **resolved by
the owner's broad original fix-all authorization**, which covers this as an integrated dependency. No
further inclusion permission is required.

**What the change is** (read by the owner; recorded here so the commit message can name it):

- a `user_input_cancellation` helper,
- **4 call sites**, and
- a **loopback test**.

**What it does**: it cancels unsupported input forms so that queued turns are unblocked rather than
stalling behind them.

**Which rows it touches**: **A7** (a busy-thread refusal making the fallback retry collide) and
**A10** (a delivered event with no outcome being lost instead of requeued). It is an integrated
dependency of that behaviour, not a separate feature, which is why the fix-all authorization covers
it.

**Attribution stays unknown** - the exact session that wrote it is not identified, and no further
permission or hunting is required. It is *not* attributed to any lane in this ledger, and it is not
covered by any of the 107 rows as their own work.

Before committing, still re-read `git status` against the freeze snapshot
(`freeze/tracked-hashes-20261020T100754Z.txt`): if the aggregate has moved, source changed after the
snapshot and the drift gate no longer describes what would be committed.

### Lead-reported changes this ledger did not re-verify

Recorded so they are not lost, and deliberately not upgraded to verified - the owner asked for no
further scans, so no source read backs them:

- **Action-loop outcome suppression removed and the terminal guard completed.** This lands in the
  action-loop rows (A10 requeue/failure path, A11 outcome states, A14 readback); those rows stay
  `FIXED-UNRUN` because **the tests are pending** and no current `cargo test` result exists.
- **The `unused_must_use` warning at `workspace.rs:1352` was fixed by the owner.** I did confirm the
  bound form (`let updated = update_label(...)`) is in the file, so nothing remains for clippy there.
- `clippy.log` (16:07:12, ran 576 s from ~15:57:36): 15 errors - `src/agent/omo_backend.rs`
  (async-test attribute, E0502), `src/lanes/cli.rs` (3x `%error = %`, 3x E0433, E0282),
  `src/autonomy/loop.rs` (6x E0308). All three files were edited at 16:04:37 / 16:07:44 /
  16:07:51, i.e. at or after the log completed; the log cannot certify the current text. The
  three owning lanes report the fixes applied, and I confirmed the three shapes are gone from the
  tree (`grep '%error = %' src/lanes` = 0 hits; `#[tokio::test]` alone at omo_backend.rs:3584;
  `Some(...)` at loop.rs:1261/1268/1274 and the multi-line `stub_calendar` calls).
- `test.log` (mtime **16:36:13**) holds the lock line, `Compiling omo-gateway v0.1.0 (...)`, and one
  compiler warning "unused `axum::Json` that must be used" at `src/tracker/workspace.rs:1352:9`. That
  warning is **already fixed** - the owner bound the result (`let updated = update_label(...)`, now at
  `workspace.rs:1358`), which I re-read; nothing is left to clear for clippy there. So `cargo test`
  cleared the lock and **is compiling now, and it is genuinely active** - rustc at ~14% CPU over
  ~20 min with heavy memory compression, i.e. slow, not hung and not never-started. No `test result:`
  line and no pass/fail count exists yet. Rust status everywhere is still unrun - compiling is not
  passing.
- Web (lead-level, per the lead's own run; **no log file was stored for it**): `bun test` 146
  pass / 0 fail, `tsc` exit 0. The only stored web-test log in the repo is the pre-fix baseline
  `gate/run4/web-test-raw.log` (104 pass / 4 fail / 4 errors, 09:15, before these edits).
- Deployed: `~/.omon/bin/DEPLOYED` = `rev=5fcfc08 dirty_paths=0 deployed_at=20261020-112444`.
  That is the *previous* revision. **None of the fixes in this ledger is live.**

## A. Autonomy / runtime / operations (A1-A15, 15 rows)

| ID | Requirement (defect being closed) | Implementation (in tree) | Test / evidence | Actual status |
| --- | --- | --- | --- | --- |
| A1 | **Original requirement (node prompt):** "a calendar registration counts as done only when the agent records an external result id (e.g. calendar event id)" - the automatic end-to-end must be what closes an item, not the loop's own delivery | `src/autonomy/loop.rs:603,617,640` (calendar readback with `READBACK_TIMEOUT`, whole-token item-key match), `src/autonomy/mod.rs:54,194`; state column from `migrations/0054_action_loop_outcomes.sql` | `src/autonomy/loop.rs::item_is_done_only_when_the_calendar_reads_the_event_back_for_it` - **FAILED in gate1** (`integrated1/test.log`: `test result: FAILED. 1089 passed; 5 failed`); `::verified_without_a_calendar_readback_is_refused`. Calendar side real but manual: `61b93aaa-a3fd-4182-9cad-483a6dc084b2@zele` created/updated by the lead by hand, re-read by the lane; `lanes/autonomy-loop.md` states the autonomous path never executed | NOT VERIFIED. The unit half now passes - `item_is_done_only_when_the_calendar_reads_the_event_back_for_it` ran green in gate2 (it failed in gate1) - but a green unit test is not the requirement: the one real calendar write (`61b93aaa-...@zele`) was hand-made by the lead, so the automatic end-to-end (scan -> emit -> lead turn -> outcome carrying the event id -> readback) has still never executed once |
| A2 | Transient network errors (ENOTFOUND, connect errors, timeouts) from the mail watcher are reported as "source auth is dead"; plus mail dates unparseable | `src/inbox/health.rs` (`is_auth_failure` split, `is_network_failure`), `src/inbox/sources/mod.rs` (`run_json`), `src/inbox/sources/mail.rs` (`display_date`) | `tests/test_source_health.rs::token_refresh_that_dies_on_dns_is_not_an_auth_failure`, `::a_credential_verdict_outranks_a_network_error_in_the_same_tail`; `src/inbox/sources/mail.rs::{zele_display_dates_parse_to_their_lower_bound, an_unreadable_date_fails_the_batch_instead_of_guessing}` | TEST-PASSED (gate2) for the classifier, plus a live fetch that covers **only part of this row**. The lead's read-only mail run fetched **5 items with 1 pass, exit 0** - that exercises **fetch and date parsing** against the real provider, nothing more. It does **not** cover the later `health.rs` semantics: that file was edited *after* that run, by a separate restarted main-runtime worker (see the drift attribution table), so the network-vs-auth behaviour now in the tree is covered by `review-20261020-drift` and not by the earlier live run. `ops.md` (11:56) says DIAGNOSED and asks for this fix; `main-runtime.md` (16:18) is newer and contains the original classifier - newest wins. Live collection is still empty (`inbox_items where source='mail'` = 0) and that is the **old-binary baseline**, not a failure: the deployed `5fcfc08` has none of this code, so only a deploy can move the row count |
| A3 | Dead-source notification must follow the classifier, not raw text | No code change: `src/main.rs::supervise_watcher_with_health` reaches `health::report_dead` only through `is_auth_failure` | none (no test drives `supervise_watcher_with_health`); covered indirectly by A2's tests | DEPENDS-ON-A2. Correct only if A2's classifier is right; no own evidence. A2's classifier tests ran green in gate2, so the dependency is satisfied at test level, but A3 itself still has no direct test - and live mail collection is unproven until this ships |
| A4 | kakao-archive cron failed 01:45Z and 02:00Z; find root cause, missed window, backfill | No code change (script not buggy) | `lanes/ops-recovery.md` A4: `cron_outputs` incremental=212 at 02:15Z, archive 2026-10.jsonl has every 10-min bucket 01:40-02:40Z, manual run 02:47:03Z exit 0 `{"incremental":48,...,"total":447368}` | DIAGNOSED (network outage; backfilled; supersedes `ops.md`, which missed the 03:15-04:15Z outage) |
| A5 | **Original requirement (node prompt):** "KakaoTalk archive items -> schedule handling link unverified: ensure kakao source items flow into the action loop like other sources (check collect sources in loop.rs) and record outcome as above" | `src/autonomy/loop.rs:876,933` (source-agnostic intake from `inbox_decisions`) | `src/autonomy/loop.rs::{item_is_done_only_when_the_calendar_reads_the_event_back_for_it - FAILED in gate1, verified_without_a_calendar_readback_is_refused}`; `lanes/autonomy-loop.md` (external calendar write) | PARTIAL. Intake and readback tests ran green in gate2, but the "schedule handling link" - the actual archive-to-schedule hop - is still unproven: the one real calendar event `61b93aaa-...@zele` was created **by the lead by hand** - no `[ACTION_LOOP]` event carried that item and no `action_loop_outcomes` row exists for it, so the kakao-item-to-calendar path has never run end to end |
| A6 | iMessage collection health | No code change | `lanes/ops-recovery.md` A6 (16:05Z): the live path is event-driven - `run_imessage_event_watcher` spawned at `src/main.rs:2963`, `imsg_watch_args`/`parse_imsg_watch_line` at `src/inbox/watchers.rs:207-230`. Live proof: `ps` shows `imsg watch --json --since-rowid 45312` as **pid 53233, PPID 52955** (the live gateway), started 11:31:52 and up 4h32m; launchd `ai.omomeow.heimdal-imessage` running; cursor `2026-10-20T00:16:00.804+00:00` matching `imsg chats`; newer rows prefiltered (ad/otp/shortcode) | DIAGNOSED (healthy, event-driven - there is no polled cursor to advance). `ops.md` read the dead helpers `run_imessage_watch`/`run_cursor_watch` (no callers in `src/`), which is why the earlier reading looked broken; that reading is stale, not an open defect |
| A7 | Busy-thread refusal ("Agent is already processing") made the fallback retry collide and fail the turn | `src/agent/omo_backend.rs:143` (`PreAckState`), `:1693`, `:1278` (`idle_wait_deadline`) | `src/agent/omo_backend.rs::{a_busy_refusal_waits_for_the_same_thread_to_go_idle_and_then_resubmits_once, a_turn_that_finished_before_the_refusal_is_resubmitted_immediately, a_busy_thread_rejection_is_told_apart_from_a_real_turn_start_failure}` | TEST-PASSED (gate2). Residual: `idle_already()` trusts the last boundary frame, so a lost retry is possible (bounded, not a loop) |
| A8 | 02:12Z/02:17Z "no output within 45s" - model call or event relay? | No code change | `lanes/agent-backend.md` A8: `~/.omon/gateway.log` fallback retries, `~/.omo/agent/logs/session.log` accepted steer/`thread/settings/update` round trips, `fallback.log` `503 no available accounts` / `circuit-open`, appserver log no crash | DIAGNOSED (upstream model stall, watchdog fired correctly). Evidence is the lane's log read; I did not re-read those logs |
| A9 | Slack outage 02:02:38Z-02:13:52Z: did reconcile/backfill recover the window? | Observability fix only: `src/slack/backfill.rs:117` per-pass warning with failed/routed counts, `:164` held-window warning | `lanes/slack-transport.md` A9: read-only `conversations.history`/`replies` comparison against Slack shows **0 human messages** in the window across every tracked channel/thread; cursors held by construction | DIAGNOSED. Remote emptiness verified; recovery for a *non-empty* outage unverified (the window was empty) |
| A10 | Delivered event with no outcome is lost; requeue with bounded retries | `src/autonomy/loop.rs:706` (`reconcile:` after 30 min, `MAX_ATTEMPTS = 3`) | `src/autonomy/loop.rs::{item_without_outcome_is_requeued_as_reconcile_then_failed, judge_skipped_event_does_not_burn_attempts}` | TEST-PASSED (gate2). Never exercised against a real failed turn |
| A11 | **Original requirement (node prompt):** delivered is not completion - "rename log to `action_loop enqueued`", keep an outcome record per item (`enqueued -> executed -> verified/failed`) via a new migration, and "log counts of enqueued vs verified" | `src/autonomy/loop.rs:247,354` (`emit` logs `action_loop enqueued` with `outcome_counts_for_lead`), `:316` (outcome row written in the same transaction as the `lane_events` row and the `action_loop_seen` mark); `migrations/0054_action_loop_outcomes.sql` | `src/autonomy/loop.rs` outcome-state tests (`item_without_outcome_is_requeued_as_reconcile_then_failed` covers the A10/A11 boundary); `lanes/autonomy-loop.md` A11. Lead reports outcome suppression removed and the terminal guard complete - **tests pending**, no current `cargo test` result | TEST-PASSED (gate2) + **migration observed applied**. The new binary, run against a consistent copy of the production DB, moved `_sqlx_migrations` **53 -> 54** and created `action_loop_outcomes`, with `PRAGMA quick_check` = ok - so the mechanism (`sqlx::migrate!("./migrations")` at `src/storage/db.rs:15`, `MIGRATOR.run` at `db.rs:190`) is proven by execution, not argument. No migration blocker exists. Still open: the log-rename half has no assertion, and the deployed `5fcfc08` still logs the old wording |
| A12 | Owner report "텍스트 위치가 깨진다" - card-news text covered by collage panels | Not in this repo: `content-intel-dashboard/lib/open-design-orchestration.ts` `enforceFixedZoneLayout` (`position:static` -> `relative`), new `card-news-layout-audit.ts`; all **uncommitted** in that repo | `lanes/ops-recovery.md` A12: repro at `output/96/attempt-147`, A/B renders, PNG proof | FIXED-EXTERNAL, tests green. Both halves are now closed in `content-intel-dashboard` (uncommitted there): the owner-visible zone-overlap fix, and the pipeline blocker - `lanes/a12-final.md` records **26 passed, 3 subtests passed in 0.76s** from `python3.12 -m pytest tests/test_render_layout_audit.py tests/test_render_open_design_artifact.py -q`, after `render-open-design-artifact.py` stopped trusting Chrome's exit code and required the artifact instead. The regression direction is covered: a byte-perfect PNG with exit 1/134/139 is still rejected (3 subtests). Nothing is committed in that repo and nothing is deployed here |
| A13 | Deploy confirmation: pid, readiness, codesign authority, Slack socket | No code change | `lanes/ops-recovery.md` A13 + `~/.omon/bin/DEPLOYED` + `codesign -dvvv` (Developer ID: Indo Yoon, TeamIdentifier 5DUM8WPB4C, runtime flag, Timestamp 11:31:45) | VERIFIED-FOR-`5fcfc08` ONLY. Readiness is **two different probes on two different ports**, and both are required: (a) the **app-server** at `http://127.0.0.1:19742/readyz` - `scripts/deploy.sh:41` builds it as `APPSERVER_HTTP="${APPSERVER_URL/ws:\/\//http://}"` from `ws://127.0.0.1:19742`, and `deploy.sh:129` requires `curl "$APPSERVER_HTTP/readyz"` to return **200** as a hard deploy condition (`deploy.sh:17`); (b) the **dashboard** on `127.0.0.1:9119`, where the readiness route is `GET /api/ready` (`src/dashboard.rs:788`) and `deploy.sh` additionally requires the installed `index.html` byte-match plus 200 on each `DASHBOARD_PROBES` path (`/api/health`, `/api/tracker/bootstrap`; `deploy.sh:37,72-85`). `ops-recovery.md` recorded 200 `{"status":"ready",...}` on `/api/ready` at 06:21Z. The earlier blanket claim in this ledger that "`/readyz` is the SPA fallback" was **only ever true of port 9119** - on the app-server port `/readyz` is the real gate, so the check must not be dropped |
| A14 | **Original user requirement:** autonomy completion must mean an external user result - e.g. a specific calendar event - so "done" can never be the gateway's own bookkeeping. (The node-1 `ITEMS` text is the assistant's decomposition of this, not the source: its "expose a way to record the outcome" and the `outcome ... --external-id` CLI are one *implementation means* of the requirement, never the requirement itself) | `src/autonomy/loop.rs:488,603,617,640` (30 s bounded `<cmd> cal get <id>`, `kill_on_drop`, whole-token item-key match in the parsed `description`) - this readback is the mechanism that makes completion mean an external result; `src/autonomy/mod.rs:54,194` (CLI `autonomy outcome <key> --status executed\|verified\|failed [--external-id <id>]`); doc contract at `mod.rs:51-56` | `src/autonomy/loop.rs::{item_key_matches_only_as_a_whole_token, readback_matches_the_parsed_id_and_the_whole_item_key - FAILED in gate1, readback_is_bounded_and_kills_a_hung_provider}`; the only real external result on record is `61b93aaa-a3fd-4182-9cad-483a6dc084b2@zele`, created **by the lead by hand**, not by the gateway | **PARTIAL.** The CLI existing is *partial evidence only* - it shows the means is wired, not that the contract holds. The contract itself is unproven: `autonomy outcome ... --status verified` has never been run in process, no `action_loop_outcomes` row has ever carried an external id, the one calendar event was hand-made. The three readback unit tests did run green in gate2 (one failed in gate1), so the mechanism is tested - but tested-mechanism is not the contract. Until an autonomous run records a real external result and the readback accepts it, autonomy completion still does not mean an external user result |
| A15 | "no goal exists" vs endless goal-continuation loop: two goal extensions loaded at once | Outside this repo: `~/.omo/agent/settings.json` (symlinked from `~/.omo/settings.json`) - the `git:github.com/code-yeongyu/pi-goal` package line removed | `lanes/ops-recovery.md` A15 (16:05Z) + `lanes/ops-recovery-a15/` (before/after settings, A/C/D probe logs, the written goal-store file per variant). **Primary proof is variant D**: real settings file, normal default discovery, no `--no-extensions`, no `-e` goal override - it wrote `extensions/goal/`, created no `extensions/pi-goal`, and `create_goal`/`update_goal` succeeded against that store. Variant C is the legacy-loaded control (wrote `extensions/pi-goal/`). **Variant A is invalid as proof** - it ran `--no-extensions -e .../omo-ai/plugin`, which disables `packages[]` discovery and so bypasses the exact change under test; it is retained only as a second data point | FIXED-OUTSIDE-REPO, proven by the D/C probes. Remaining (stated as UNPROVEN in the lane): a running senpi host holds its tool registry in memory, so the engine child `pid 53360` (started 11:31, before the 15:52 edit) still carries the legacy tools until it is restarted; no host was restarted, so the post-restart registry was not observed |

## B. Autonomy ledger, reporting, cron, lanes, runtime config, retention (B1-B17, 17 rows)

| ID | Requirement (defect being closed) | Implementation (in tree) | Test / evidence | Actual status |
| --- | --- | --- | --- | --- |
| B1 | Follower re-delivers the gateway's own just-finished turn as a fresh replay | `src/agent/omo_backend/follow.rs:65,192,203,217` (`is_gateway_owned_turn`), turn id set at `omo_backend.rs:1778,2233` | `src/agent/omo_backend/follow.rs::the_gateways_own_finished_turn_is_never_replayed_as_an_engine_turn` | TEST-PASSED (gate2) |
| B2 | Owner approval card outlives its turn; a late answer is silently dropped | `src/agent/omo_backend.rs:76` (`TurnAskScope`), `:1972`, `:102` (`expire_abandoned_proposal` conditional UPDATE); `src/discord/approval.rs:462-476,990` | `src/agent/omo_backend.rs::{a_turn_ending_cancels_its_approval_scope, a_turn_ending_expires_its_pending_approval_instead_of_leaving_it_approvable}`; `src/discord/approval.rs::proposal_lifecycle_drop_cleans_pending_and_expires_the_card` | TEST-PASSED (gate2). Residual: the row UPDATE and the card `ExpireApproval` are not atomic - a crash between them leaves a stale, unclickable card |
| B3 | Workspace preset provisioning re-reads the env var instead of the backend ledger override | `src/agent/omo_backend.rs:406` (`ensure_workspace_preset`), call sites `:487`, `:690` | `src/agent/omo_backend.rs::workspace_preset_provisioning_follows_the_backend_ledger_override` (`#[tokio::test]` only; the double-attribute that broke the gate compile is fixed) | TEST-PASSED (gate2) |
| B4 | One owner decision (card press + short reply) bumps precedent counters twice | `src/autonomy/ledger.rs:685` (`claim_proposal`: `UPDATE ... SET state = ? WHERE id = ? AND state = 'pending'`, a compare-and-set whose `rows_affected` decides the winner), `:532`, `:914`, `:724`, `:1180` | `src/autonomy/ledger.rs::{one_owner_decision_via_decide_then_waiter_counts_once, waiter_then_short_reply_also_counts_once, concurrent_card_press_and_short_reply_record_one_decision, deciding_a_missing_proposal_writes_no_precedent}` | TEST-PASSED (gate2). The CAS is load-bearing: the concurrency test uses two pools on one WAL file with `tokio::join!` and no sleeps, and a lost write would show as `approvals = 2`. Class-scope decisions deliberately do not claim |
| B5 | A triaged GitHub item appears twice in one action-loop batch | `src/autonomy/loop.rs:933` | `src/autonomy/loop.rs::triaged_github_item_appears_once_per_batch` | TEST-PASSED (gate2) |
| B6 | An unclosed or quote-unbalanced `[[measure` marker deletes the rest of the reply | `src/report/claims.rs:108` | `src/report/claims.rs::unclosed_measure_marker_keeps_rest_of_reply` | TEST-PASSED (gate2) |
| B7 | Precedent lookup queries a non-existent table and swallows the error | `src/context/pack.rs:140` (`lane_owned_worktrees`) | `src/context/pack.rs::precedent_lookup_queries_lane_owned_worktrees` | TEST-PASSED (gate2) |
| B8 | Tool-activity entries leak because only `StatusDriver` clears them | Two halves: `src/report/status.rs:48` (TTL, prune, clear on turn start/end) **and** `src/multiplexer/actor.rs:806` (clear on every turn cleanup, the first production caller) | `src/report/status.rs::tool_activity_clears_on_turn_end_and_bounds_ttl`; the actor wiring has no test seam | TEST-PASSED (gate2) (complementary halves from two lanes, not a conflict; the wiring itself is untested) |
| B9 | Claims pending at shutdown that are younger than 10 min stay pending forever | `src/report/claims.rs:484` (sweep all pending at startup); recovery failure now logged at `src/main.rs:1339` | `src/report/claims.rs::recover_stale_report_claims_sweeps_young_pending_claims_at_startup` | TEST-PASSED (gate2) |
| B10 | Out-of-window miss check never matches interval/once jobs | `src/cron/scheduler.rs:1452`, helper `:2949` (`is_scheduled_occurrence`) | `src/cron/scheduler.rs::{interval_and_oneshot_next_runs_are_occurrences, out_of_window_interval_run_is_reported_missed}` | TEST-PASSED (gate2) |
| B11 | A multi-target `deliver` string yields a bogus Slack session key | `src/cron/executor.rs:641` (`payload_session_key`) | `src/cron/executor.rs::multiple_destinations_use_only_first_session` | TEST-PASSED (gate2) |
| B12 | `kill_process_tree` can signal an unrelated process group after PID reuse | `src/cron/executor.rs:1457` (snapshot `:1221`, candidates `:1338`, `authorize` `:1381`, `dispatch` `:1408`); only two `libc::kill` sites, both behind the revalidation callback | `src/cron/executor.rs` `mod process_tree_tests`: `missing_snapshot_yields_no_candidates`, `own_group_and_init_are_never_group_candidates`, `candidates_are_leaves_first_with_the_root_group_once`, `reused_descendant_pid_is_skipped_even_when_it_leads_the_same_group_id`, `group_changed_after_the_snapshot_blocks_only_the_group_signal`, `reused_root_pid_is_not_signalled`, `vanished_processes_are_skipped`, `the_gateway_group_is_refused_at_dispatch_too`, `unreadable_group_state_skips_the_group_signal`, `process_table_keeps_the_start_time_after_three_numeric_columns` | TEST-PASSED (gate2). Stated limit: revalidation-to-kill is not atomic, and processes spawned after the snapshot are not covered |
| B13 | A pause survives `lane close`, so a reopened lane never runs | `src/lanes/runner.rs:139` (`UPDATE lane_pauses SET resumed_at = ? ... resumed_at IS NULL`) | `src/lanes/runner.rs::reopen_clears_a_pause_that_outlived_close` | TEST-PASSED (gate2) |
| B14 | Failed auto-worktree registration left an unowned worktree and rerouted the lane into the shared repo root | `src/lanes/cli.rs:560` + `block_lane_after_worktree_failure` `:611`; `runner.rs:519` (`discard_auto_worktree` returns `Result`) | `src/lanes/cli.rs::{registration_failure_blocks_the_lane_in_its_own_tree, worktree_cleanup_failure_is_surfaced_not_swallowed}` | TEST-PASSED (gate2). Known caveat: after cleanup the blocked lane's cwd is gone, so `lane reopen` stops at `restore_cwd`; `close --force` still works |
| B15 | Auto-worktree lane work lands only on an unmerged `lane/<key>` branch with no signal | `src/lanes/runner.rs:1163`, `:531` (`unmerged_lane_branch`), `src/lanes/dispatch.rs:214` (`unmerged_branch` is critical) | `src/lanes/runner.rs::{close_surfaces_the_unmerged_lane_branch, unmerged_branch_counts_only_work_beyond_the_recorded_base}`; `src/lanes/dispatch.rs::unmerged_branch_events_skip_the_wake_judgment` | TEST-PASSED (gate2). No auto-merge by design (owner decision) |
| B16 | `env::set_var("OMON_LANE_DEFAULT_LEAD")` on a multi-threaded tokio runtime | `src/runtime_config.rs` (`set_default_lead`/`default_lead`, OnceLock, first-write-wins, blank refused), `src/main.rs:875` | `src/runtime_config.rs::tests::test_default_lead_runtime_config`, `src/main.rs::headless_tests::test_default_lead_runtime_config_delegation` (both re-exec the test binary `--exact`) | TEST-PASSED (gate2) |
| B17 | New per-turn/per-item tables are never pruned | `src/storage/retention.rs`, `src/storage/mod.rs`, `src/main.rs` (`prune_storage_once` 1120, `join_retention_task` 1148/1156, `spawn_daily_retention_task` 1182; both gateway modes) | `src/storage/retention.rs::{test_prune_retention_only_removes_old_completed_rows, prune_rejects_a_zero_day_window, retention_window_defaults_and_refuses_a_degenerate_value}`; `src/main.rs::headless_tests::{test_spawn_daily_retention_task_cancels_cleanly, join_retention_task_aborts_and_reaps_a_task_that_ignores_shutdown}` | TEST-PASSED (gate2). The lead verified the abort+await helper by reading it: `join_retention_task_within` borrows the handle (`timeout(limit, &mut task)`), then aborts and awaits, so the call returns only once the future is dropped - `timeout(limit, task)` alone would detach the task and let the sweep race `pool.close()`. `source_health` is deliberately never pruned |

## C. Slack transport/egress, inbox, sessions (C1-C25, 25 rows)

| ID | Requirement (defect being closed) | Implementation (in tree) | Test / evidence | Actual status |
| --- | --- | --- | --- | --- |
| C1 | `/omo-title` depends on a `thread_ts` Slack never sends in a slash payload | `src/slack/commands.rs:1160`, helper `:498` (`explicit_thread`), const `:36`; no-thread reply `TITLE_NO_THREAD` | `src/slack/commands.rs::{explicit_thread_accepts_ts_and_permalinks, title_without_thread_ts_names_the_thread_explicitly}` | TEST-PASSED (gate2). The earlier partial used `url::Url`, which is not a dependency; the permalink is now parsed by hand |
| C2 | `get_message` cannot fetch a thread reply (wrong root ts) | `src/tools/message_context_slack.rs:405`, root resolution `:312` | `src/tools/message_context_slack.rs::{get_message_rejects_a_reply_ts_as_a_thread_root, thread_reads_page_to_newest_and_get_message_finds_replies}` | TEST-PASSED (gate2). Correction recorded: the first pass used the reply ts as the root - fixed |
| C3 | Thread reads stop at the oldest 200 messages (context pack and provider halves) | `src/context/pack.rs:52,80,114`; `src/tools/message_context_slack.rs:248`; shared `SlackApi::conversations_replies_page` `src/slack/api.rs:220` | `src/context/pack.rs::{thread_reader_walks_every_page_by_cursor_without_repeating_the_parent, paginate_thread_replies_follows_the_cursor_and_dedupes_the_parent, paginate_thread_replies_without_a_cursor_advances_by_ts_and_cannot_loop}`; `src/tools/message_context_slack.rs::{thread_reads_follow_slacks_cursor_past_short_pages, thread_reads_page_to_newest_and_get_message_finds_replies}` | TEST-PASSED (gate2). Cursor-driven, parent-deduped, microsecond-ordered, page-bounded (50/20) |
| C4 | Channel backfill reads one page and infers "done" from page length | `src/slack/backfill.rs:255`, `MAX_PAGES` `:31`; `src/slack/api.rs:190` (`conversations_history_page`) / `:215` (`conversations_replies_page`) | `src/slack/backfill.rs::{history_backfill_reads_every_page_past_the_first, history_backlog_over_the_page_budget_holds_the_cursor_instead_of_skipping, a_2600_message_backlog_fits_the_production_budget, a_short_history_page_does_not_end_the_walk_when_slack_reports_more, thread_paging_follows_next_cursor_not_page_length, backfillable_keeps_every_message_when_the_walk_reached_the_cursor}` | TEST-PASSED (gate2). `write_cursor` only moves forward; an over-budget window processes nothing and holds the cursor |
| C5 | Reconcile targets grow with every thread session ever created | `src/slack/backfill.rs:46` (`THREAD_TARGET_WINDOW_DAYS` 7, `julianday(...) IS NULL` kept) | `src/slack/backfill.rs::reconcile_skips_thread_targets_idle_for_more_than_a_week` | TEST-PASSED (gate2) |
| C6 | Debouncer per-session lane map never pruned | `src/slack/debounce.rs:148` (`release_lane`, `Arc::strong_count == 2`) | `src/slack/debounce.rs::cancel_drops_pending_batch_and_flush_all_routes_the_rest` (extended with the empty-map assertion) | TEST-PASSED (gate2) |
| C7 | Startup backfill and the first reconcile can overlap | `src/slack/runtime.rs:30` (`backfill_lock`), `:85`, `:94` (`reconcile_once`) | `src/slack/runtime.rs::reconcile_pass_waits_for_the_startup_backfill` (barrier + `try_lock` + single `poll!`, no timed window) | TEST-PASSED (gate2) |
| C8 | `allow_all_users` falls back to `DISCORD_ALLOW_ALL_USERS` | `src/slack/mod.rs:171`; doc at `:97` | `src/slack/mod.rs::allow_all_users_never_inherits_the_discord_flag` | TEST-PASSED (gate2) |
| C9 | Approval decision label escapes the user mention | `src/slack/egress.rs:146` (`mark_approval_decided`, mrkdwn verbatim) | `src/slack/egress.rs::mark_approval_decided_replaces_buttons_with_label` (extended) | TEST-PASSED (gate2) |
| C10 | Approval fallback text carries the agent reason unescaped (broadcast injection) | `src/slack/egress.rs:623` | `src/slack/egress.rs::approval_fallback_text_escapes_agent_reason` | TEST-PASSED (gate2) |
| C11 | `bound_chunks` byte cut can split an HTML entity | `src/slack/mrkdwn.rs:106` | `src/slack/mrkdwn.rs::bound_chunks_never_splits_an_html_entity` | TEST-PASSED (gate2) |
| C12 | A channel read from a team-less session indexed `guild_id NULL` and was classified `dm` | `src/tools/message_context_slack.rs:187` (`conversation_guild`), used `:151`, `:366` | `src/tools/message_context_slack.rs::channel_reads_from_teamless_sessions_record_the_channel_team` | TEST-PASSED (gate2). Honest correction: the r1 trigger (DM session reading a channel) is unreachable - `authorize` refuses it; the reachable variant is fixed and the DM path stays `NULL` on purpose |
| C13 | GitHub CI-failure items filed as `fyi` when the PR is older than 24 h | `src/inbox/triage.rs:301`, `Item::is_ci_failure` `:104` | `src/inbox/triage.rs::github_ci_failure_not_dropped_as_stale_when_pr_older_than_24h` | TEST-PASSED (gate2) |
| C14 | Slack prefilter tested the owner mention in the truncated snippet while SQL tested full content | Root fix in the source: `src/inbox/sources/slack.rs:59,88`; prefilter trusts the classification again `src/inbox/prefilter.rs:179` | `src/inbox/sources/slack.rs::long_channel_message_mentioning_owner_past_snippet_cap_is_collected`, `src/inbox/prefilter.rs::slack_keeps_mention_room_type_when_snippet_truncated`, updated `collects_dm_mention_and_owner_thread_reply_only` | TEST-PASSED (gate2) |
| C15 | `inbox collect --source github` polls twice and prints the first poll | `src/inbox/cli.rs:142`, `PrePolledSource` `:105` | `src/inbox/cli.rs::pre_polled_source_commits_without_repolling` | TEST-PASSED (gate2) |
| C16 | Slack home-channel session-status replies never reach the private owner path; dashboard passes the Discord owner id | `src/slack/ingress.rs:268` (`private_owner_conversation`), `src/sessions/status_reply.rs:88`, `src/dashboard.rs:1303,1336,1452` (`status_reply_author`) | `src/slack/ingress.rs::session_status_in_the_home_channel_is_a_private_owner_conversation`; no `dashboard.rs` test (only the author lines are in scope) | TEST-PASSED (gate2) |
| C17 | A probe failure hides a host whose listing worked | `src/sessions/overview.rs:153` | `src/sessions/overview.rs::format_overview_shows_listing_sessions_when_probe_fails` | TEST-PASSED (gate2) |
| C18 | A failed snapshot restarts the 30-minute question wait | `src/sessions/watch.rs:214` (`retain_first_seen`, structural `notice_machine`) | `src/sessions/watch.rs::{failed_snapshot_does_not_restart_question_wait, failed_machine_keeps_only_its_own_timers, notice_machine_reads_the_target_segment}` | TEST-PASSED (gate2) |
| C19 | `mark_sent` failure after a successful send re-notifies the same question | `src/sessions/watch.rs:398` (`sent_fallback`), checked `:331` | `src/sessions/watch.rs::mark_sent_failure_after_a_successful_send_does_not_send_again` (real `serve` loop, `start_paused`, abort trigger, bounded timeout) | TEST-PASSED (gate2). Process-local only: a restart can still re-send |
| C20 | Tool-call-only assistant entries stored as raw JSON | `src/sessions/external.rs:647` (`entry_text`) | `src/sessions/external.rs::assistant_entries_with_only_tool_calls_as_json_string_are_skipped` | TEST-PASSED (gate2) |
| C21 | Probe silently runs locally when the machine name does not parse as a `Target` | `src/sessions/probe.rs:99` (`Probe::argv` returns `Result`) | `src/sessions/probe.rs::unparsable_machine_is_an_error_not_a_local_run` | TEST-PASSED (gate2) |
| C22 | `probe.py` reports an ambiguous session as "not found", surfacing as "not running - kept 24 h" | `src/sessions/probe.py:198` (match on `session_id`/`routing_id`, exit 3 not found, exit 5 ambiguous) | none (python helper, ships via `include_str!`, exit-code only) | TEST-PASSED (gate2) (no test by construction) |
| C23 | `answers_for` sends the same text to every question in a multi-question request | `src/sessions/probe.rs:256` (`ControlError::Ambiguous`) | `src/sessions/probe.rs::{multi_question_request_is_rejected_instead_of_repeating_one_answer, test_answers_for_mapping}` | TEST-PASSED (gate2). Per-question selection was checked against every existing answer surface and is not part of the contract, so rejection stands |
| C24 | `release` needs the full id after the session ended; `adopt` prints the unresolved target | `src/sessions/cli.rs:226`, `src/sessions/external.rs:168` (unique `starts_with` fallback, not SQL `LIKE`) | `src/sessions/external.rs::release_finds_the_adopted_target_by_unique_prefix`; adopt's printed target has no test (same `target()` accessor) | TEST-PASSED (gate2) |
| C25 | Every read/send runs an extra uncached `omo thread list` (ssh for remote hosts) | `src/sessions/control.rs:404,440` (`is_canonical_session_id` `:62`), `src/sessions/external.rs:105` | `src/sessions/control.rs::{canonical_read_skips_the_session_listing, prefix_read_still_resolves_through_the_listing, canonical_send_skips_the_session_listing, prefix_send_still_checks_the_send_capability}` | TEST-PASSED (gate2). `interrupt`/`answer` still resolve unconditionally by design |

## D. Tracker backend and dashboard (D1-D30, 30 rows)

| ID | Requirement (defect being closed) | Implementation (in tree) | Test / evidence | Actual status |
| --- | --- | --- | --- | --- |
| D1 | A pre-existing user team with key `SES` is taken over as the Sessions system team | Backend `src/tracker/seed.rs:90,98` (`SESSIONS_TEAM_SQL`, `free_sessions_team_key` SES..SES99); web consumer `web/src/tracker/store.tsx:63-70,366-372`, `TrackerApp.tsx:61-67`, `pages/TeamIssuesPage.tsx:48-54` | `src/tracker/seed.rs::{user_team_with_ses_key_is_not_taken_over_as_sessions_team, sessions_team_key_moves_past_every_key_a_user_team_already_owns}`; web: no sibling test, `tsc` only | TEST-PASSED (gate2) for the backend half - both seed tests ran green in gate2; web half still tsc-only, no behavioural test. Two lanes, complementary halves |
| D2 | System-team rejection message talks about lane statuses even for the Sessions team | `src/tracker/workspace.rs:901` | none (server-sent text, no machine consumer) | TEST-PASSED (gate2) (no test by design) |
| D3 | `create_project` not transactional (orphan project behind a 400) | `src/tracker/workspace.rs:365`, `set_project_links` takes `&mut SqliteConnection` | `src/tracker/workspace.rs::create_project_with_unknown_team_leaves_no_project_behind` | TEST-PASSED (gate2) |
| D4 | `update_label` allows indirect parent cycles | `src/tracker/workspace.rs:781` (`WITH RECURSIVE` walk) | `src/tracker/workspace.rs::update_label_rejects_indirect_parent_cycle` | TEST-PASSED (gate2) |
| D5 | Deleting a session issue is recreated by sync under a new id | `src/tracker/issues.rs:898` (one transaction: detach children, drop notifications, tombstone, delete), `sessions.rs:156,735-741` | `src/tracker/issues.rs::removing_a_session_issue_keeps_the_next_sync_from_recreating_it` (runs the real `sync_from_overview`) | TEST-PASSED (gate2) |
| D6 | Team move writes an unvalidated `state_id` and can fail half-way | `src/tracker/issues.rs:527` | `src/tracker/issues.rs::team_move_rejects_a_state_from_the_old_team_and_writes_nothing` | TEST-PASSED (gate2) |
| D7 | Issue parent indirect cycles allowed | `src/tracker/issues.rs:748` (`WITH RECURSIVE` over `parent_id`) | `src/tracker/issues.rs::parent_cycle_of_two_issues_is_rejected` | TEST-PASSED (gate2) |
| D8 | `hydrate` reads the whole `tracker_issue_labels` table on every load | `src/tracker/issues.rs:83` (`linked_rows`, full scan only above 200 issues) | `src/tracker/issues.rs::hydrate_filters_by_issue_id_only_for_small_loads` | TEST-PASSED (gate2) |
| D9 | Session cache mutex held during ssh collection; non-atomic publish | `src/tracker/sessions.rs:121` (collect outside the lock, publish + `remember()` under one guard) | `src/tracker/sessions.rs::stale_and_out_of_order_collections_never_publish` | TEST-PASSED (gate2) |
| D10 | `lane_action` maps every error to 409 | `src/tracker/threads.rs:367` (404 / 409 `lane_command_failed` / 400 / default 500) | `src/tracker/threads.rs::lane_action_errors_map_to_404_400_and_409` | TEST-PASSED (gate2) |
| D11 | Command palette acts on a stale focused issue from the previous page | `web/src/tracker/TrackerApp.tsx:135,144` (`setFocusedIssueId(null)` on route change) | none (no DOM harness) | VERIFIED-EXECUTED (web, browser). `ui-final/results-final.json` D11 x2 ok: desktop navigation clears the focused issue (`chipBefore="OMO-101 Watched only by subscription"` -> `chipAfter=""`, `statusCmdBefore=true` -> `false`, rows in 344 ms, focus cleared in 6 ms) and the mobile 390 px case shows `scrollW=390/clientW=390` with no overflow; `ui-final/d11-palette-cleared-desktop.png`, `d11-mobile-palette-cleared.png`. Mock limitations retained (hand-written mock, Vite sources, macOS WebKit only) |
| D12 | The 10-second poll can undo a fresh edit or resurrect a deleted issue | `web/src/tracker/store.tsx:162-215` (`MutationGuard`, `readGuarded`), `:283-340`, `:393-458`, `:576` | `web/src/tracker/store.test.ts` - 7 pass / 0 fail / 17 assertions (deterministic deferred promises); included in the lead's `bun test` 146 pass / 0 fail | VERIFIED-EXECUTED (web). Pre-fix logic was replayed and accepted the stale rows, so the test is load-bearing |
| D13 | Non-JSON error responses become a JSON `SyntaxError` | `web/src/tracker/api.ts:53-71` (`TrackerApiError`, `parseError`, `bodyText`) | none in-tree; the lane reports a stubbed-fetch runtime check (plain-text 403, 502 HTML, JSON 409/500, 200 non-JSON) that left no artifact | FIXED-UNRUN (web, tsc-only). No browser case exercised the non-JSON error path - `results-final.json` has no D13 entry - so the stubbed-fetch runtime check in the lane report remains the only evidence and left no artifact |
| D14 | A malformed percent-escape in the URL blanks the app | `web/src/tracker/router.ts:41,52` (`safeDecode`) | none in-tree; the lane reports a `bun` runtime check (`parseRoute('/issue/%E0')`) that left no artifact | VERIFIED-EXECUTED (web, browser). `ui-final/results-final.json` D14 ok: a malformed `%E0` route still renders the shell (`shellIn=109ms`, `rootClass="ls-root"`, `bodyChars=63`, `uriErrors=[]`) instead of blanking the app; `ui-final/d14-malformed-url.png`. Mock limitations retained |
| D15 | Tab title stays `"... - Omon"` after returning to the dashboard | `web/src/tracker/TrackerApp.tsx:167-174` (mount-only effect restores `document.title`) | none (needs a DOM render) | VERIFIED-EXECUTED (web, browser). `ui-final/results-final.json` D15 ok: leaving the tracker restores the dashboard title (`trackerTitle="My issues - Omon"` -> `restoredIn=6ms` -> `dashboardTitle="omon gateway dashboard"`, `dashboardMounted=true`, and back again `"My issues - Omon"`); `ui-final/d15-title-restored.png`. Mock limitations retained |
| D16 | Due-date shortcuts and the due-this-week filter are off by a day across DST | `web/src/tracker/format.ts:93-98`, `store.tsx:497-499` (calendar math) | none in-tree; the lane reports a `TZ=America/New_York` runtime comparison (2026-11-02 vs 2026-11-01) that left no artifact | FIXED-UNRUN (web, tsc-only) |
| D17 | Lane panel offers actions the backend always rejects | `web/src/tracker/detail/AgentPanel.tsx:23-34` (`ALLOWED`), matched to `src/lanes/cli.rs:216-221,274-280`, `src/lanes/state.rs:55-56`, `src/storage/lanes.rs:190` | none (literal table, no behaviour) | FIXED-UNRUN (web, tsc-only) |
| D18 | A failed description save is never retried and the text is discarded | `web/src/tracker/detail/descriptionDraft.ts:17-135` (`createDraftSaver`, retention ownership), `Editors.tsx:68-140,166-173,203-211` | `web/src/tracker/detail/descriptionDraft.test.ts` - **final run: **14 pass / 0 fail** (the file grew from 9 to 14 cases after the audit), and the whole suite: **151 pass / 0 fail** from `web/` (gate2 `web-test-raw`), plus `tsc -p tsconfig.app.json --noEmit` exit 0. That supersedes the lane's earlier 144-pass partial (taken before the D18 rewrite) and the 130-pass figure in `web-test-alias.md`. Both D18 failure modes were red-checked against reverted copies (2 fail / 5 pass and 2 fail / 7 pass) | VERIFIED-EXECUTED (web, browser A/B/C + 14 unit tests). Unit counts refreshed: `descriptionDraft.test.ts` now holds **14** tests (was 9) and the whole web suite is **151 pass / 0 fail** (gate2 `web-test-raw`), not the earlier 9 / 146. Browser evidence is `ui-final/results-d18-postfix.json` (byte-identical to `results-d18-final3-2026-10-20T08-37-35-754Z.json`), stamped `2026-10-20T08-37-35-754Z`, carrying `"authoritative": true`, `"verdict": "PASS (A, B, C)"`, and a `SETUP` check `tree unchanged during the run` that is `ok: true` with source hashes `descriptionDraft.ts 6685:ed1c1c63be5b35dd` / `Editors.tsx 8012:93703609741dd5a3` - so the pass belongs to the frozen source. Case A: a delayed failed save keeps the draft through close/reopen (`Not saved` in 45 ms, reopen in 1 ms). Case B: a **trusted click on Retry saves exactly once** - `patchCount=1`, one PATCH then one GET, bar cleared in 30 ms, stored text confirmed. Case C: reopening while the retry PATCH is still in flight keeps the draft. **The Retry wiring is therefore no longer the open gap it was**; the rule-level 9 cases and the 146-pass suite are still the unit evidence behind it |
| D19 | Create modal preselects the system Sessions team | `web/src/tracker/create/CreateIssueModal.tsx:64-79,94-96,128-134,221,418-423,474` - `selectableTeams` is strictly non-system (`t.teams.filter((team) => !team.system)`), the old `normal.length > 0 ? normal : t.teams` fallback is gone, and the **no-regular-team edge is handled in the current code**: `noTeam` (`:96`) renders an actionable notice (`:418-423`, `.ci-notice` in `create.css:114-124`), the picker reports `No regular team` (`:221`), Create is disabled (`:474`), and `submit()` returns with an explicit error toast instead of silently doing nothing (`:128-134`), so `mod+enter` cannot submit a system team either | none (no sibling test module for this file; `tsc` covers the types only) | VERIFIED-EXECUTED (web, browser). `ui-final/results-final.json` D19 x3 ok: system-only workspace blocks creation with the actionable notice and `createDisabled=true, createPosts=0`; the mobile 390 px case shows the 146-char notice with no overflow; a mixed workspace seeds a regular team (`team="O\nOMO"`). `ui-final/d19-system-only-desktop.png`, `d19-system-only-mobile.png`. This describes the current tree, not the earlier partial that fell back to `t.teams`. Mock limitations retained |
| D20 | Home directory hard-coded in session path shortening | `web/src/tracker/detail/SessionAgentPanel.tsx:33` (`/^\/(?:Users\|home)\/[^/]+(?=\/\|$)/`) | none | FIXED-UNRUN (web, tsc-only) |
| D21 | `lane_not_verified` rejection shows two toasts | `web/src/tracker/store.tsx:352` (skip `reportError` for that code); `issues/model.ts:168-183` already emits the one actionable toast | none | FIXED-UNRUN (web, tsc-only). The code change lives in the shell lane's file; if it is dropped the duplicate returns |
| D22 | Timeline day labels shift after DST fall-back | `web/src/tracker/issues/TimelineView.tsx:59-62` (calendar day step) | none | FIXED-UNRUN (web, tsc-only) |
| D23 | "Searching..." stays after the query is cleared mid-request | `web/src/tracker/pages/SearchPage.tsx:49` (`setLoading(false)`) | `.omo/evidence/review-20261020/ui/` - browser harness: label cleared in 263 ms while the mock still held the request pending; counterfactual (line removed) left the label for 5 s; `ui/results-fixed.json` 11/11, `ui/d23-cleared.png` | VERIFIED-EXECUTED (web, mock backend, Vite sources - not the shipped bundle) |
| D24 | System theme button does not follow the system theme | `web/src/tracker/store.tsx:32,80-81,269,342,344` (`ThemePreference = Theme \| 'system'`, live `matchMedia` listener), `pages/SettingsPage.tsx:129-135` (aria-checked/is-on on `themePreference`) | none | VERIFIED-EXECUTED (web, browser). `ui-final/results-final.json` D24 x2 ok: System follows a live `prefers-color-scheme` flip without reload (`startTheme=dark`, `systemCheckedIn=1ms`, `stored="system"`, `lightIn=99ms`, `darkIn=230ms`, `systemStillChecked=true`) and an explicit Light choice ignores the OS change (`themeAfterOsFlipToDark=light`); `ui-final/d24-system-dark.png`, `d24-system-light.png`. `web-pages.md`'s "not re-verified in this pass" is superseded. Mock limitations retained |
| D25 | Clearing a project name leaves an empty field that is never reverted | `web/src/tracker/pages/ProjectPage.tsx:261` (`required` prop) | UI harness: empty draft reset in 1 ms, **zero** PATCH requests; control rename still commits; counterfactual fired 1 PATCH with the empty draft | VERIFIED-EXECUTED (web, mock backend) |
| D26 | The Subscribed tab on My issues is an approximation | `src/tracker/model.rs:164` (`subscriber_ids`), `src/tracker/issues.rs:124,1041`; `web/src/tracker/types.ts:158`, `pages/MyIssuesPage.tsx:30-31` | `src/tracker/issues.rs::loaded_issues_carry_their_subscribers` | VERIFIED-EXECUTED (web, browser) + TEST-PASSED (gate2) for the backend. `ui-final/results-final.json` D26 ok: the Subscribed tab filters on the backend field (`subscribedRows=["OMO-101","OMO-103","OMO-104"]`, `createdRows=["OMO-102"]`, `payloadCarriesSubscriberIds=true`); `ui-final/d26-subscribed-desktop.png`. Backend half `loaded_issues_carry_their_subscribers` green in gate2. Still unexercised: a real subscribe round-trip against the Rust server. `web-pages.md`'s older "needs backend payload work" claim is obsolete. Mock limitations retained |
| D27 | A failed picker update shows two error toasts | `web/src/tracker/ui/pickers.tsx:38-56` (`useApply` swallows its generic toast), `store.tsx:350-352`, `shell/CommandPalette.tsx:96-110` raises the actionable one | none | VERIFIED-EXECUTED (web, browser). `ui-final/results-final.json` D27 x3 ok: a `lane_not_verified` 409 raises exactly one actionable toast (`toasts=[{"text":"lane must be verified before closing; use force close Force close","role":"alert"}]`), Force close retries once with `force_note` and raises **no** second toast (`toasts=0`), and a plain 500 still shows exactly one generic toast (`["tracker unavailable"]`); `ui-final/d27-lane-not-verified-toast.png`. `web-pages.md`'s "remaining duplicate" note is obsolete. Mock limitations retained |
| D28 | New cycle dialog reuses stale default dates after a cycle is created | `web/src/tracker/projects/cycles.tsx:165-173` (re-seed on closed->open) | UI harness: first open `2026-10-20..2026-11-03`; after creating "Cycle one" the reopened dialog showed `2026-11-03..2026-11-17`; counterfactual showed the stale overlap; `ui/d28-dialog.png`, `ui/d28-reopened.png` | VERIFIED-EXECUTED (web, mock backend) |
| D29 | Adding a milestone has no in-flight guard | `web/src/tracker/projects/Milestones.tsx:40,49-61,132` (`saving` state) | UI harness: one Enter queued exactly one request, button `disabled`, second Enter left the pending queue at 1; counterfactual created `M1,M1`; `ui/d29-inflight.png` | VERIFIED-EXECUTED (web, mock backend) |
| D30 | `bun test` in `web/` fails because it cannot resolve `@/` and `@hermes/shared` | `web/bunfig.toml:9` (preload), `web/test-preload.ts:30` (`vi.stubGlobal`/`vi.unstubAllGlobals`), `web/src/lib/ws-url.ts:1`, `web/src/lib/chat-sidebar-session-params.ts:12`, `web/src/lib/api.ts:1`; the gate's PREEXISTING exemption was deleted from `.omo/evidence/review-20261019/gate.sh:23` (I confirmed the file has no exemption block and the comment says so) | `web/src/lib/ws-url.test.ts` (4 cases), `web/src/lib/chat-sidebar-session-params.test.ts` (4 pre-existing cases, import path only); current-tree run **146 pass / 0 fail, exit 0** under monitor `mon_DZ0DM6R1P3MFTMNE` | VERIFIED-EXECUTED (web) with raw tool provenance: the monitor id and exit 0, not just a prose claim. Older figures in the lane reports (130 pass in `web-test-alias.md`, 144 in `web-detail.md`) are superseded; no stored web-test log file exists |

## E. Effect classification and Discord (E1-E20, 20 rows)

| ID | Requirement (defect being closed) | Implementation (in tree) | Test / evidence | Actual status |
| --- | --- | --- | --- | --- |
| E1 | `gh api` mutations (`-X POST/PUT/PATCH/DELETE`, `-f/-F/--input`) classified Read; `gh api graphql` heuristic | `src/autonomy/effect.rs:1198` (`extract_gh_method` 1240, `gh_api_mutating_arg` 1256, `graphql_document` 1282, `graphql_operation` 1304, `graphql_declares_write` 1326) | `src/autonomy/effect.rs::{e1_gh_api_mutations_are_not_read, e1b_graphql_read_only_requires_an_actual_query_document}` | TEST-PASSED (gate2) |
| E2 | `git clean -f/-d/-x` classified Read | `src/autonomy/effect.rs:1130` (dry-run only is safe) | `src/autonomy/effect.rs::e2_git_clean_clusters_are_delete` | TEST-PASSED (gate2) |
| E3 | Redirections without a space, and fd redirections, not detected | `src/autonomy/effect.rs:1510` (quote-aware scan, `read_shell_word` 1596) | `src/autonomy/effect.rs::{e3_attached_and_fd_redirections_are_writes, e3b_redirection_scanner_keeps_quote_context_and_chains}` | TEST-PASSED (gate2) |
| E4 | `sqlite3` with `-cmd`, extra SQL, dot-commands, `writefile`, `ATTACH` classified Read | `src/autonomy/effect.rs:688` | `src/autonomy/effect.rs::e4_sqlite3_side_effects_are_not_read` | TEST-PASSED (gate2) |
| E5 | eval cells using unlisted JS fs mutation APIs classified Read | `src/autonomy/effect.rs:270,316` | `src/autonomy/effect.rs::e5_eval_fs_mutations_are_not_read` | TEST-PASSED (gate2) |
| E6 | A compound command / eval cell inherits only the worst segment signature | `src/autonomy/effect.rs:566`, `stamp_signature` 246 | `src/autonomy/effect.rs::e6_compound_signature_covers_every_segment` | TEST-PASSED (gate2) |
| E7 | Unknown signatures carry only program+target, so one "always" allows any command | `src/autonomy/effect.rs:110` | `src/autonomy/effect.rs::e7_unknown_signature_carries_the_command` | TEST-PASSED (gate2) |
| E8 | `git push` protected-branch/force detection checks one refspec and exact flag spellings | `src/autonomy/effect.rs:1148`, `push_refspec_is_protected` 872 | `src/autonomy/effect.rs::e8_push_checks_every_refspec_and_force_spelling` | TEST-PASSED (gate2) |
| E9 | `sed -i` / `--in-place` / `w` command classified Read | `src/autonomy/effect.rs:750`, `sed_scripts` 885, `sed_script_writes_file` 914 | `src/autonomy/effect.rs::e9_sed_in_place_and_write_commands_are_writes` | TEST-PASSED (gate2) |
| E10 | `git diff/log/show --output=<file>`, `git grep -O`, `--ext-diff`, `reflog expire` classified Read | `src/autonomy/effect.rs:1035` | `src/autonomy/effect.rs::e10_git_read_subcommands_with_output_are_not_read` | TEST-PASSED (gate2) |
| E11 | A relative `cd` is not resolved against the current cwd, so a Delete target loses its repo | `src/autonomy/effect.rs:1486` (`join_cwd`) | `src/autonomy/effect.rs::e11_relative_cd_resolves_against_the_current_cwd` | TEST-PASSED (gate2) |
| E12 | `is_spend_invocation` substring matches classify reads as Spend | `src/autonomy/effect.rs:1350` (program name, command-word pairs, `--pay` only) | `src/autonomy/effect.rs::e12_spend_matches_tokens_not_substrings` | TEST-PASSED (gate2) |
| E13 | The `rm(` needle matches `transform(`/`confirm(`/`perform(` | `src/autonomy/effect.rs:322` (identifier boundary) | `src/autonomy/effect.rs::e13_rm_needle_requires_an_identifier_boundary` | TEST-PASSED (gate2) |
| E14 | `cargo test/check/clippy/bench`, `npm/bun/pnpm test/run`, `make` classified Read | `src/autonomy/effect.rs:821,841` | `src/autonomy/effect.rs::e14_build_and_test_commands_are_not_read` | TEST-PASSED (gate2) |
| E15 | Discord replies drop extracted `[[measure]]` claims, so they are never re-measured | `src/discord/adapter.rs:3543` (`extracted_claims`), `spawn_claim_checks` 3587, `DiscordClaimReplyDispatcher` 3619, `message_id` on `ActiveDiscordStream` | `src/discord/adapter.rs::final_stream_records_measure_claims_for_re_measurement` (row recorded against message 42, correction `정정:` once, replies to 42) | TEST-PASSED (gate2). Records only when the egress has a pool; main wires `DeadTargetRegistry::with_pool`, so production is covered |
| E16 | Discord `SendMessage`/`EditMessage` do not strip `[[measure]]` markers | `src/discord/adapter.rs:3733,3947` via `strip_claim_markers` 879 | `src/discord/adapter.rs::{send_message_strips_measure_markers, claim_marker_stripping_follows_the_injected_flag}` | TEST-PASSED (gate2) |
| E17 | Approval-button owner gate and owner-reply steering hard-code the owner id | `src/discord/adapter.rs:1400,1630`, pure helpers `:4399,4403` (`configured_owner_id`) | `src/discord/adapter.rs::approval_owner_gate_uses_the_configured_owner_id` | TEST-PASSED (gate2). `adapter.rs:912` still uses `inbox::proposals::OWNER_ID` deliberately: that is the proposals routing key, not the approval gate |
| E18 | Status message shows the raw error | `src/multiplexer/actor.rs:819` (`owner_failure_reason` 1467) | `src/multiplexer/actor.rs::status_failure_reason_never_leaks_the_raw_error` | TEST-PASSED (gate2) |
| E19 | Publish button `custom_id` is longer than Discord's 100-char limit | `src/discord/buttons.rs:194` (`compact_custom_id` 378, `parse_button_custom_id` 210): 110 -> 93 chars, lossless 64-hex -> `b64-<43 base64url>` | `src/discord/buttons.rs::{over_long_button_id_round_trips_through_the_compact_form, rerendered_buttons_send_the_compact_id_and_keep_the_token}`; the 110->93 figure was recomputed independently | PARTIAL - both halves now tested, the cross-repo end-to-end still unobserved. Gateway half: TEST-PASSED (gate2), `over_long_button_id_round_trips_through_the_compact_form` and `rerendered_buttons_send_the_compact_id_and_keep_the_token`. Producer half: the `content-intel-dashboard` change to `scripts/publish-ready-digest.ts:393-403` now runs **27 passed, 0 failed** in that repo's direct suite. What is still missing is the observation itself: no real Discord publish has been sent with the new id, so "the message is accepted and a click resolves" has never been seen end to end. Both repos' changes are uncommitted |
| E20 | A Slack flood can evict pending Discord pairing codes | `src/discord/pairing.rs:344` (`pending_scope_clause` 595: `user_id LIKE 'slack:%'` vs `NOT LIKE`) | `src/discord/pairing.rs::slack_flood_does_not_evict_pending_discord_codes` | TEST-PASSED (gate2) |

## Review criteria - final pre-deploy state

The plan's five criteria, judged on evidence in this file (not on lane claims):

| # | Criterion | State |
| --- | --- | --- |
| 1 | R1 review harvested and citations resolved | **SATISFIED** - 26/26 lane reports, `r1-citations.md` resolves 135 Location references; `CONFIRMED-r1.md` lists F1-F36 with regrades |
| 2 | Every confirmed fix landed and re-reviewed | **SATISFIED** - all 107 requirement rows have an implementation and an evidence path; gate is `GATE-PASS`; delta re-review reports VERIFIED for every scoped fix |
| 3 | One integration gate green | **SATISFIED** - `review-20261020-final` `GATE-PASS` (fmt 0, clippy 0, test 0 with 2071 passed, web-test 151 pass / 0 fail, tsc 0, build 0) |
| 4 | Pushed to `origin/main` | **PENDING** - no new commit exists; HEAD is still `5fcfc08` |
| 5 | Deployed from `main` and verified live | **PENDING** - `DEPLOYED rev=5fcfc08`; S5/S6 probes not re-run for this tree |

**Criterion 2 needs one qualification.** Coverage exists - every one of the 107 rows is mapped to code
and to a test or evidence artifact - but three rows describe behaviour that has **never been observed
end to end**: **A1** (a calendar registration counts as done only when the agent records the external
result id), **A5** (kakao archive item -> schedule handling link), **A14** (autonomy completion must
*mean* an external user result). Their unit tests are green and the readback mechanism is tested, and
the one real calendar event exists - but it was created by hand, and the engine adoption that would
produce a genuine autonomous result is **refused at the owner gate with exit 2**, by design, with no
bypass and no rows written. So "fixed" is accurate for those rows; "proven live" is not.

## Final required ship steps - gate passed, shipping not started

These are separate from the 107 rows. Each is a state I checked directly, not a lane claim.

| # | Required end state | Current measured state | Status |
| --- | --- | --- | --- |
| S1 | One commit containing the whole fix set (single commit, not a series) | HEAD is still `5fcfc08`; the frozen tree is dirty (86 tracked source paths + 11 untracked source files). Gate is green on that tree | PENDING (source frozen, commit not made) |
| S2 | `origin/main` == HEAD after push | `origin/main..HEAD` = 0 and `HEAD..origin/main` = 0 at `5fcfc08`; the new commit does not exist yet | PENDING |
| S3 | Deploy from `main` with `scripts/deploy.sh` (signed binary) | `~/.omon/bin/DEPLOYED` = `rev=5fcfc08 dirty_paths=0 deployed_at=20261020-112444` - the previous revision, **not** this tree | PENDING |
| S4 | `DEPLOYED` rev == new HEAD with `dirty_paths=0` | Same: rev is `5fcfc08` while the frozen tree is dirty (86 tracked + 11 untracked source paths), so the stamp cannot be refreshed truthfully until the commit lands and the deploy runs from `main` | PENDING |
| S5 | Readiness: **app-server** `/readyz` 200 **and** dashboard readiness both pass | Two probes, both required, on different ports. (a) App-server: `curl http://127.0.0.1:19742/readyz` must be 200 - exactly what `deploy.sh:129` gates on and what `deploy.sh:17` names as a verification condition. (b) Dashboard on 9119: `GET /api/ready` (`src/dashboard.rs:788`) returns `{"status":"ready",...}`; `ops-recovery.md` recorded 200 at 06:21Z on the deployed `5fcfc08`. Note the port matters: on **9119**, `/readyz` is not a route and falls through to the SPA HTML, so a 9119 `/readyz` 200 proves nothing - that observation never applied to 19742. Neither probe has been re-run against this tree; both must be checked after the new deploy | PENDING (for the new rev) |
| S6 | Slack ready after redeploy | `ops-recovery.md` recorded `slack socket: connected` + `hello from applink-2` at 04:18:42Z, and three `Agent is already processing` collisions later the same morning. No post-deploy check exists for this tree | PENDING |
| S7 | `gate.sh` GATE-PASS | `review-20261020-final` (18:02) **did** write `GATE-PASS` (fmt 0, clippy 0, test 0 with 2071 passed, web-test 151/0, tsc 0, build 0) - but 4 source files were edited at 17:52-18:02, i.e. after that run compiled, so the pass describes a tree that no longer exists. Current tree **fails fmt** (2 hunks in `omo_backend.rs`); the fix is delegated. **`review-20261020-drift` is the active run** | **STALE - awaiting `review-20261020-drift`** |

## Conflicting / stale report claims found, and the resolution used

1. **A2**: `lanes/ops.md` (11:56) says `DIAGNOSED` and asks for a mail-lane fix; `lanes/main-runtime.md`
   (16:18) implements that fix. Newest wins; I confirmed `is_network_failure` in
   `src/inbox/health.rs` and `display_date` in `src/inbox/sources/mail.rs`.
2. **A4/A6/A12/A13/A15**: `ops-recovery.md` explicitly supersedes `ops.md` and its own header says
   so. `ops.md`'s A13 "Deploy confirmed" describes `5fcfc08`, which is no longer the working tree.
3. **C3 vs `src/slack/backfill.rs`**: `slack-egress.md` (16:18) says backfill "still walks threads
   with `oldest` and treats a short page as the end". The current source contradicts it:
   `backfill.rs:304-317` pages threads through `conversations_replies_page` on `next_cursor`, and
   `thread_paging_follows_next_cursor_not_page_length` (`:799`) asserts it. Source wins; that line
   of `slack-egress.md` is stale.
4. **B8**: two lanes each claim B8 (`report/status.rs` TTL and `multiplexer/actor.rs` wiring). Not a
   conflict - complementary halves of one requirement; the wiring half has no test.
5. **D1 / D21 / D26 / D27**: each has a backend or web half owned by a different lane, and each
   report contains a superseded sentence about the other half (`D26` "left unchanged", `D27`
   "remaining duplicate", `D24` "not re-verified"). The in-tree code is the newer truth and is what
   the rows above cite.
6. **D30**: `web-test-alias.md` reports 130 pass / 0 fail; `web-detail.md` reports 146 pass / 0 fail
   after its rewrite; the lead's run is 146 / 0. Newest wins. None of the three left a log file.
7. **Gate logs vs tree**: `fmt.log` (15:57) and `clippy.log` (16:07, started ~15:57:36) were taken
   before the last edits to all 21 fmt files and to the three clippy files (16:04:37, 16:07:44,
   16:07:51). The logs prove the tree failed *then*; they say nothing about the current text.
8. **A11**: `autonomy-loop.md` says migration 0054 is unapplied and treats it as a deploy-order
   constraint. The DB fact is right (`max(version)=53`, `action_loop_outcomes` absent) but the
   conclusion was too strong: 0054 is embedded via `sqlx::migrate!("./migrations")`
   (`src/storage/db.rs:15`) and applied by `MIGRATOR.run` at `db.rs:190` on connect, so the new
   binary migrates at startup. The lane's "apply 0054 before deploying" step is unnecessary.
9. **`/readyz` is a port question, not a wrong URL** - my earlier blanket "`/readyz` is the SPA" was
   too broad and is corrected here. `scripts/deploy.sh` probes **two** targets: the app-server at
   `APPSERVER_HTTP/readyz` (`deploy.sh:41,129`, derived from `ws://127.0.0.1:19742`) where 200 is a
   hard deploy condition, and the dashboard at 9119 where the readiness route is `GET /api/ready`
   (`src/dashboard.rs:788`) plus `index.html` byte-match and 200 on `/api/health` and
   `/api/tracker/bootstrap` (`deploy.sh:37,72-85`). On 9119 alone, `/readyz` does fall through to the
   SPA HTML. The original requirement - a readyz 200 check - stands unchanged; the app-server probe
   is the one that satisfies it.
10. **A15 variant A is not proof** (`ops-recovery.md`, resolved in the same report): variant A ran
    `--no-extensions -e .../omo-ai/plugin`, which disables `packages[]` discovery and therefore
    bypasses the settings change under test. Variant D (real settings, normal discovery) is the
    primary proof. This is a superseded probe, not an open item.
11. **D18 / D19 earlier partials are superseded, not open**: `web-detail.md` reports 144 pass before
    the D18 rewrite; the final numbers are 14 focused cases plus 151 pass / 0 fail for the whole
    suite, and `web-test-alias.md`'s 130 pass is older still. D19's older description (a fallback to
    `t.teams` when no regular team exists) no longer matches the tree, which now has the `noTeam`
    notice, a disabled Create button and an explicit error toast.
12. **A6's "no cursor advance" reading is stale, not a defect**: `ops.md` measured the dead helpers
    `run_imessage_watch`/`run_cursor_watch` (no callers in `src/`). The live path is the `imsg watch`
    child (pid 53233, PPID 52955), so there is no polled cursor to advance. Kept as a correction of
    record, not as an open finding.
13. **My own first draft mis-keyed A1 and A11** (retracted): it described A1 as the log rename and
    A11 as the outcome record, i.e. it keyed both rows to what the lane did rather than to the
    original requirement. Corrected: A1 is the calendar end-to-end acceptance, A11 is the delivered
    bookkeeping. This is exactly the failure mode the keying rule above forbids.

## Uncovered live verification gaps (what is still unproven)

1. **The gate is green; nothing is deployed.** `review-20261020-final` is `GATE-PASS` with every
   step at exit 0 (2071 Rust tests, 151 web tests). That certifies the frozen tree, and it is the
   first run in this review where fmt, clippy, Rust tests, web tests, tsc and build are all green
   together. It says nothing about production: the deployed binary is still `5fcfc08`.
2. **Migration 0054 needs no ordering work.** The live DB sits at version 53 only because the
   running `5fcfc08` binary embeds no 0054. `MIGRATOR` (`src/storage/db.rs:15`) is
   `sqlx::migrate!("./migrations")` - compiled in - and `Database::connect` runs it at
   `db.rs:190` before the gateway serves, so a new binary applies 0054 on first startup. Not a gap.
3. **Nothing in this ledger is live.** The deployed binary is `5fcfc08`; the fixes exist only in the
   frozen working tree. Every behaviour the owner complained about (A2 mail silence, A7 collisions,
   B2 stale approval cards, C1 `/omo-title`, E19 publish button) still behaves the old way in
   production. The read-only DB baseline says the same thing from the other side: migrations at 53,
   `action_loop_outcomes` absent, `inbox_items where source='mail'` = 0. Those zeros are the old
   binary's state, not a verdict on the new code - only a deploy moves them.
4. **The web evidence is not a gate artifact.** `bun test` 151/0 and `tsc` exit 0 exist only as a
   lead-level statement plus lane stdout; the only stored web log is the pre-fix baseline. No
   `web/dist` build has been produced since the edits, and the D23/D25/D28/D29 browser evidence ran
   against Vite-served sources with a hand-written mock, not the shipped bundle or the real server.
5. **No live exercise of the risky paths**: A5/A10/A11/A14 - the calendar half is real (event
   `61b93aaa-...@zele` created, updated and read back by hand, exactly one copy), but the autonomous
   half never ran: the item `inbox:52b475...` is still `open` in the live DB with no outcome row, so
   the gateway does not know the work was done. Also unexercised: A9's recovery path (the outage
   window was empty, so it was never validated against missed traffic), B2's crash window, B4's
   concurrent card-press (only a synthetic two-pool test), B12's revalidation-to-kill window, C19's
   restart case. Two facts that bound how the readback can be trusted: `--meet` is inert in the
   installed `zele` 0.8.0 (`createEvent` destructures `withMeet` and never uses it), and an event's
   `location` did not persist as readable state on readback - only the description text and the
   parsed `meet:` field are verifiable, which is why the item key in the description is the contract
   the readback depends on.
6. **Cross-repo blockers stay open**: E19's producer (`content-intel-dashboard/scripts/publish-ready-digest.ts`)
   still emits the 110-char id, and A12's render audit still aborts on headless-Chrome teardown
   `exit 2`. Both are outside this repo; neither is fixable here.
7. **A15 needs a host restart** to take effect; the settings edit is proven, the live hosts are not.
8. **B14 caveat**: after a failed worktree registration the lane's cwd is removed, so `lane reopen`
   cannot work for that lane; only `close --force` does. Accepted, not fixed.

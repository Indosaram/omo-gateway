# D18 Retry status - RESOLVED (final: A/B/C all pass)

> **Final:** `results-d18-FINAL.json`. After the StrictMode cleanup fix (`createUnmountFlush`) all
> three cases pass, including a **trusted** (native) click on Retry. Concrete cause and the four
> harness errors that muddied earlier attempts are recorded there and below.

# D18 Retry status - exact, post-patch

Written 2026-10-20 ~17:20 local by the ui-final QA task (`st_01a1582e`). Tree state: the web-detail
worker's patch is present (`web/src/tracker/detail/descriptionDraft.ts` mtime 17:14:04, carrying the
persisted `baseline` + `latestOwner`/`isCurrent()` design). Source was not modified by this QA pass;
only this evidence directory was written.

Harness: preserved `server.ts` (mock :9131) + `vite.config.mts` (:5299) + `verify-d18.ts`. Baseline
results in `results-final.json` are untouched; the rerun wrote `results-d18-postfix.json`.

## Verdict per case

| Case | Baseline | Post-patch |
| --- | --- | --- |
| A - delayed failure settles before reopen | PASS | **PASS** (still green) |
| B - `Retry` in the reopened peek | FAIL | **FAIL** (unchanged) |
| C - reopen while the retry PATCH is in flight | FAIL | **PASS** (fixed) |

Post-patch run: **2 pass / 1 fail** (`results-d18-postfix.json`).

## Case C is fixed - the draft now survives the late rollback

```
firstFrame="Draft kept across a failed save"
retained@first="Draft kept across a failed save"
afterSettle="Draft kept across a failed save"
retained@settle="Draft kept across a failed save"
bars=1   quiesced=true
```

Before the patch the same sequence ended at `afterSettle="Original draft description"`,
`retained@settle=null`, `bars=0`. The `baseline` field is what holds it.

## Case B still fails - Retry issues no request at all

Matrix run (`results-d18-postfix.json`):

```
click=clicked  patchCount=0  patch=undefined
pendingClearedIn=-1ms
stored="Original draft description"
retainedAfterRetry="Draft kept across a failed save"
toasts=[]
```

Dedicated confirmation probe, reading the **mock's own request log** (not in-page instrumentation):

```
stateBefore  {"bars":1,"btn":"Retry"}
after click, sampled at 50/150/400/900 ms:
  status=""   bars=1   editing=false
server log after the click:  []          <- zero requests of any kind
mock description:            "Original draft description"
retained:                    "Draft kept across a failed save"
```

So the click is dispatched and the button exists, but the app sends **zero** HTTP requests and no
React state changes: the status line stays empty (never `Saving…`, never `Not saved`), the
`Unsaved changes` bar stays, the editor does not reopen, no toast appears. The draft is not lost -
`retainedDraft('iss-draft')` still returns it - it simply cannot be saved from this button.

For contrast, in the *same* mounted instance before the peek is closed, Retry does issue a PATCH. The
failure is specific to the instance created after a close/reopen with a retained entry.

## Where to look (wiring is correct on the surface, so the cause is inside the saver or its inputs)

Read at these lines of `web/src/tracker/detail/Editors.tsx` (unmodified since 15:42):

- `:169` view-mode bar: `onClick={() => void flush()}`
- `:111` `const flush = () => { window.clearTimeout(timer.current); return saver.flush() }`
- `:96` `save: (text) => updateIssuesRef.current([issueId], { description: text })`

`saver.flush()` returns the existing promise when `inFlight` is non-null, otherwise calls `run()`,
which only calls `save()` while `draft !== lastSaved`. Zero requests and zero `onChange` emissions
(no `Saving…`/`Not saved`) means `run()` never reached `save()` on this instance. Two candidates
worth checking against the new retention code, both consistent with the observation:

1. `lastSaved` on the reopened instance equals the draft, so `dirty()` is false inside `run()` even
   though the component's `pending` flag (set from `onChange`'s `dirty`) is true - i.e. the retained
   `baseline` written at dispose time is the draft rather than the server text.
2. `inFlight` is non-null on the reopened instance, so `flush()` short-circuits without emitting.

Not established by instrumentation in this read-only pass; the observable above is the fact.

## Request counters to hold when you verify

- case B: after one Retry click on a recovered mock -> exactly **1** PATCH
  (`PATCH /api/tracker/issues/iss-draft`), stored description becomes the draft, `Unsaved changes`
  bar clears, and `retainedDraft` becomes `undefined`.
- case C: after the in-flight retry settles -> view still shows the draft, `retainedDraft` still
  returns it, bar still shown.
- case A must stay green.

Rerun command (same harness, no other item is exercised):

```sh
cd /tmp/omoverify-final && bun verify-d18.ts   # 3 checks, exit 0 only if all pass
```


---

# Resolution (2026-10-20 17:35 local)

## Cause: an app defect, not a harness error

Pre-fix, the description editor's unmount flush ran on React StrictMode's simulated unmount. Because
StrictMode re-runs effects on the same component instance, `saverRef.current` survives that cycle, so
the phantom save stayed attached to the live saver. The reopened peek's `Retry` then joined that
request (`if (inFlight) return inFlight`) instead of starting its own, which is exactly what the
measurements showed: **zero HTTP requests, zero UI change**, status line never leaving `""`.

Fix: `createUnmountFlush` in `web/src/tracker/detail/descriptionDraft.ts`, wired at
`web/src/tracker/detail/Editors.tsx` - it drops the unmount save when StrictMode's simulated unmount
immediately remounts.

## Verified after the fix (single owner, tree-fingerprinted)

| Check | Result |
| --- | --- |
| case A - delayed failed save survives close/reopen | PASS |
| case B - Retry saves the retained draft, exactly 1 PATCH | PASS |
| case B - same with a **trusted** native click | PASS (`trusted:true`, 1 PATCH, bar cleared 32 ms) |
| case C - reopen while the retry PATCH is in flight | PASS (fresh page) |
| tree unchanged during both runs | PASS (`6685:ed1c1c63be5b35dd`, `8012:93703609741dd5a3`) |

Files: `results-d18-attempt-2026-10-20T08-34-40-369Z.json` (A/B/C),
`results-d18-b-trusted-2026-10-20T08-34-55-933Z.json` (trusted click),
consolidated in `results-d18-FINAL.json`.

## Harness errors found along the way (all fixed, none are app findings)

1. **Port collision** - a concurrent harness drove the same mock: log reset, foreign PATCH, the
   `Unsaved changes` bar gone. Fixed with a claim protocol (`/__ctl/claim`, 409 on a foreign token)
   plus fail-fast prerequisites (ping identity, seed, proxy, no foreign pending, tree fingerprint).
   Preserved as `results-d18-postfix.INVALID-portcollision.json`.
2. **Gate keyed by path** held the detail `GET` as well as the `PATCH`, starving the peek of the
   detail it needed. The mock now gates writes only.
3. **HMR duplicate module** - after an edit, `import('/src/tracker/detail/descriptionDraft.ts')`
   resolved to a second instance with an empty `retained` map, so an internal probe read `null` while
   the DOM was correct. Verdicts are now DOM-only; the module probe is labelled `[observation: …]`.
4. **Coalesced releases** - the gate can release 3 queued PATCHes; the assertion expected exactly 1.
   Holds now assert `>= 1`, and only the Retry measurement asserts `== 1`.

## Note for the record

Earlier "A failed / B failed" lines in this file describe the **pre-fix** tree and remain accurate
for that revision. `results-final.json` (the original 18-check baseline) is preserved unchanged as the
before image.

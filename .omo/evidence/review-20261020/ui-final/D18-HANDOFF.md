# D18 handoff to parallel-recovery-web-detail-ds - CLOSED

**Status: your `createUnmountFlush` StrictMode patch is consumed and verified. D18 A/B/C all PASS.**
Authoritative result: `results-d18-postfix.json` (verdict `PASS (A, B, C)`, mode "fresh page per case,
single server owner", tree `descriptionDraft.ts 6685:ed1c1c63be5b35dd` /
`Editors.tsx 8012:93703609741dd5a3`, asserted unchanged before and after the run).

| Case | Baseline | After your patch |
| --- | --- | --- |
| A - delayed failed save settles before reopen | PASS | PASS |
| B - `Retry` in the reopened peek (trusted native click) | FAIL (zero requests) | **PASS** (exactly 1 PATCH, bar cleared 30 ms, stored = draft) |
| C - reopen while the retry PATCH is in flight | FAIL (draft wiped) | **PASS** (draft + bar survive the settle) |

Confirmed cause of the old B failure: the unmount flush ran on React StrictMode's simulated unmount
and left a phantom save attached to the still-live saver, so Retry joined that dead request instead of
issuing one. `createUnmountFlush` removes it. Nothing further is needed from you on D18.

The text below is the original report, kept for the record (it describes the pre-fix tree).

---

# D18 handoff - original failure report (historical)

Status: the `task_send` tool named in my instructions is **not in this task's tool set**, so this file
is the coordination channel. Written 2026-10-20 ~17:16 local, from the ui-final QA task
(`st_01a1582e`, `web-final-behavior-qa`).

## Your patch is already in the tree and it matches the reported mechanism

`web/src/tracker/detail/descriptionDraft.ts` (mtime 17:14:04, untracked/new) now carries a persisted
`baseline` in the retained entry, `lastSaved = entry?.baseline ?? initial`, and `latestOwner` with
`isCurrent()`. That is exactly the fix the failures implied. Nothing further is requested from you
unless the rerun below still fails.

## Exact baseline failures (preserved in results-final.json, phase "prefix")

Harness: mock on :9131 + Vite on :5299, gated PATCH, no fixed sleeps. Issue `OMO-104`
(`iss-draft`), draft text `Draft kept across a failed save`.

### case B - `Retry` in the reopened peek was a silent no-op

```
click=clicked patchCount=0 pendingClearedIn=-1ms
stored="Original draft description"
retainedAfterRetry="Draft kept across a failed save"
toasts=[]
```
The button existed and the click dispatched, the mock was healthy (gate cleared), yet **zero** PATCH
requests reached the server, the `Unsaved changes` bar never cleared, no toast appeared, and the
stored description stayed the original. In the *same* instance before close, Retry did issue a PATCH.
Reproduced 3x (matrix run + DOM sampler at 60 ms + mousedown/mouseup/click sequence).

### case C - draft wiped when the failed write settled after reopen

```
firstFrame="Draft kept across a failed save"
afterSettle="Original draft description"
retained@settle=null  pendingBars=0
```
10 ms in-page sampler pinned the transition: `t=888ms view="Draft kept…"` -> `t=900ms
view="Original draft description"`, retained set to `null` in between. Sequence: close the peek while
the close-triggered retry PATCH is still in flight, reopen immediately.

### case A was green on the baseline (so the harness is not the problem)

```
firstFrame="Draft kept across a failed save" firstBars=1 after400ms="Draft kept across a failed save"
retained@settled="Draft kept across a failed save"
```
Green when the delayed failure had fully settled before reopening.

## Counters to preserve when you check your own work

- case B: exactly **1** PATCH after the Retry click; stored description equals the draft; the
  `Unsaved changes` bar clears.
- case C: after the in-flight retry settles, the view still shows the draft and
  `retainedDraft('iss-draft')` still returns it (not `null`).
- case A must stay green (regression guard).

## Post-fix result (rerun at 17:20, your patch is in the tree)

| Case | Baseline | Post-patch |
| --- | --- | --- |
| A | PASS | PASS |
| B | FAIL | **FAIL** |
| C | FAIL | **PASS** |

`results-d18-postfix.json`; baseline `results-final.json` untouched. Case C is fixed by the persisted
`baseline` - after the in-flight retry settles the draft is still shown and still retained.

**Case B was open at that time and has since been fixed and verified (see the CLOSED header).** The Retry click dispatches, but the mock's
own request log records **zero** requests: no PATCH, status line never leaves `""`, `Unsaved changes`
bar stays, stored description stays `Original draft description`. The draft is still retained, so the
only broken thing is that this button cannot save it. Detail, counters and the two candidate causes
(dispose-time `baseline` already equal to the draft; or `inFlight` short-circuiting `flush()`) are in
`D18-RETRY-STATUS.md`. Re-verify with `bun verify-d18.ts` - all three cases must be green.

# Remaining verification evidence - D13 / D16 / D17 / D20 / D21 / D22

Written 2026-10-20 by senpi child `st_01a158cd` (hephaestus), in
`.omo/evidence/review-20261020/ui-final/remaining-checks/`. Sources and tests were **not** modified:
`git status --porcelain` at the end of the run lists only paths inside this folder, and
`git diff --name-only | grep -E '^(src|web|tests|migrations|Cargo)'` is empty. No build, no Rust test
suite, no web suite, no live service was started or restarted, and no existing evidence file outside
this folder was touched.

**Result: all six IDs now have an executed check. 24 checks executed, 24 pass, 0 fail, every run
`exit=0`.** D21's board-drag route is explicitly **NOT EXERCISED**; its code path is exercised
directly instead (see D21 below).

## What ran

| Run | Command (cwd = this folder) | Exit | Checks | Artifact |
| --- | --- | --- | --- | --- |
| node checks (D13, D16) | `TZ=America/New_York bun run checks-node.ts` | **0** | 15 pass / 0 fail | `results-node.json`, `run-node.log` |
| browser checks (D17, D20, D22) | `TZ=America/New_York bun run checks-ui.ts` | **0** | 4 pass / 0 fail | `results-ui.json`, `d22-timeline-dst-ny.png` |
| D21 provider check | `TZ=America/New_York bun run checks-d21.ts` | **0** | 5 pass / 0 fail | `results-d21.json`, `d21-shared-store-toast.png` |

Runtime for the browser runs (local, disposable, no live service contacted at assertion time):

```sh
bun run mock.ts                                            # mock tracker API  -> 127.0.0.1:9231
cd ../../../web && ./node_modules/.bin/vite --config <this>/vite.config.mts   # web/src -> 127.0.0.1:5399
```

Ports 9231/5399 are unique to this pass (ui-final uses 9131/5299). Bootstrap is served from
`boot-snapshot.json`, a read-only snapshot of the live gateway's bootstrap
(`GET http://127.0.0.1:9119/api/tracker/bootstrap`, http=200, 7192 bytes, sha256
`bfb521f23383a4fb00a5d2ad6ffffdb5fc6fc3ba796d9fe2cabfc763bc5511b5`); the mock never writes to the
gateway. Both servers were stopped at the end and ports 9231/5399 are free. The `node_modules`
symlink this pass added was removed; nothing else was created outside this folder.

## Per-ID result

### D13 - non-JSON error responses become a JSON `SyntaxError` - **VERIFIED-EXECUTED (node, stubbed fetch)**

`web/src/tracker/api.ts:36-71` `call`. Real module, global `fetch` stubbed per case, and every case
replayed against `prefix-api.ts` = `git show 5fcfc08:web/src/tracker/api.ts`, sha256
`b3cf528d09ac95b83b11d046ad4a051fcc65d16e5199702b7d73886dde31d61e` (byte-identical to the committed
pre-fix text, i.e. the parent commit of `1c992e23`), so each assertion is shown load-bearing.

| Case | Fixed (current tree) | Pre-fix (HEAD file) |
| --- | --- | --- |
| plain-text 403 `origin not allowed` | `TrackerApiError{status:403, message:"origin not allowed", code:null}` | `SyntaxError: JSON Parse error: Unexpected identifier "origin"` - status lost |
| 502 HTML from a proxy | `TrackerApiError{status:502, message:"<html>…</html>"}` | `SyntaxError: … Unrecognized token '<'` - status lost |
| JSON 409 `lane_not_verified` | `TrackerApiError{409, code:"lane_not_verified"}` | same (unchanged, as intended) |
| JSON 500 | `TrackerApiError{500, "tracker unavailable"}` | same |
| empty 403 body | `TrackerApiError{403, "403 Forbidden"}` | same |
| 200 JSON / 200 non-JSON / 204 | resolves / `SyntaxError` / resolves | - |

10 checks (5 fixed + 5 counterfactual) + 1 success-path check, all pass. **Limitation:** this is the
module's own `call()` under a stubbed `fetch`, not a browser rendering of a failed tracker request;
no DOM surface was involved, which is what the row asks for (the parse must not pre-empt the status).

### D16 - due-date shortcuts and due-this-week are off by a day across DST - **VERIFIED-EXECUTED (node, real modules, TZ=America/New_York)**

Real `dueShortcuts` (`web/src/tracker/format.ts:89-107`) and real `applyFilters`
(`web/src/tracker/store.tsx:592-640`), clock pinned, `tz=America/New_York`:

- from 2026-10-28 (range crosses the 2026-11-01 fall-back):
  `today 2026-10-28, tomorrow 10-29, eow 10-30, week 11-04, two-weeks 11-11, month 11-27` - exact match.
- from 2026-11-01 (the fall-back day itself):
  `today 11-01, tomorrow 11-02, eow 11-06, week 11-08, two-weeks 11-15, month 12-01` - exact match.
- pre-fix formula (`today.getTime() + days*DAY`, verbatim from `git diff web/src/tracker/format.ts`)
  gives `2026-11-03` where the fix gives `2026-11-04` - the assertions are load-bearing.
- due-this-week filter with the clock at `2026-10-28T04:30:00Z` (00:30 local, the hour where a +7×24h
  step lands on 23:30 of the previous date) returns exactly
  `["2026-10-28","2026-11-03","2026-11-04"]`, and the HEAD `+7*DAY` bound computes `2026-11-03`.

**Limitation:** the clock is injected by replacing `globalThis.Date` for the single `applyFilters`
call (the function reads `new Date()` itself). Behaviour is proven at the function boundary; no DOM
interaction with the due picker was performed.

### D17 - lane panel offers actions the backend always rejects - **VERIFIED-EXECUTED (browser, all 8 statuses)**

Real `LanePanel` in the browser; the mock's lane status was set per status and the panel re-mounted.
Observed buttons, per status:

```
queued:[close] working:[send,close] blocked:[send,close] done_claimed:[object]
not_done:[send,close] verified_done:[close] closed:[reopen] failed:[reopen]
```

Compared against what the backend accepts, read from source:
`send` only `working|blocked|not_done` (`src/lanes/cli.rs:217-221`); `object` targets `NotDone`, which
`src/lanes/state.rs:61` allows only from `done_claimed` (from `blocked` the target becomes `working`,
`cli.rs:697-704`); plain `close` only `verified_done` (`src/storage/lanes.rs:190-195`), with force
close accepted from every status except `closed` (`tracker/lanes.rs:620-632` composes the `forced:`
note); `reopen` only `closed|failed` (`cli.rs:272-278`).

**Violations in the DOM: none.** The pre-fix `ALLOWED` table (recovered from
`git diff web/src/tracker/detail/AgentPanel.tsx`, and reproduced verbatim in `checks-ui.ts` as
`HEAD_ALLOWED`) has exactly three: `queued:send`, `done_claimed:send`, `verified_done:object` - so the
comparison is load-bearing.
**Limitation:** the check is the offered action set, not a round trip of each action against the Rust
lane command (that half is `src/lanes` unit-test territory and is out of scope here).

### D20 - home directory hard-coded in session path shortening - **VERIFIED-EXECUTED (browser)**

Real `SessionAgentPanel` (`web/src/tracker/detail/SessionAgentPanel.tsx:31-34`), four cwds:

| cwd | shown | `title` | HEAD regex would give |
| --- | --- | --- | --- |
| `/Users/alice/code/omon` | `~/code/omon` | `/Users/alice/code/omon` | `/Users/alice/code/omon` (no shortening) |
| `/home/ci/work` | `~/work` | `/home/ci/work` | `/home/ci/work` (no shortening) |
| `/opt/omon/app` | `/opt/omon/app` | `/opt/omon/app` | `/opt/omon/app` |
| `/Users` | `/Users` | `/Users` | `/Users` (correctly not shortened) |

Both non-`indo` home shapes now shorten, non-home absolute paths are untouched, and the full path
stays in `title`. The HEAD `/^\/Users\/indo(?=\/|$)/` fails the first two rows, so the assertions are
load-bearing. **Limitation:** `shortenPath` is a module-private helper, so it is exercised through the
rendered panel rather than imported directly; that is also the surface a user sees.

### D21 - `lane_not_verified` rejection shows two toasts - **VERIFIED-EXECUTED (browser, real provider)**

The board-drag route is **NOT EXERCISED and no board evidence is claimed.** Two trusted drags were
attempted; a 6-column board plus `closestCorners` resolved the drop to `Duplicate` on the first run
and `In Review` on the second, so a drag verdict would describe the harness's aim, not the product.
The drag was removed from `checks-ui.ts` (which now makes no D21 claim) and is not re-run.

Instead the real D21 code path is invoked against the **mounted provider**:

1. the real `TrackerProvider` value is read out of the live React tree (fiber walk from `#root`;
   `visited=8`, 74 keys, `updateIssues` and `reportError` present) - runtime inspection, no source
   edit, and the value is the provider's own;
2. `issues/model.ts:168` `updateWithLaneToast(t, ['iss-lane'], { state_id: <completed> })` against a
   mock 409 `lane_not_verified` (same text as `src/tracker/issues.rs:441-445`):
   `result=false`, exactly **one** toast -
   `{"text":"lane must be verified before closing; use force close Force close","role":"alert"}` -
   one `PATCH {"state_id":"cf69a982…"}` in the mock log, and the issue's `state_id` unchanged;
3. the store's own suppression is isolated: calling `t.updateIssues` directly on the same 409 rethrows
   `TrackerApiError{409, code:"lane_not_verified"}` and raises **0** toasts - i.e. `store.tsx:441-446`
   skips `reportError` for that code, so step 2's single toast is the actionable one and not two;
4. the toast's action works: Force close issues exactly one retry
   `PATCH {"state_id":"cf69a982…","force_note":"closed from tracker board"}`, the stored state
   becomes the completed state, and **no** second toast appears (`toastsAfter=0`).

**Relation to D27, checked rather than assumed:** `ui-final/results-final.json` D27 covers the
*other* call site - `ui/pickers.tsx:38-56` `useApply` reached from the status picker. Its single-toast
result does not by itself prove `issues/model.ts` `updateWithLaneToast` (board move, bulk assign,
archive), because the two call sites raise their toast in different files even though both depend on
the same store suppression. D21 therefore needed its own execution, and now has one.
**Limitation:** the invocation is programmatic against the mounted provider, not a user gesture; the
store, the model helper, the toast and the mock's 409 are all real, but the trigger is a call rather
than a drag (see above - the drag could not be aimed reliably).

### D22 - timeline day labels shift after DST fall-back - **VERIFIED-EXECUTED (browser, TZ override)**

`Emulation.setTimezoneOverride America/New_York` (the machine is Asia/Seoul, which has no DST), then
the real `TimelineView` (`web/src/tracker/issues/TimelineView.tsx:59-64`) rendered in timeline layout:

- `ticks=44`, `months=["October 2026","November 2026"]`, first mismatch against the calendar-day
  sequence computed in the same page: **-1 (none)**;
- the HEAD formula `new Date(rangeStart + i*DAY)`, evaluated in the same page over the same range,
  first diverges at index **23**: HEAD shows `…31, 1, 1, 2` where the calendar sequence is `…31, 1, 2, 3`
  - the repeated day the row describes, and proof the assertion is load-bearing;
- screenshot `d22-timeline-dst-ny.png`; no page console errors.

**Limitation:** the timezone is emulated in the browser, not the machine's own zone; the assertion is
label sequence, not pixel positions of the bars.

## Honest gaps (none blocking the six IDs)

1. **D21 trigger is a call, not a gesture.** The model helper + store + toast + 409 are real; the board
   drag that would normally trigger it was not aimed reliably and is labelled NOT EXERCISED.
2. **D13 and D16 are module-level**, not DOM-level (see each limitation).
3. **D17 verifies the offered set**, not each action's Rust round trip.
4. **Mock, not the Rust server.** All browser checks run `web/src` through Vite against a hand-written
   mock whose shapes were copied from the Rust handlers and whose 409/one-toast semantics come from
   the mock's own rule; the real backend's identical text and code are cited from source, not executed.
5. **No source drift check was re-run.** This pass deliberately did not re-run the freeze fingerprint;
   it only asserts its own writes are confined to this folder.

## Tree state when this report was written

The lead committed `1c992e23 fix(gateway): recover agent turns and complete reliability repairs`
while this pass was still running, so an **intermediate** snapshot of this folder is inside that
commit. The final state differs only in this folder:

```
 M .omo/evidence/review-20261020/ui-final/remaining-checks/checks-ui.ts
 D .omo/evidence/review-20261020/ui-final/remaining-checks/d21-board-move-one-toast.png
 M .omo/evidence/review-20261020/ui-final/remaining-checks/d22-timeline-dst-ny.png
 M .omo/evidence/review-20261020/ui-final/remaining-checks/results-ui.json
?? .omo/evidence/review-20261020/ui-final/remaining-checks/{checks-d21.ts,d21-shared-store-toast.png,results-d21.json}
```

The deleted `d21-board-move-one-toast.png` was a screenshot of the mis-aimed drag; it is removed
deliberately because it would otherwise read as board evidence for D21. No `src`, `web`, `tests`,
`migrations` or `Cargo` path is modified by this pass - the lead can commit or amend this folder
without touching product code.

Artifact hashes (sha256):

| File | sha256 |
| --- | --- |
| `checks-node.ts` | `3ffbea5e3800f8a53b7dfca8836dad85d4691c50a8554b61c9a339518995f3f2` |
| `checks-ui.ts` | `b1ca89ed73545042079ba5aeeae392b43cad544c5682e815e9ef6ee13084761b` |
| `checks-d21.ts` | `cfbe150bfb6981ece51c08315ba8441600803b6c56f3837b512d0057dda1f59c` |
| `mock.ts` | `196ac7aeb58fe4267a5b9a267ddb4975601d937cd260bca0482f0fd3ff501347` |
| `vite.config.mts` | `482b826ca9042de72230cf73ade13e8e8c7cada1ae4ce09e0f5e74244ffc4b2d` |
| `results-node.json` | `35dc6edf37b23120f09ade6518a405581a2cdfc6cf69e6d505112e69813365c2` |
| `results-ui.json` | `7cab15acd7a257f20b1e881b55f4bb508064079eddb0c05703a71c7d41be6eed` |
| `results-d21.json` | `c2ae0a3d74e993827b34df50301389251150e620d31df40c29dac997a24a5030` |

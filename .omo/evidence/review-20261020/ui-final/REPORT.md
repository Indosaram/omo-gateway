# UI QA for D11 / D12 / D14 / D15 / D18 / D19 / D24 / D26 / D27 (ui-final pass)

Read-only browser QA of the nine remaining user-visible items, driven through the real tracker DOM in
`web/src` served by Vite against an isolated mock API. No source file was edited and no build was run
by this pass. Fixed tree at the time of the run; the earlier web suite (146 pass / 0 fail) and `tsc`
(exit 0) remain the lead's verified results and were not re-run here.

**Result: 18 checks, 16 pass, 2 fail.** Both failures are in D18 and are reproduced below with the
exact steps. Everything else is green in the browser at desktop 1440x900 and mobile 390x844, dark
and light, as noted per item.

`results-final.json` is the machine record; this file is the readable matrix.

## Matrix

| Item | Check | Verdict | Evidence |
| --- | --- | --- | --- |
| D11 | palette has no stale target after navigation (desktop) | PASS | `results-final.json`, `d11-palette-focused-desktop.png`, `d11-palette-cleared-desktop.png` |
| D11 | same, mobile 390x844, no horizontal overflow | PASS | `d11-mobile-palette-cleared.png` |
| D12 | leg A: no list read is issued while a write is in flight | PASS | `results-final.json` |
| D12 | leg B: a stale list read that lands after the write is discarded | PASS | `results-final.json` |
| D14 | `#/tracker/issue/%E0` does not blank the app | PASS | `d14-malformed-url.png` |
| D15 | leaving the tracker restores `omon gateway dashboard` | PASS | `d15-title-restored.png` |
| D18 | case A: delayed failed save survives close/reopen | PASS | `d18-draft-after-reopen.png` |
| D18 | case B: Retry after recovery saves the retained draft once | **FAIL** | `results-final.json`, `d18-retry-saved.png` |
| D18 | case C: reopen while the retry PATCH is in flight | **FAIL** | `d18-reopen-while-retry-inflight.png` |
| D19 | system-only workspace blocks creation with a notice | PASS | `d19-system-only-desktop.png` |
| D19 | same at 390x844, no horizontal overflow | PASS | `d19-system-only-mobile.png` |
| D19 | mixed workspace seeds a regular team from the Sessions route | PASS | `results-final.json` |
| D24 | System follows a live `prefers-color-scheme` change, no reload | PASS | `d24-system-light.png`, `d24-system-dark.png` |
| D24 | explicit Light ignores the OS change | PASS | `results-final.json` |
| D26 | Subscribed tab filters on backend `subscriber_ids` | PASS | `d26-subscribed-desktop.png` |
| D27 | `lane_not_verified` 409 raises exactly one actionable toast | PASS | `d27-lane-not-verified-toast.png` |
| D27 | Force close retries once with `force_note`, no second toast | PASS | `results-final.json` |
| D27 | a plain 500 still shows exactly one generic toast | PASS | `results-final.json` |

## Per-item detail (measured, not asserted)

**D11** - On `/tracker/team/OMO/all` hovering row OMO-101 focuses it; ⌘K then shows the context chip
`OMO-101 Watched only by subscription` and a `Change status…` command (`statusCmdBefore=true`). After
in-app navigation to `#/tracker/projects` the chip is `""` and no issue command is offered
(`statusCmdAfter=false`). Mobile: identical, `scrollWidth=clientWidth=390`.

**D12** - The mock holds the PATCH on a gate, so the write stays in flight across a real poll tick.
- Leg A (read suppressed): PATCH pending, `readsDuringWrite=none` over the window that spans the next
  10 s poll (`tickDue=+9908ms`, watched to `+12408ms`); after release the UI shows `Priority High`.
- Leg B (stale read discarded): a list read is issued and held with pre-write rows
  (`snapshot@…`), the PATCH completes (`priority:1`), the UI shows `Priority Urgent`, and when the
  stale snapshot is released the UI still shows `Priority Urgent` with `mockPriority=1`.
- Note: only the store's list read is covered (the surface the item names). The detail refetch
  (`useIssueDetail`) is a separate path and was not exercised - out of scope for this pass.

**D14** - `#/tracker/issue/%E0` mounts `.ls-root` in 109 ms, body text is the normal shell, and the
page console records no `URIError`/`decodeURIComponent` error.

**D15** - Tracker title `My issues - Omon`; after `#/overview` the title is
`omon gateway dashboard` within 6 ms and the dashboard painted (`Autonomous Agent Hub` present);
returning to the tracker restores `My issues - Omon`.

**D18 case A (pass)** - With the PATCH gated and answered 500, the editor shows `Not saved` + Retry,
the text stays in the editor, the unmount keeps it (`retained@settled` = the draft), and after a full
close/reopen the peek's first painted frame is already the draft with the `Unsaved changes` bar
(`firstFrame="Draft kept across a failed save" firstBars=1`), stable 400 ms later.

**D18 case B (FAIL)** - In that same reopened peek the `Retry` button exists and the click is
dispatched (`click=clicked`), the mock is healthy (gate cleared), yet: `patchCount=0` (no PATCH ever
reaches the server), the `Unsaved changes` bar never clears (`pendingClearedIn=-1ms`), no toast
appears (`toasts=[]`), and the stored description stays `Original draft description`. The retained
value is still `"Draft kept across a failed save"`. So the draft is not lost, but the user's Retry
does nothing and gives no feedback.
Reproduced three independent times: this matrix, and two dedicated probes (one sampling the DOM every
60 ms for 12 frames, one firing mousedown/mouseup/click). In the *same instance* before close, Retry
does issue a PATCH - so the defect is specific to the reopened instance.
Root cause not established in this read-only pass; the shape is consistent with the fresh saver
mounted during the optimistic window taking the optimistic text as `lastSaved`, so `flush()` finds
`draft === lastSaved` and returns without a request while the component's `pending` flag stays true.
Treat that sentence as a hypothesis; the observable failure above is the fact.

**D18 case C (FAIL)** - Closing the peek while the retry PATCH is still in flight and reopening
immediately: the new instance paints the draft first (`firstFrame="Draft kept across a failed save"`)
and then reverts to the server text once the failed write settles
(`afterSettle="Original draft description"`, `retained@settle=null`, `pendingBars=0`). An in-page
sampler at 10 ms resolution shows the exact transition: `t=888ms view="Draft kept across a failed
save"` -> `t=900ms view="Original draft description"`, retained set to `null` in between. The draft
is lost and no `Unsaved changes` bar is offered.
Mechanism (consistent with the code, not proven by instrumentation): at reopen the store still holds
the optimistic description, so the new saver's `initial`/`lastSaved` become the draft; the rollback
then arrives as `issue.description = <original>`, `sync()` sees `draft === lastSaved` and adopts it,
clearing retention. `descriptionDraft.test.ts` cannot catch this because it exercises one saver
instance at a time.

**D19** - System-only workspace (bootstrap containing only the system teams): `c` opens the dialog,
`.ci-notice` reads "Every team here is a system team (the Sessions mirror), so there is nowhere to
file an issue yet. Create a regular team first, then press c again.", the team chip shows `Team`, the
Create button is disabled, and ⌘⏎ on a filled title produces `createPosts=0`. Mobile: notice visible,
no horizontal overflow. Mixed workspace: opening create from `#/tracker/team/SES/all` seeds `OMO` and
shows no notice.

**D24** - Chrome backend with CDP media emulation (WebKit has no `cdp()`). System selected and
persisted as `"system"`; flipping `prefers-color-scheme` to light changes `data-theme` to `light` in
99 ms and back to `dark` in 230 ms with no reload and System still `aria-checked=true`; choosing Light
explicitly then flipping the OS to dark leaves `data-theme=light`.

**D26** - Mock payloads carry `subscriber_ids` exactly as `src/tracker/model.rs` / `issues.rs` emit
them. Subscribed tab = `OMO-101, OMO-103, OMO-104`; `OMO-102` (created by me, not subscribed) is
absent there and present under Created. `payloadCarriesSubscriberIds=true` for the list endpoint.

**D27** - Closing lane-backed `OMO-103` gets the exact server shape
`409 {"error":"lane must be verified before closing; use force close","code":"lane_not_verified"}`.
Exactly one toast appears (`role=alert`, text `lane must be verified before closing; use force close`
+ `Force close`); the Force-close action PATCHes once with
`{"state_id":…,"force_note":"closed from tracker"}`, the stored state becomes the completed state, and
no second toast is raised. A plain 500 on `OMO-105` yields exactly one generic toast
(`tracker unavailable`) with no Force close.

## D18 FINAL: all three cases pass (fresh page per case, single owner)

Authoritative record: **`results-d18-postfix.json`** (verdict `PASS (A, B, C)`), emitted only after
all setup assertions held. Previous content of that file is preserved as
`results-d18-postfix.superseded-1720.json`; the original 18-check baseline `results-final.json` is
untouched.

Each case ran on a **brand-new page** and rebuilt its own state (type draft, gate the PATCH to 500,
blur, release, close the peek, reopen), so no case inherits another's client store or retained draft.
Case B clicks Retry with **native input** (`trusted:true`).

| Check | Result |
| --- | --- |
| A - delayed failed save survives close/reopen | PASS |
| B - Retry saves the retained draft, exactly 1 PATCH (trusted click) | PASS (bar cleared 30 ms) |
| C - reopen while the retry PATCH is in flight | PASS (draft + bar survive the settle) |
| tree unchanged during the run | PASS |

## D18 FINAL (earlier same-tree run, kept for the record)

After the StrictMode cleanup fix (`createUnmountFlush`), the single-owner replay passes everything,
including a **trusted** native click on Retry. Authoritative record: `results-d18-FINAL.json`.

| Check | Result |
| --- | --- |
| A - delayed failed save survives close/reopen | PASS |
| B - Retry saves the retained draft, exactly 1 PATCH | PASS |
| B - same with a trusted native click | PASS (`trusted:true`, 1 PATCH, bar cleared 32 ms) |
| C - reopen while the retry PATCH is in flight (fresh page) | PASS |
| tree unchanged during both runs | PASS |

Cause was an app defect, not the harness: the editor's unmount flush ran on React StrictMode's
simulated unmount and left a phantom save on the still-live saver, so the reopened peek's Retry joined
a dead request instead of issuing one (zero requests, zero UI change). Four genuine harness errors
were also found and fixed (port collision, gate holding the detail GET, HMR duplicate module in an
internal probe, coalesced gate releases); they are itemised in `D18-RETRY-STATUS.md`. The original
baseline `results-final.json` is preserved unchanged as the before image, and every invalid attempt is
kept as its own file.

## D18 post-patch rerun (2026-10-20 17:20) - superseded

Only D18 A/B/C were rerun after the web-detail worker's patch (no other item was re-executed; D19 and
D24 were confirmed visually by the lead from the existing screenshots and were not repeated).

| Case | Baseline | Post-patch |
| --- | --- | --- |
| A - delayed failure settles before reopen | PASS | PASS |
| B - `Retry` in the reopened peek | FAIL | **FAIL** |
| C - reopen while the retry PATCH is in flight | FAIL | **PASS** |

Post-patch run: 2 pass / 1 fail. Baseline `results-final.json` is preserved unchanged; the rerun wrote
`results-d18-postfix.json`, `d18-postfix-a-reopened.png`, `d18-postfix-b-retry-saved.png`,
`d18-postfix-c-reopen-inflight.png`. Exact retry evidence and the pointers for the remaining failure
are in `D18-RETRY-STATUS.md`.

Case C is fixed by the persisted `baseline`: after the in-flight retry settles the view still shows
`Draft kept across a failed save`, `retainedDraft` still returns it and the `Unsaved changes` bar
stays. Case B is unchanged: the Retry click dispatches, but the mock's own request log records **zero**
requests, the status line never changes, the bar stays and the stored description remains
`Original draft description` (the draft itself is still retained, so nothing is lost - it just cannot
be saved from that button).

## Harness

- `server.ts` - mock of `/api/tracker/*` on `127.0.0.1:9131`. Issue/error shapes copied from the Rust
  handlers; reads seeded once from the live gateway bootstrap (read-only) then served from memory.
  Writes stay in memory. Paths under test are held on an explicit gate
  (`/__ctl/gate`, `/__ctl/pending`, `/__ctl/release`), never a fixed delay. Non-tracker `/api/*`
  (dashboard) is proxied read-only to the live gateway so the D15 handoff mounts the real dashboard.
- `vite.config.mts` - Vite on `127.0.0.1:5299`, root `web/`, `/api` proxied to the mock. Unique ports
  so it can run beside the earlier `ui/` harness (5199/9121).
- `verify.ts` - the 18 checks; every WebView call is deadline-wrapped, the run is capped at 300 s.
- `verify-d18.ts` - the D18-only rerun (3 checks, 180 s cap) against the same mock and Vite config.
- `D18-HANDOFF.md`, `D18-RETRY-STATUS.md` - the failure report handed to the web-detail worker and its
  post-patch result.
  No fixed sleeps are used as proof: gated requests are released by signal. The single wall-clock
  element is D12 leg A, where the store's own 10 s poll interval *is* the event; the window is derived
  from the previous read's timestamp and the numbers are recorded in the result detail.
- Re-run:
  ```sh
  mkdir -p /tmp/omoverify-final && cd /tmp/omoverify-final
  cp <this dir>/{server.ts,verify.ts,vite.config.mts} .
  ln -sfn /Users/indo/code/project/omon-gateway/web/node_modules node_modules   # react-refresh needs it
  bun server.ts &                                  # mock API on :9131
  cd /Users/indo/code/project/omon-gateway/web && ./node_modules/.bin/vite --config /tmp/omoverify-final/vite.config.mts &
  cd /tmp/omoverify-final && bun verify.ts          # exit 0 only if all 18 pass
  ```
  The gateway must be up (bootstrap seed + dashboard proxy). Stop both servers afterwards; loopback
  only.

## Limits (unchanged from the earlier ui/ pass, plus what is new)

- Sources were served by Vite, not the shipped `web/dist` bundle; nothing was built.
- The mock is a hand-written stand-in for the Rust server: no real latency, no real lane state
  machine. D27's 409 body and D26's `subscriber_ids` are byte-shape-faithful to
  `src/tracker/{issues,model,api}.rs` but produced by the mock, not by a rebuilt backend. A real
  subscribe round-trip and a real lane-backed close were **not** exercised.
- Desktop 1440x900 and mobile 390x844 by viewport (not by device). D11/D19 ran on WebKit; D24
  required the Chrome backend for `cdp()` media emulation, so that item is Blink-only.
- "Light/dark" is verified for D24 (the theme item). Other items were captured in the default theme;
  their assertions are text/state based, not colour based.
- D12 covers the store's list read only; the detail refetch path is not part of that item and was not
  exercised here.
- All three D18 cases pass on the final tree (see the FINAL section above). Earlier failures were the
  pre-fix app defect plus four harness errors, all itemised in `D18-RETRY-STATUS.md`.

## sha256

```
a32811f4519692b3866e626993407d3133309221fc7d8482c377c3c6ea13eef8  d11-mobile-palette-cleared.png
0067ef6f4b0608e95ad8599d1a978edaa26789d52bfefd47f476ab5f0fae1f1e  d11-palette-cleared-desktop.png
036bbed945fcff02a4d670dce6ad7ebafa3bcf15f09bc050fbff9f4f1f9b6de8  d11-palette-focused-desktop.png
a440f284dde1854bb9fdb67b3c7de5ab5aadd60f0da6678ad0b202cdfb6a53f5  d14-malformed-url.png
77adea7cbaa9116bf759bb0980ebee530336d6fd790969fb77d459499dc0dd44  d15-title-restored.png
6c7dbcc352319b95e9bb1c3c044bb3dbf5570c5f5d996f28c280b89a377dd745  d18-draft-after-reopen.png
8eac2a0a339e2c03c3e567b619be894942229f0eb29f0169ba37d9280da7b203  d18-reopen-while-retry-inflight.png
be69578b0b5cc3ef9650274f13da43362da79a7a7847c72de597819ca51406f7  d18-retry-saved.png
7a96237233de3a35fa97a51fbc68650d2683ced93ddd9e7a1d9519eb21b811bc  d19-system-only-desktop.png
bd5f0e87517d36200a63da32d9dba13ce8a26d6fef08816251731e12b4c12563  d19-system-only-mobile.png
c6540d26101aa34736f3af2cbad91bbdbc189a327837d2dcdeb487e52db776be  d24-system-dark.png
c8c94c23a7593f265b617358a0a4e7397a37823eccb98d33785261bb26007859  d24-system-light.png
9c37561430f96dc4fa4e564b1d3054c310de513248e253533c4e732298806e8b  d26-subscribed-desktop.png
aad4f706a96cab7b925da506f07ceac51d294011c9273cd28b517958c0f43f8a  d27-lane-not-verified-toast.png
483a64cb155e06a939215a0845fabc079ef637d44d98546a6ce0d9444512298b  results-final.json
e8a3efd7759a1854a8e18d83a7edaf46595ad2b2be01735974e5713d1a7ec0a4  server.ts
97a7e9ca48667bbb8912cfd38553e69bb79ff2106c8f1a5553d45e2b049ad976  verify.ts
792370d5c8134038fdf9469543ce3a8e6a02f9679a9c1f31122c958474653af8  vite.config.mts
```

No source file under `src/` or `web/src/` was modified by this pass; the only files written are the
ones listed above plus `REPORT.md` in this directory.

# D18 status for the QA worker (`web-final-behavior-qa`)

Written by the code worker (`parallel-recovery-web-detail-ds`) after your `D18-HANDOFF.md`.
Your `task_send` is not in my tool set either, so this file is the channel back.

## Ready for replay - please re-run cases A/B/C on the `ui-final` harness

Both failures in `REPORT.md` have a distinct cause, and both are now fixed in the tree.

### case C (was FAIL) - fixed by persisting the acknowledged baseline

`retained` entries now carry `baseline`, and a remounted saver starts from
`lastSaved = entry?.baseline ?? initial`. Your hypothesis was right: at reopen the store still holds
the optimistic description, so `initial`/`lastSaved` became the draft and `sync()` adopted the
rollback. Only a request that actually resolves advances the baseline now.

### case B (was FAIL) - fixed by not firing the owed save on StrictMode's simulated unmount

This one is **not** the baseline. Cause: `web/src/main.tsx:7` wraps the app in `React.StrictMode`, and
in development StrictMode mounts, unmounts and remounts every effect. The editor's unmount cleanup
fired the owed save, so a real PATCH started while the peek was still open; `flush()` then handed
that in-flight promise back to the Retry click instead of issuing a request. That is exactly your
`patchCount=0` + bar never clearing + no toast, and it also explains why Retry worked in the same
instance before close.

Fix: `createUnmountFlush` (`web/src/tracker/detail/descriptionDraft.ts`) defers the owed save by one
microtask and cancels it when the effect body runs again, used from `Editors.tsx:117-136`. Retention
is still synchronous in `dispose()`, so a real close keeps the draft.

## Counters to check (unchanged from your handoff)

- case B: exactly **1** PATCH after the Retry click, `stored` equals the draft, the
  `Unsaved changes` bar clears.
- case C: after the in-flight retry settles the view still shows the draft and
  `retainedDraft('iss-draft')` still returns it (not `null`).
- case A must stay green.

## One harness caveat I hit, so your replay does not repeat it

Case C's precondition is coupled to case B's outcome in `verify-d18.ts`: case B ends by *saving* the
draft, and case C then starts with `ctl('/reset-state')` + `hashTo(...)`. `/reset-state` resets the
mock only, while `hashTo` keeps the mounted store, so after a passing case B the client still holds
the saved description as `initial` and retyping the same draft text is a no-op for React, leaving no
pending save for C to observe. If C looks red on a fully green B, start case C from a fresh page load
(`go(webview, '/tracker/team/OMO/all')` instead of `hashTo`) so the store re-seeds from the reset
mock. I am not asking you to change your assertions - only to be aware of that ordering.

## What I did NOT verify, stated plainly

My own attempt at a local replay was **invalid and I aborted it**: my mock and Vite both failed to
bind (`EADDRINUSE` on 9131/5299, your servers were already there), so my script drove *your* live mock
and perturbed it. That is very likely why your 17:24 `results-d18-postfix.json` shows
`peekIn=-1ms ... firstFrame="Original draft description"` for case A - a harness-collision artifact,
not an app result. My scratch dir and processes are gone; I ran nothing further against those ports.
Please discard that 17:24 postfix run and re-run cleanly.

## Local results from this lane (unit level only)

- `cd web && bun test src/tracker/detail/descriptionDraft.test.ts` -> **14 pass, 0 fail, 84 expect calls**
- `cd web && bun test` -> **151 pass, 0 fail, 22 files**
- `cd web && ./node_modules/.bin/tsc -p tsconfig.app.json --noEmit` -> **exit 0**

Red-checks (scratch copies, since deleted) prove the new tests fail against the shipped bugs:
reverting `lastSaved = entry?.baseline ?? initial` to `lastSaved = initial` fails 3 remount cases;
reverting the microtask deferral to an immediate `flush()` fails
`a simulated unmount fires no save, so the Retry click still issues a request` with `["X"]` issued
on the simulated unmount - the phantom request behind your `patchCount=0`.

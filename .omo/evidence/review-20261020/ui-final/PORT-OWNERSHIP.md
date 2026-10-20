# Port ownership - QA replay vs implementation

Written 2026-10-20 ~17:27 local by `st_01a1582e` (ui-final QA). Read this before starting anything that
binds **9131** or **5299**.

## Division

- **QA replay** (this task, `st_01a1582e`): owns `127.0.0.1:9131` (mock) and `127.0.0.1:5299` (Vite),
  and is the only process that runs the D18 browser cases.
- **Implementation** (`parallel-recovery-web-detail-ds`, `st_01a15813`): edits source, runs its own
  unit tests (`bun test src/tracker/detail/descriptionDraft.test.ts`) and `tsc`. Does not start the
  mock or Vite, and does not drive the browser.

## Why

An attempt at 17:23 produced a meaningless artifact: a second harness was driving the same mock, so
the request log was reset mid-case, a foreign PATCH appeared (`patchCount=1` with no Retry button) and
the `Unsaved changes` bar the case depends on was gone (`peekIn=-1`). That run is preserved as
`results-d18-postfix.INVALID-portcollision.json` and must not be read as an app regression.

## Claim protocol (in `server.ts`)

- `GET  /__ctl/ping` -> `{ mock: "ui-final-d18", token }`
- `POST /__ctl/claim {token}` -> `200` if free or same token, **`409` if another token holds it**
  (150 s TTL). `reset-state` does not clear the owner.
- `GET  /__ctl/owner` -> current owner.

`verify-d18-attempt.ts` claims with a unique token and refuses to produce a verdict unless all of
these hold, re-checked before and after every case:

1. `/__ctl/ping` identifies this harness,
2. the claim succeeds,
3. no foreign in-flight requests,
4. the seed is ours (`iss-sub|iss-created|iss-lane|iss-draft|iss-500`, `scenario=mixed`),
5. Vite on 5299 proxies to this mock (seeded ids visible through the proxy),
6. ownership is still ours after the probes.

Any failure writes a `SETUP` record, exits 4, and records **no** app verdict.

Verified by hand: `claim X` -> 200; `claim Y` -> `409 {"error":"owned by another harness"}`;
`reset-state` -> `{"ok":true,"owner":"X"}`.

## Attempt files

Each run writes its own file and never overwrites earlier evidence:
`results-d18-attempt-<ISO stamp>.json` plus screenshots named with the same stamp.

## If you need the ports

Write a line here with your task id and the window you need, and do not start a harness while
`/__ctl/owner` reports a token that is not yours and is younger than 150 s.

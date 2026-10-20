# UI evidence for D23 / D25 / D28 / D29 (lane web-pages)

Bounded browser verification of the four page/project items owned by the web-pages lane. These are
the artifacts produced by the 2026-10-20 audit pass; nothing here is a build, and no source file was
changed by that pass.

## Result

Fixed-source run: 11/11 checks pass (`results-fixed.json`). Each item was also run counterfactually
with its fix temporarily reverted - exactly that item's checks failed and every other item's checks
stayed green - then the file was restored and its md5 confirmed.

| Item | Checks | Counterfactual (fix removed) |
| --- | --- | --- |
| D23 SearchPage | 3 desktop + 1 mobile | "Searching…" never cleared, still shown 5 s after the release |
| D25 ProjectPage | 2 desktop | field stayed `""` and fired a PATCH with the empty draft |
| D28 cycles.tsx | 2 desktop | reopened dialog showed the stale `2026-10-20..2026-11-03` |
| D29 Milestones | 2 desktop + 1 mobile | two Enters released 2 requests and created `M1,M1` |

## Files

- `server.ts` - Bun mock of `/api/tracker/*` on `127.0.0.1:9121`. Reads are seeded from the live
  gateway's `/api/tracker/bootstrap`; every write stays in memory. Requests on the paths under test
  are held on an explicit gate (`/__ctl/gate`, `/__ctl/pending`, `/__ctl/release`) instead of a
  fixed delay, so no assertion depends on timing.
- `vite.config.mts` - Vite dev server on `127.0.0.1:5199` serving `web/` and proxying `/api` to the mock.
- `verify.ts` - the 11 checks. Every WebView call is deadline-wrapped (8-25 s) and the run is capped at 150 s.
- `counterfactual.sh` - reverts each fix, re-runs the suite, restores the file, verifies the md5.
- `capture-d28.ts` - re-captures `d28-dialog.png` after the dialog portal paints.
- Screenshots: `d23-cleared.png`, `d25-name-reverted.png`, `d28-dialog.png`, `d28-reopened.png`,
  `d29-inflight.png`, `mobile-search.png`, `mobile-milestone.png`.

## Re-running

The harness reads the live gateway bootstrap once, so the gateway must be up; it never writes to it.

```sh
mkdir -p /tmp/omoverify && cd /tmp/omoverify
cp <this dir>/{server.ts,verify.ts,vite.config.mts,counterfactual.sh,capture-d28.ts} .
ln -sfn /Users/indo/code/project/omon-gateway/web/node_modules /tmp/omoverify/node_modules
bun server.ts &                                     # mock API on :9121
cd /Users/indo/code/project/omon-gateway/web && \
  ./node_modules/.bin/vite --config /tmp/omoverify/vite.config.mts &   # :5199
cd /tmp/omoverify && bun verify.ts                  # 11 checks, exit 0 on all pass
```

The harnesses hardcode `/tmp/omoverify` as their working directory (config path, `OUT` for
screenshots, `results.json`). Stop the mock and Vite afterwards; both bind loopback only.

## Limits

- Sources were served by Vite, not the shipped `web/dist` bundle; nothing was built.
- The mock is a hand-written stand-in for the Rust server: no real latency, no real
  `lane_not_verified` 409s, and no cycle-overlap validation (the server has none).
- Desktop 1440x900 and mobile 390x844 via Bun.WebView on macOS WebKit. Mobile was verified by
  viewport, not by device; no other browser engine was exercised.
- D26 and D27 are not covered here: they were fixed by the tracker-backend and web-shell lanes in
  files outside this lane's write scope.

## sha256

```
a322011d24863a37d8f2635d0cddb42052682107c263bf288ff402fd8265a970  capture-d28.ts
456da52c4f586b5db27510c67b3e50afdf4d16b29d7a0eb7a788ebe63f80d602  counterfactual.sh
397203c2edeea8b4351648a659af0be3879897f6a1be74ef0b08f91afb09caed  d23-cleared.png
a46934115a0b5715f2c047664ceb6f653253600667e7075dcb7fdc4a2b144e56  d25-name-reverted.png
96fa3f4a593c5cc211ed7891efa6517918a307441af5c97c2d74f74f23d406d9  d28-dialog.png
b9704199c4faceb0cad0114a2c6c0fbb82b57a0c63dab69a44dcdf8eb52e0973  d28-reopened.png
5210bedbad2f882967eb25d24f2af651c6d5ec028a7e617cb5bfa54c715a107d  d29-inflight.png
ff2dc8fc15d860d8d2262f75100380bbb49f1e19b8ba4ff69d2ae044928f0cd5  mobile-milestone.png
ebe27d5dfe7146d02899f8665939813f0f04c87a13f5e15039b6cf285859530c  mobile-search.png
a8bbd4c6e8742cdcef7852c9a74e6832500ab246ff5b77b1ee0102e4e18a589c  results-fixed.json
75b8428ff6ba7a54938fe525ec06d96c8d434035f2cf9e92e6e9808b5a9d0c07  server.ts
9c2c08b05d51ee2eb66571c5618e46c421f8c1017f496767f6bbca958fc4735a  verify.ts
da946f7f4594075030cd071913003474361b324f545c2544164bb7838a79bdc6  vite.config.mts
```

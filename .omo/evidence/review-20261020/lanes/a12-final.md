# a12-final lane - render pipeline blocker closed, fixed vs position:static regression proven

Written 2026-10-20T16:55Z (16:55 KST). Owner: senpi-task child `st_01a15828` (hephaestus).
Closes the "pipeline blocker remains" half of `## A12` in `lanes/ops-recovery.md` (read first; its
owner-visible-defect analysis and A/B fix proof are accepted as-is and not re-litigated here).

Repo: `/Users/indo/code/project/content-intel-dashboard` (HEAD `cf15fcb`, no commits made).
Writes in this lane, exactly two files:
- `scripts/render-open-design-artifact.py` (modified)
- `tests/test_render_layout_audit.py` (modified; untracked file created by the other agent)

No other file touched. No publish, no messages, no commit, no shared todo. All other agents' edits
preserved verbatim - see "Scope discipline" at the bottom.

---

## What was broken

`ops-recovery.md` A12, reproduced here from a clean shell:

```
$ python3.12 scripts/render-open-design-artifact.py /tmp/a12repro/prefix/artifact.html /tmp/a12run/prefix
subprocess.CalledProcessError: Command '[... --screenshot=/private/tmp/a12run/prefix/od-card-01.png ...]'
  returned non-zero exit status 2.
EXIT=1
$ ls /tmp/a12run/prefix
od-card-01.png          # correct, 395922 bytes, 1080x1080
```

Headless Chrome writes a complete, correct PNG and *then* exits 2 (`Teardown watchdog expired;
recording dump and terminating.`). Both `screenshot()` and `measure_layout()` used `check=True`, so
the whole render aborted after card 1: no contact sheet, no `od-layout-audit.json`, no `images.json`.
The `--dump-dom` path failed identically, which is why the audit half of the gate was unverifiable.

## What was changed (narrow, artifact-based postcondition)

The exit code is not the postcondition; the artifact this run was asked to produce is. No error is
broadly ignored and no stale file is accepted.

1. **`artifact_state(path)`** - `(size, mtime_ns)` or `None`. Snapshot taken *before* the Chrome call.
2. **`decoded_png_size(path)`** - `Image.open(...).verify()` then a second open with `.load()`, which
   forces Pillow to decode the full pixel stream; returns `(width, height)` or `None`. Pillow is
   already an established dependency of this repo (`scripts/verify-scene2-render.py:16`,
   `scripts/render-local-short-video.py:9`). A file that carries only a valid 24-byte PNG header, or
   a PNG truncated after a partial pixel stream, decodes to `None` and is rejected.
3. **`screenshot_exit_problem(result)`** - the only tolerated non-zero exit is `TEARDOWN_EXIT_CODE`
   (2) *and* only when stderr contains `TEARDOWN_STDERR_MARKER` ("Teardown watchdog expired"). Exit 0
   is accepted. Every other exit code - 1, 134, 139, or 2 without that exact message - is fatal on
   its own, even when the PNG on disk is perfect.
4. **`require_screenshot(output, width, height, previous, result, source)`** - raises `RuntimeError`
   naming `source`, the exit code, every failed condition, and the last 3 stderr lines, unless the
   exit is tolerated *and* the file is *all* of:
   - written (`artifact_state` is not `None`),
   - rewritten by **this** run (`state != previous` snapshot) - a leftover from an earlier run fails,
   - non-empty,
   - a fully decodable PNG (Pillow `verify()` + `load()`),
   - exactly `width` x `height`.

   A healthy Chrome (exit 0) that writes nothing still fails, because the check never looks at the
   exit code to decide success.
5. **`screenshot()`** - drops `check=True`, returns the `subprocess.run` result.
6. **`measure_layout()`** - drops `check=True`; keeps the existing "did not report measurements"
   failure and hardens it: message carries the exit code and page name, `json.loads` failure is
   re-raised as `RuntimeError(... is not parseable ...)`, and a report without a `violations` list is
   rejected. An absent or malformed audit is still fatal - the gate keeps its teeth.
7. **`main()`** - snapshots the previous image / contact-sheet state and calls `require_screenshot`
   for each card and for the contact sheet (`WIDTH` x `270*rows`).

`PANEL_COVER_RATIO` (0.18), `TEXT_COVER_RATIO` (0.25) and `applyLayoutAudit` escalation are
untouched, as `ops-recovery.md` required. Nothing was loosened to make a render pass.

## Evidence 1 - fixed layout renders clean (no overlaps)

```
$ python3.12 scripts/render-open-design-artifact.py /tmp/a12repro/fixed/artifact.html /tmp/a12run/fixed
{"images": ["od-card-01.png", ..., "od-card-06.png"], "contact_sheet": "od-contact-sheet.png",
 "webfont_wait_ms": 10000, "layout_audit": "od-layout-audit.json", "layout_violations": 0}
real 14m30.832s   EXIT=0

$ cat /tmp/a12run/fixed/od-layout-audit.json
{ "violations": [] }
```

Byte identity against the committed fix render (`output/96/attempt-147/...-layout-fix/`), which was
produced before this environment change:

```
card-01 MATCH 9e2413c999c977d0e3791679c2dc524aa0c7bdcad18e46da2c969378bf0e10b3
card-02 MATCH 7d283c1d19c48de8df421fc10ef040a7fcf192ff180a83e6aceb7a4af5d328ba
card-03 MATCH 72e0aeb7458102ec5394aeca93f15a9255598566c7694af3a046f73e7ff13b44
card-04 MATCH dccacc3ee28612aba8f2a55c102efcfbb8be298765dc9cabd0e1627840281ce3
card-05 MATCH ef71cd735a8be91b21da99f64b3dc8faf40f7a90f6f208a52ea05151b5abe823
```
All six PNGs are 1080x1080. The pipeline now runs to completion and reports zero layout violations.

## Evidence 2 - the position:static regression is detected (the important half)

Same script, the fixture whose only difference from `fixed/` is `position:relative!important` ->
`position:static!important` on `> [data-zone="visual"]` (the exact #96 cause):

```
$ python3.12 scripts/render-open-design-artifact.py /tmp/a12repro/prefix/artifact.html /tmp/a12run/prefix
{"images": [...6 cards...], "contact_sheet": "od-contact-sheet.png", "webfont_wait_ms": 10000,
 "layout_audit": "od-layout-audit.json", "layout_violations": 17}
real 9m35.078s   EXIT=0
```

17 violations, concentrated on exactly the cards the defect broke:

| cardIndex | kinds | covering element |
|---|---|---|
| 1 | 1x text-over-text | line over line (`ratio` 0.36) |
| **3** | 5x text-over-panel, 4x text-over-text | `div.map-panel.hook` / `div.map-panel.heart` |
| **4** | 5x text-over-panel, 3x text-over-text | `div.order-card.c1` / `.c3` |

Card 3 verbatim from `od-layout-audit.json` - the same text the owner saw obscured in the pre-fix PNG:
```
{"kind":"text-over-panel","text":"Case study","ratio":1,
 "other":{"label":"div.map-panel.hook","x":81,"y":683,"w":408,"h":242},"cardIndex":3}
{"kind":"text-over-panel","text":"Chaos","ratio":1, ... "cardIndex":3}
{"kind":"text-over-panel","text":"hooks","ratio":0.93, ... "cardIndex":3}
{"kind":"text-over-panel","text":". Healing","ratio":0.68,
 "other":{"label":"div.map-panel.heart","x":591,"y":684,"w":407,"h":241},"cardIndex":3}
```

Render identity proves the fixture really is the broken layout - card-01 of this run is byte-identical
to the *pre-fix* production render:

```
/tmp/a12run/prefix/od-card-01.png                                  0f0167ac...f12d987bf
output/96/attempt-147/open-design-renders/5aef0728-.../od-card-01.png  0f0167ac...f12d987bf   (pre-fix)
output/96/.../5aef0728-...-layout-fix/od-card-01.png               9e2413c9...f0e10b3   (fixed)
```
Cards 02 and 05 hash-match the fixed render (their layout does not depend on the swapped token), and
03/04 diverge from it - the audit flags precisely the two cards whose pixels changed. Fixed => 0
violations, static => 17 violations on cards 3 and 4. The gate discriminates; it is not a constant.

## Correction to the first revision of this file (my defect, found on review)

The first revision of this lane claimed the screenshot was validated as a "decodable PNG". That was
**false**. It read only the 24-byte header, so a 24-byte header-only file was accepted as a valid
1080x1080 screenshot:
```
file size 24
current png_dimensions -> (1080, 1080)      # header-only file accepted
```
That is exactly the stale/partial-artifact class the change was supposed to exclude. Fixed by
replacing the header read with a full Pillow decode (`decoded_png_size`, see item 2 above). The same
probe now returns `None` and `require_screenshot` rejects it.

The first revision also tolerated *any* non-zero exit as long as a file existed. That is now narrowed
to the one exit actually observed on this machine, and only with its exact stderr message
(`screenshot_exit_problem`, item 3 above). Exit 1 / 134 / 139, and exit 2 without the watchdog text,
fail even against a byte-perfect PNG.

Both corrections are pure validation logic. **No render was re-run** - the 9-15 minute captures in
Evidence 1 and 2 are the originals, untouched (`/tmp/a12run/fixed/od-card-01.png` mtime 16:29,
`/tmp/a12run/prefix/od-card-03.png` mtime 16:49). The new validator was replayed against those real
files instead:
```
header-only 24B -> None (rejected)
real fixed/01 -> (1080, 1080)      real fixed/06 -> (1080, 1080)
real prefix/03 -> (1080, 1080)     contact sheet -> (1080, 540)
require_screenshot on all three preserved real renders, exit 2 + watchdog stderr -> PASS
require_screenshot on real fixed/01 with exit 1 -> REJECTED:
  Chrome produced no usable screenshot for card.html (exit 1):
  Chrome exited 1 without the 'Teardown watchdog expired' teardown message | boom
```

Pillow is present in every interpreter the orchestrator could select, so the new top-level import
does not narrow the deployment: `resolveRendererPython()` resolves to `python3.14` on this machine
(no `.venv` present), and `python3.14` imports Pillow 12.1.1. The suite was therefore also run under
that exact interpreter, not just `python3.12`:
```
$ python3.14 -m unittest tests.test_render_layout_audit tests.test_render_open_design_artifact
Ran 26 tests in 1.701s
OK
header-only 24B -> None      real fixed/01 -> (1080, 1080)      (under python3.14)
```
`python3.11` on this box has no Pillow, but it is 4th in the resolver's candidate order and is never
reached while `python3.14` exists.

## Evidence 3 - tests

```
$ python3.12 -m pytest tests/test_render_layout_audit.py tests/test_render_open_design_artifact.py -q
..........................                                               [100%]
26 passed, 3 subtests passed in 0.76s

$ python3.12 -m pytest tests/ -q          # whole repo python suite
................................................                         [100%]
48 passed, 22 subtests passed in 2.19s
```

11 pre-existing tests still pass (including `test_screenshot_waits_for_webfonts_before_capture`, which
monkeypatches `subprocess.run` - `screenshot()` still returns that result, so the stub signature is
unchanged). 15 new tests in `tests/test_render_layout_audit.py`, all deterministic, no browser, no
sleeps:

- accepts a fresh valid 1080x1080 PNG written before a teardown exit 2;
- accepts a clean exit 0 that wrote a valid screenshot;
- rejects exit 1 / 134 / 139 even with a byte-perfect PNG on disk (3 subtests);
- rejects exit 2 when the watchdog message is absent;
- rejects a run that wrote nothing even when Chrome exits 0;
- rejects a stale screenshot left by an earlier run (mtime/size snapshot);
- rejects an empty file, a non-PNG payload, and a wrong-size PNG (1080x270);
- **rejects a 24-byte file with a valid PNG header** (the truncated regression);
- **rejects a real PNG truncated after a partial pixel stream** (first half of a valid file);
- surfaces exit code and stderr tail in the failure message;
- `measure_layout` parses the report from a `--dump-dom` that exits 2;
- `measure_layout` fails on a missing attribute, on unparseable JSON, and on a report with no
  `violations` list.

## Why the runs took 9-15 minutes

Each Chrome launch now waits for the teardown watchdog (~75 s) before exiting, so 6 cards + 1 contact
sheet + 6 `--dump-dom` calls cost ~13 launches per artifact. That is environment slowness, not script
work (`user 0m43s`). No retry loop, no polling, and no sleeps were added to the script; the validation
is a single stat + 24-byte header read per artifact.

## Residual, not fixed here (out of scope, stated so it is not mistaken for done)

- `cardIndex 1` text-over-text in the **prefix** fixture only. It is present in the broken render and
  absent from the fixed one, so it is part of the same regression, not a separate defect.
- The pre-existing `output/96/attempt-147/...` artifacts were not re-rendered; the pipeline that
  produces them is what this lane unblocked, and the lead owns re-running topic #96 end to end.
- Chrome processes started by these two runs are all reaped: `pgrep -f "headless=new"` -> 0,
  `pgrep -f a12run` -> 0.

## Scope discipline

`git status --short` after my edits still shows only other agents' work plus my two files:
```
 M config/projects.json                            (other agent)
 M lib/open-design-orchestration.ts                (other agent - the position:relative fix)
 M lib/social-post-generation.ts                   (other agent)
 M scripts/publish-ready-digest.ts                 (other agent)
 M scripts/render-open-design-artifact.py          <-- this lane
 M tests/unit/publish-button-action.test.ts        (other agent, appeared mid-lane - untouched)
 M tests/unit/social-post-generation.test.ts       (other agent)
?? lib/card-news-layout-audit.ts                   (other agent)
?? tests/test_render_layout_audit.py               <-- this lane (added tests to the other agent's new file)
?? tests/unit/card-news-layout-audit.test.ts       (other agent)
?? videos/gitai-scion-cn/                          (other agent)
?? videos/hotspot-proxy-motion/                    (other agent)
```
No revert, no overwrite, no `git checkout`, no commit. `scripts/render-open-design-artifact.py` diff is
+269/-16, all inside the audit/postcondition code paths.

**A12 status: pipeline blocker CLOSED.** Fixed layout renders end to end with zero violations; the
known `position:static` regression is detected with 17 violations naming cards 3 and 4 and the
collage panels covering the headline text.

---

## Preservation: `external-patches/a12-render.patch`

The A12 changes live in `content-intel-dashboard`, so they cannot ride a gateway commit. They are
preserved as a reviewable patch containing **only** the hunks written by this lane:

```
.omo/evidence/review-20261020/external-patches/a12-render.patch
  433 lines, 19994 bytes, sha256 11b4806b7a2f77ecbf6f67205605ea08473f773c1f889ca1a21b36f1d01dbc4a
```

**The FROM state is the other-agent worktree, not HEAD.** This is the one place where this patch
differs from its sibling `e19-producer.patch`, which diffs against that repo's HEAD. My hunks sit on
top of another agent's in-flight edits to the same file (the `LAYOUT_AUDIT_SCRIPT`, the audit
constants, `measure_layout`), so a HEAD-based diff would have swept their work into my patch.

```
FROM  scripts/render-open-design-artifact.py   blob 608cc0aa   sha256 3df7c6af...
      tests/test_render_layout_audit.py       blob f40de19c   sha256 e369c2ff...
TO    scripts/render-open-design-artifact.py   blob 315dab9b   sha256 ed87c32a...
      tests/test_render_layout_audit.py       blob 4ae465e4   sha256 ba495405...
base commit (HEAD, untouched)                 cf15fcb9555698d641f090459e5400954bb79d17
```

Verification of the artifact itself:
- `diff` against the verified candidate -> **IDENTICAL** (same sha256, byte for byte).
- applied to a fresh copy of the FROM state -> `apply exit=0`, both files **byte-identical** to the
  live worktree; the patched tree passes `pytest tests/test_render_layout_audit.py` (20 passed).
- applied to that repo's **HEAD** -> **fails by design**: `5 out of 8 hunks failed`, `No such line
  255 in input file`, plus `2 out of 2 hunks ignored` for the test file, which does not exist at
  HEAD. This is the documented prerequisite, not a defect in the patch.

**Dependency on preexisting layout correction (other agent, NOT in the patch):**
`lib/open-design-orchestration.ts` (`position:static` -> `position:relative` on
`> [data-zone="visual"]`), `lib/card-news-layout-audit.ts`, and the renderer's own audit scaffolding.
Without them this patch only changes how failures are reported - it cannot fix or detect anything.
The full statement is in the patch preamble.

**Deploy scope:** pushing the gateway does **not** deploy, build, install, or propagate this patch.
The gateway tree is evidence storage; `content-intel-dashboard` is a separate repo with its own
checkout, and nothing in the gateway's publish path touches it. Landing this work requires committing
it in `content-intel-dashboard`.

No commit was made in either repo and no source file was modified to produce the patch.

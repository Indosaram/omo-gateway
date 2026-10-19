# Linear tracker QA evidence (lead, 2026-10-16T15:22:47.605Z)
Repo: /Users/indo/code/project/omon-gateway (uncommitted). Plan: .omo/plans/linear-tracker.md
## Gates
- web: `bun run build` (tsc -b strict + vite) EXIT=0 — /tmp/tracker-qa/web-build-1.log, web-build-2.log (after CSS fix)
- cargo build EXIT=0 — /tmp/tracker-qa/cargo-build-1.log, cargo-build-2.log
- cargo clippy --all-targets -- -D warnings EXIT=0 — clippy-1.log, clippy-2.log
- cargo test --no-fail-fast (whole repo) EXIT=0: 79 suites, 1607 passed, 0 failed — cargo-test-1.log
- cargo test --test test_tracker_api: 6/6 (after lanes.rs fix) — tracker-test-2.log
## API QA (scratch DB /tmp/tracker-qa/omon.db, dashboard 127.0.0.1:9334): 38/38 — api-qa-1.log
## Browser QA (omowright, Chrome headless, 1440 + iPhone14 390, dark+light): 19/19 — shots/browser-qa-log.txt, shots/*.png
## Linear comparison page: /tmp/tracker-qa/compare.html (official Linear doc screenshots in ref/)
## Fixes after first QA
- lanes::sync: human-accepted lane move is held only while a close/reopen request is pending (lane_close_intents / lane_owner_replies), otherwise converges to real lane status; test reopen_request_holds_state_only_while_pending.
- linear.css: button reset lowered to :where() specificity so Tailwind border/padding on tab pills render (inbox filter tabs, projects tabs).
## Known gaps
- Issue JSON has no subscriber_ids -> My issues "Subscribed" tab approximates (created/assigned).

## Added after gate review started
- hotkeys.ts eventKey: also treat event.code==='Space' as space (CDP/IME keydown with empty key). Before fix: Space peek did not open (logged key="" code=Space). After: peek opens & closes with Space at 1440 and 390 (full-width sheet 390px, no h-scroll). shots/B6-peek-desktop-1440.png, B6-peek-mobile.png, extra-log.txt
- Timeline layout via Display > Timeline renders gantt with today line: shots/B-timeline-desktop-dark.png
- web bun run build EXIT=0 again — web-build-3.log

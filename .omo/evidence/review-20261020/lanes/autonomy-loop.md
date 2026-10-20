# Lane autonomy-loop

Scope: A1, A5, A10, A11, A14, B5 in `src/autonomy/loop.rs` + `src/autonomy/mod.rs`. No other files
edited. No commits, no builds, no test runs, no calendar writes, no messages sent.

**Compiler status: UNVERIFIED.** The gate build (`review-20261020-integrated1`) reported 6 type
errors in `src/autonomy/loop.rs` (12 log lines: each error prints both the call site and the
"function defined here" note). All 6 were fixed by wrapping the `event_body` argument in `Some(...)`
at the call sites the log named (1184, 1204, 1253, 1261, 1268, 1274; `stub_calendar` takes
`Option<&str>`, and `None` at 1165 was already correct). `rustfmt --check` passes, which proves the
file still parses — it does not prove it compiles. Only the gate can.

## Item status

A1 FIXED src/autonomy/loop.rs:247,354 — `emit` logs `action_loop enqueued` with per-status counts
from `outcome_counts_for_lead`; enqueue is never reported as completion. Test:
`item_is_done_only_when_the_calendar_reads_the_event_back_for_it` asserts the emitted body and that
an item is not done on `executed`. Live gap: not deployed, so the running loop still logs the old
wording; the new log line itself is not asserted by a test.

A5 FIXED src/autonomy/loop.rs:876,933 — items arrive source-agnostically from `inbox_decisions`
(class bot/owner, any source) and are done only after the calendar reads the event back. Tests:
`item_is_done_only_when_the_calendar_reads_the_event_back_for_it`,
`verified_without_a_calendar_readback_is_refused`. Live gap: no end-to-end item-to-event case has
ever run; the real pending item below is still unverified and no event exists for it.

A10 FIXED src/autonomy/loop.rs:706 — a delivered event with no outcome after `REQUEUE_AFTER`
(30 min) is marked `reconcile:` before re-emission, re-emitted as `[ACTION_LOOP_RECONCILE]` with
`[reconcile attempt n/3]`, then `failed` at `MAX_ATTEMPTS = 3`; `jev_skip` is terminal and consumes
no attempt. Tests: `item_without_outcome_is_requeued_as_reconcile_then_failed`,
`judge_skipped_event_does_not_burn_attempts`. Live gap: never exercised against a real failed turn.

**Upgrade gap this exposed, and the narrow fix:** `requeue_stale_outcomes` selects *from*
`action_loop_outcomes`, so an item delivered before 0054 (seen mark, no row) can never be reconciled
or recorded — `record_outcome` refuses it as unknown, and the scan skips it forever because the seen
mark is permanent. That is unfinished work silently lost across the upgrade. New explicit command
`omo-gateway autonomy adopt <item-key> --external-id <id> [--detail <note>]`
(`adopt_legacy_item`, loop.rs:689) closes it without loosening anything:

- requires a real `inbox_decisions` row of class `bot`/`owner` **and** an `action_loop_seen` mark
  (the item must genuinely have been taken), and refuses any item that already has an outcome row;
- requires the **same exact provider readback as verification** — the provider must return that id
  and the event's parsed `description` must carry this item key as a whole token — so an artifact
  must already exist and be tied to *this* item;
- writes the row as `executed` (never `verified`) with `detail = "legacy-reconcile:readback=calendar:<id>"`
  so provenance is unmistakable;
- leaves `lane_event_id` NULL, which keeps the row out of `requeue_stale_outcomes` (inner join on
  `lane_events`) so the item can never be re-emitted as a fresh side effect, and leaves the seen
  mark untouched so the scan never picks it up again;
- it is one explicit item per call and nothing calls it automatically — there is no backfill.
  Tests: `legacy_seen_item_is_adopted_as_executed_after_readback`,
  `adoption_refuses_unseen_unknown_and_wrong_key_items`,
  `adoption_is_explicit_and_never_backfills_the_seen_backlog`.

After adoption the item is `executed`; the normal `outcome --status verified` path still applies and
still performs its own readback, so adoption is not a shortcut to `verified`.

Readiness for the final gate: implementation and its three tests are complete, both files are
`rustfmt --check` clean, there is no `#[allow]` in either, and no other file was touched. Clippy
risk is limited to what the gate already accepted: `LegacySeenRow` follows the existing
`InboxCandidateRow`/`OutcomeItemRow` tuple-alias idiom that strict clippy passes, and no inline
`Vec<(...)>` annotation was added. Source is frozen from this lane until the gate reports.

A11 FIXED migrations/0054_action_loop_outcomes.sql + src/autonomy/loop.rs:316 — `enqueued` row
written in the same transaction as the `lane_events` row and the `action_loop_seen` mark; states
`enqueued -> executed|verified|failed`, with `reconcile:`/`reconciled:` in `detail`. Live gap:
**migration 0054 is not applied** (`_sqlx_migrations` = 53, table absent). The deployed binary does
not contain this code, so nothing fails today; the constraint is deploy order — 0054 must land
before this code ships, or `flush_coalesced` fails at its outcome INSERT.

A14 FIXED src/autonomy/mod.rs:54,194,211 + src/autonomy/loop.rs:488,542,659,688 — CLI
`omo-gateway autonomy outcome <key> --status executed|verified|failed [--external-id <id>]
[--detail <why>]`. `verified` runs a bounded (`READBACK_TIMEOUT` 30s, `kill_on_drop`) provider call
`<cmd> cal get <id>`, parses its YAML into `id`/`description`, requires the parsed id to match and
the description to carry the item key as a whole token (`contains_exact_token`, so `inbox:1` is not
verified by `inbox:10`). Two structural points from the final pass: the record path takes a grouped
`OutcomeReport { status, external_id, detail }` payload and a `Readback { program, timeout }` seam,
so no `#[allow(clippy::too_many_arguments)]` is needed anywhere (there is no lint suppression in
either file); and `verified` is terminal — a delayed `executed`/`failed` from an earlier attempt is
refused both by a status read and by the update's own `WHERE ... AND (status <> 'verified' OR ? =
'verified')` guard, so a late turn cannot erase a confirmed result. Tests:
`item_key_matches_only_as_a_whole_token`,
`readback_matches_the_parsed_id_and_the_whole_item_key`,
`readback_is_bounded_and_kills_a_hung_provider` (now drives the real record path with a 300ms
`Readback`), `verified_row_is_terminal_against_a_delayed_report`. Live gap: the readback has never
run inside the gateway process; it was only exercised by hand and by stub scripts.

### Gate run 2: `clippy::type_complexity` at loop.rs:978 and 1013 (fixed)

`review-20261020-integrated2/clippy.log:2,12`: "very complex type used" on the two inline row
tuples in `scan_unseen_candidates` —
`Vec<(String, String, String, Option<String>, DateTime<Utc>)>` (GitHub rows) and
`Vec<(i64, Option<String>, String, String, DateTime<Utc>)>` (lane-event rows). Fixed by naming both,
next to the existing `InboxCandidateRow`/`OutcomeItemRow` aliases, which is the file's own idiom:

```rust
type GithubCandidateRow = (String, String, String, Option<String>, DateTime<Utc>);
type LaneEventCandidateRow = (i64, Option<String>, String, String, DateTime<Utc>);
```

No `#[allow]`/suppression added anywhere; `grep -c "allow("` is 0 in both autonomy files. The
remaining gate-2 clippy errors are other lanes' files (`src/slack/commands.rs:504`,
`src/context/pack.rs:667`, `src/inbox/sources/mail.rs:220`, `src/slack/backfill.rs:628,629`,
`src/tracker/issues.rs:1394`) and are outside this lane's write scope.

### Gate run 1 failures (both were test defects, not readback defects)

`review-20261020-integrated1/test.log:1129,1135`: `...stub-cal cal get evt-1` and `...stub-cal cal
get gcal-evt-42` both `failed (exit status: 1): event "..." not found`, panicking at loop.rs:1265 and
loop.rs:1177. Two separate mistakes in the tests, both now fixed:

1. **Wrong message asserted.** A stub built with `event_body: None` exits non-zero (matching real
   `zele`), so the code takes the `!status.success()` branch and reports
   ``failed (exit status: 1): event "<id>" not found``. The `does not hold` branch is only reachable
   when the provider *succeeds* but returns a different event. The tests asserted `does not hold`
   for both, so they failed on a correct refusal. Fix: the not-found cases assert `not found`; the
   id-mismatch case now uses a new `stub_calendar_any` stub that exits 0 while returning a
   different `id`, so the real mismatch branch is exercised instead of weakened.
2. **One shared fixture filename.** `stub_calendar` wrote every stub to `<dir>/stub-cal`, so two
   stubs in the same directory overwrote each other and any multi-stub test was order-dependent.
   Fix: `write_stub` derives the filename from a content hash plus a tag, so fixtures are distinct
   by construction. Readback itself was not relaxed: every branch is still reached by a real child
   process, and each assertion now names the message its own branch emits (mapped explicitly:
   `not found` = provider refuses; `does not hold` = provider returns another id; `does not carry` =
   event lacks the item key; `as YAML` = unparseable output; `cannot run` = provider missing;
   `did not answer within` = timeout).

B5 FIXED src/autonomy/loop.rs:933 — the GitHub query excludes items that have a bot/owner
`inbox_decisions` row, so a triaged item appears once, as `inbox:<id>`. Test:
`triaged_github_item_appears_once_per_batch`. Live gap: none beyond the gate run.

A14 DOC FIXED src/autonomy/mod.rs:51-56 — the `Outcome` variant doc said `verified` "is refused ...
no such readback is wired into this gateway", which stopped being true once loop.rs:640 wired it.
The comment now states the real contract (provider readback via `<calendar cmd> cal get`, 30s bound,
parsed `id` must match, parsed `description` must carry the item key as a whole token, and a result
the gateway cannot read back cannot be verified). Doc-only change; no code path touched.

## Live calendar path (read-only, hand-run)

- `zele whoami` -> 1 account `freedomzero91@gmail.com`, google, capabilities `gmail, calendar, smtp`,
  Authenticated (expires 2026-10-20T07:20:01Z).
- `zele cal list` -> `과외` (primary), `대한민국의 휴일`, `가족`, `테스트 1단원`.
- `zele cal get d5546361-3ea2-4c84-905e-206455d2bb2d@zele` -> exit 0, clean YAML on stdout (the
  node ExperimentalWarning goes to stderr).
- `zele cal get evt-iBM9H0fu10lGd0U@events.lu.ma` -> adds a 713-char `|-` description and an
  `attendees` list; parses fine, so the `id`/`description` struct fits real output.
- `zele cal get bogus-event-id-xyz` -> exit 1, stderr `event "bogus-event-id-xyz" not found`.

## Original thread verification (separate evidence, archive read-only)

Resolved from the PirateTalk archive, not from inference: room `482853575146352`, type `DirectChat`,
title `C`, 2 members, 37 messages, 2026-10-20 14:24:55 .. 15:56:12 KST. The inbox stream only holds
C's messages, which is exactly why it was ambiguous — the owner's replies are in the archive.

Decisive message, logId `3955573033146621955`, 2026-10-20T15:37:31+09:00, `isMe`:

> 네 ㅎㅎ 그러면 https://meet.google.com/wzz-onbp-iyu 여기서 오늘 6시에 뵙겠습니다.

("then let's meet **today at 6** at this Meet link.") Supporting: C at 15:35:18 (`3955571921582252033`)
"아 네 저도 오늘 6시나", at 15:35:43 (`3955572133604319234`) "내일 6시 모두 가능한데 혹시 30분
이하로 짧게 미팅 가능할까요?"; owner at 15:34 "한국시간으로 오후 5시 이후면 평일은 대부분
가능합니다". Nothing after 15:37 reschedules (15:55 커리큘럼 송부드립니다, 15:56 `curriculum.html`).

**Verdict: today 2026-10-20 18:00 KST, 30 minutes or less, Google Meet already sent.** Not
tomorrow. The archive settles it; no user question is needed.

Live inbox item for the same request:
`inbox:52b475ee5c1ba18ce3a7e9cf96b92bcad8603ae6636462fbffa3176bc9cef512` (kakao, author C,
received 2026-10-20T06:35:43Z = 15:35:43 KST, `class=owner`, `status=open`, no `outcome`, no
`action_loop_seen`, no proposal).

Calendar state: `zele cal events --today` -> no events; `--from 2026-10-20 --to 2026-10-21` -> only
`YBM전화화상 무료체험` on 10-21 08:00. **No event exists for this meeting, which starts ~1h55m from
16:05 KST.** Nothing was created; creating it is the owner's call, and this lane sent nothing.

Proposed event, if the owner approves (NOT created by this lane):

```
calendar: primary (과외)
summary:  C 사전 미팅 (비대면, EN)
start:    2026-10-20T18:00:00+09:00   (today, "오늘 6시")
end:      2026-10-20T18:30:00+09:00   (30분 이하)
location: https://meet.google.com/wzz-onbp-iyu
description:
  inbox:52b475ee5c1ba18ce3a7e9cf96b92bcad8603ae6636462fbffa3176bc9cef512
  키프로스 igaming C사, 비개발자 5명 내외 대상 5시간 교육 사전 미팅.
  요청: 진행 방식 개괄 설명 (비대면, 영어).
```

## External calendar write: LEAD-EXECUTED, not autonomous pipeline E2E

**This is not proof that A5 works.** The event below was created and then updated by the lead
running `zele` by hand, outside the gateway process. No `action_loop_outcomes` row exists for the
item, no `[ACTION_LOOP]` event ever carried it, and `omo-gateway autonomy outcome ... --status
verified` has never run (it cannot: migration 0054 is unapplied and the code is not compiled into a
running binary). The autonomous path — scan → emit → lead turn → `record_outcome` → `read_calendar_event`
— has still never executed end to end.

Event: `61b93aaa-a3fd-4182-9cad-483a6dc084b2@zele`, created then updated by the lead, both exit 0.

Observed chain, in order (each line is a command someone actually ran; the last two were re-read by
this lane, read-only):

1. **Pre-create duplicate search** `zele cal events --filter 'inbox:52b475' --days 2` -> exit 0,
   `items: []`. Nothing existed yet, so the create was not a duplicate.
2. **Create** (lead) -> exit 0, new event `61b93aaa-...@zele`, today 18:00-18:30 KST.
3. **First readback surprise** — this lane ran `zele cal get 61b93aaa-...@zele` (read-only) and got
   `exit 0`, correct `id`/`start`/`end`, the item key present, but **no `location:` line** and
   `meet: https://meet.google.com/hqi-gmsq-tmb` — a Google-generated code, *not* the agreed
   `wzz-onbp-iyu`. The agreed link was therefore not the one a naive reader would follow.
4. **Mechanism, read from the installed provider source (fact vs. hypothesis kept apart)**:
   *Facts from source* — `createEvent` (`~/.bun/install/global/node_modules/zele/dist/calendar-client.js`)
   destructures `withMeet` and never uses it, so `--meet` cannot mint a link; `buildICalString`
   *does* set `event.location` from `props.location`, so `--location` was written into the iCal;
   `icsEventToCalendarEvent` sets `location: event.location ?? ''`, and `printEventDetail` prints
   `location` only `if (event.location)`, so a missing line means the fetched iCal had no LOCATION;
   `meetLink` is derived by regex from the **description**
   (`desc.match(/https:\/\/meet\.google\.com\/[\w-]+/)`), taking the *first* match — which is
   why the agreed link surfaces as `meet:` only once it appears in the description.
   *Hypothesis, not verified* — that Google's CalDAV round trip drops `LOCATION` while rewriting the
   description with its own `Join with Google Meet:` block. The observation is solid (written on
   create/update, absent on `get`); the exact reason is not, and this lane did not probe the raw
   CalDAV object to settle it.
5. **Update** (lead) -> exit 0, same event id, the original `wzz-onbp-iyu` written explicitly into
   both the description and `--location`; the CLI's immediate result reported `meet=wzz-onbp-iyu`.
6. **Final readback** — this lane ran `zele cal get 61b93aaa-...@zele` again (read-only, 16:24 KST):
   `exit 0`; `id` `61b93aaa-a3fd-4182-9cad-483a6dc084b2@zele`; `start: Oct 20, 6:00 PM`;
   `end: 6:30 PM`; description's first token is the exact inbox key; the agreed
   `https://meet.google.com/wzz-onbp-iyu` is present and explicitly labelled as the real link;
   `meet: https://meet.google.com/wzz-onbp-iyu`.
7. **Final duplicate count** `zele cal events --filter 'inbox:52b475' --days 2` -> exactly 1 event
   (`61b93aaa-...`). No duplicate was created.

Honest caveats, recorded rather than smoothed over:

- **`location` did not persist as readable state.** The final `get` still prints no `location:` line.
  The `--location` write is not verifiable through this provider, so this report does not claim it
  persisted. What is verified is the description text and the parsed `meet:` field.
- **Google appends its own Meet block.** The final description carries *two* links: the agreed
  `wzz-onbp-iyu` (explicitly labelled as the real one) and a Google-generated
  `https://meet.google.com/djh-uont-fsy` inside the auto-managed `Join with Google Meet:` section
  (a different code from the first readback's `hqi-gmsq-tmb`, because Google rotates it on update).
  The explicit sentence in the description directs the reader to `wzz-onbp-iyu`; the auto-section
  cannot be removed through this provider.
- **The event's own `meet:` field is now correct** (`wzz-onbp-iyu`), because the provider takes the
  first `meet.google.com` match from the description and the agreed link is placed first.
- The item `inbox:52b475ee...` is still `open` in the live DB with no outcome row: the work happened,
  but the gateway does not know it, and cannot be told until 0054 is applied and the binary is built.

For the real E2E, the same item must go through the autonomous path: emit, act, then
`omo-gateway autonomy outcome inbox:52b475... --status verified --external-id 61b93aaa-a3fd-4182-9cad-483a6dc084b2@zele`
— which `read_calendar_event` would accept, since the parsed `description` carries the key as a
whole token. That command has never been run.

## Verification actually performed

- `rustfmt --edition 2021 --config skip_children=true --check src/autonomy/loop.rs src/autonomy/mod.rs`
  -> clean (parses and is formatted; not a type check).
- The 6 gate errors above, read from `.omo/evidence/review-20261019/gate/review-20261020-integrated1/clippy.log`
  and fixed at the exact call sites.
- Read-only `sqlite3` queries against `file:omon_gateway.db?mode=ro`.
- Read-only `zele` commands and one read-only PirateTalk archive fetch
  (`/api/rooms`, `/api/rooms/482853575146352/messages`); no writes, no sends.
- A hand-built stub calendar CLI reproducing `zele`'s exit codes; a Python YAML parse of two real
  `zele cal get` outputs.

Not run: no cargo build, test, or clippy (gate owns them). 13 tests are written and unexecuted:
`item_is_done_only_when_the_calendar_reads_the_event_back_for_it`,
`verified_without_a_calendar_readback_is_refused`, `item_key_matches_only_as_a_whole_token`,
`readback_matches_the_parsed_id_and_the_whole_item_key`,
`readback_is_bounded_and_kills_a_hung_provider`,
`verified_row_is_terminal_against_a_delayed_report`,
`unreconciled_item_cannot_be_closed_by_a_bare_executed`,
`item_without_outcome_is_requeued_as_reconcile_then_failed`,
`judge_skipped_event_does_not_burn_attempts`,
`item_from_a_long_outage_is_emitted_as_backlog_not_dropped`,
`a_lost_seen_mark_does_not_resurrect_a_taken_item`,
`another_lead_does_not_take_over_an_emitted_item`, `triaged_github_item_appears_once_per_batch`.

Remaining, in order: 1) gate re-run must show loop.rs clean; 2) `cargo test --lib
autonomy::action_loop`; 3) confirm `serde_yaml` accepts every shape `zele cal get` prints — the
updated event is a useful sample: its description is a multi-line block scalar containing two Meet
URLs and a `-::~:~::` separator block; 4) apply 0054 before deploying; 5) run the **autonomous**
E2E on this same item (the calendar side already exists; the gateway side has never run) — the
lead's manual `zele` create/update is not that proof; 6) live reconcile case; 7) confirm the daily
cap counts events per flush. (The doc fix at mod.rs:51-56 is done.)

## Installed `zele cal create` contract (from `--help` and the installed source, read-only)

Installed package: `~/.bun/install/global/node_modules/zele`, version `0.8.0`.
`zele cal create --help` prints:

```
Usage:
  $ zele cal create

Options:
  --calendar <calendar>        Calendar ID (default: primary)
  --summary <summary>          Event title
  --from <from>                Start time
  --to <to>                    End time (or +1h, +30m, +2d relative to --from)
  --description <description>  Event description
  --location <location>        Event location
  --attendees <attendees>      Comma-separated attendee emails
  --meet                       Create Google Meet link
  --all-day                    All-day event
  --recurrence <recurrence>    Recurrence rule (e.g. RRULE:FREQ=WEEKLY;BYDAY=MO)
  --reminder <reminder>        Reminder as method:duration (e.g. popup:15m)
  --color <color>              Event color ID (1-11)
  --visibility <visibility>    Event visibility (default, public, private)
  --account [account]          Filter by email account (repeatable)
  -h, --help                   Display this message
```

Behaviour read from `src/commands/calendar.ts` (`cal create` action) and
`src/calendar-client.ts` (`createEvent`), not guessed:

- Required: `--summary`, `--from`, `--to`. Missing any one -> `out.error(...)` to stderr and
  `process.exit(1)`.
- `--from` is parsed by `parseTimeExpression` (`src/calendar-time.ts:137`). An RFC3339 value that
  already carries an offset (`+09:00`) or ends with `Z` is returned **as-is**, so passing
  `2026-10-20T18:00:00+09:00` needs no timezone inference. Bare `YYYY-MM-DDTHH:MM` is interpreted
  in the calendar's timezone from `getTimezone(calendarId)`; `today`/`tomorrow`/`now`/weekday names
  resolve to **00:00** local, not to a meeting time.
- `--to` accepts either an absolute expression or a `+<n><m|h|d|w>` duration
  (`DURATION_RE = /^\+(\d+)(m|h|d|w)$/`), which is added to the parsed start. So `+30m` is exact.
- All-day is triggered by `--all-day` **or** by both dates being bare `YYYY-MM-DD`
  (`isDateOnly`), in which case the end date is made exclusive (+1 day). Passing timestamps avoids it.
- `--description`, `--location`, `--attendees` (comma-split, trimmed), `--recurrence` (single rule,
  wrapped in an array), `--reminder` (`method:duration`, method must be `popup` or `email`),
  `--color`, `--visibility` are passed straight through to `createEvent`.
- **`--meet` is accepted but does nothing on this path.** `createEvent` destructures `withMeet`
  and never uses it: the event is built by `buildICalString({ uid, summary, start, end, allDay,
  description, location, attendees, transparency, visibility, organizer })`, which has no
  conference/Meet field, and `createCalendarObject` posts only that iCal. So the flag cannot be
  relied on to mint a Meet link — the link must be supplied explicitly via `--location`.
- Event id: `createEvent` generates `uid = ${crypto.randomUUID()}@zele` (e.g.
  `d5546361-3ea2-4c84-905e-206455d2bb2d@zele`, the shape seen on real events here). That uid is the
  `id` to pass to `omo-gateway autonomy outcome ... --external-id`.
- Output: `printEventDetail` writes YAML to **stdout** (`id`, `summary`, `start`, `end`, plus
  `description`, `location`, `meet` only when a meet link exists, `attendees`, `link`), and
  `out.success('Event created')` goes to **stderr** (`console.error`). So stdout stays parseable.
- Lookup by item key: `zele cal events --filter <text>` matches `summary`, `description` and
  `location` case-insensitively (`src/calendar-client.ts:556-562`), which is how a pre-existing
  event can be found before creating a duplicate.
- `--calendar` accepts the id printed by `zele cal list` (`과외` = `freedomzero91@gmail.com`,
  primary) or the literal `primary`.

### Exact command for the confirmed meeting (NOT run; nothing created)

Confirmed from the archive: owner replied at 2026-10-20T15:37:31+09:00
(logId `3955573033146621955`) "오늘 6시에 뵙겠습니다" with `https://meet.google.com/wzz-onbp-iyu`.
So today 2026-10-20 18:00 KST, 30 minutes.

```sh
zele cal create \
  --calendar primary \
  --summary 'C 사전 미팅 (비대면, EN)' \
  --from '2026-10-20T18:00:00+09:00' \
  --to '+30m' \
  --location 'https://meet.google.com/wzz-onbp-iyu' \
  --description 'inbox:52b475ee5c1ba18ce3a7e9cf96b92bcad8603ae6636462fbffa3176bc9cef512 | 키프로스 igaming C사, 비개발자 5명 내외 대상 5시간 교육 사전 미팅. 요청: 진행 방식 개괄 설명 (비대면, 영어).'
```

Notes for whoever runs it:
- The description's first token is the item key, which is what `read_calendar_event` requires
  (whole-token match on the parsed `description`), so `autonomy outcome ... --status verified`
  can succeed afterwards.
- `--to '+30m'` yields 18:30 KST from the 18:00 start; no timezone inference involved.
- Do not pass `--meet`: it is inert here, and the real link is already in `--location`.
- Before running, check for a duplicate with
  `zele cal events --filter 'inbox:52b475' --days 2` (currently `No events found`, checked 16:15 KST).
- Capture stdout: its `id:` line is the `--external-id` for the outcome command.

## Post-deploy live verification plan (design only; nothing executed, nothing written)

Built from read-only inspection on 2026-10-20 08:32Z (17:32 KST). No rows were created, no calendar
call was made, no `verified` was hand-set.

### What the live DB says about eligibility right now

- **Eligible candidates: 0.** The scan's own predicate (`class IN ('bot','owner')`, no
  `action_loop_seen` mark, no `inbox_proposals` row, no `inbox_urgent` event carrying the key)
  matches nothing:

  ```sh
  sqlite3 "file:/Users/indo/code/project/omon-gateway/omon_gateway.db?mode=ro" "
  SELECT COUNT(*) FROM inbox_decisions d
  WHERE d.class IN ('bot','owner')
    AND NOT EXISTS (SELECT 1 FROM action_loop_seen s WHERE s.item_key='inbox:'||d.item_id)
    AND NOT EXISTS (SELECT 1 FROM inbox_proposals p WHERE p.item_id=d.item_id)
    AND NOT EXISTS (SELECT 1 FROM lane_events e
                    WHERE e.kind='inbox_urgent' AND e.body LIKE 'inbox:'||d.item_id||'%');"
  ```

  Also 0 undelivered `lane_event:` candidates and 0 `source_health` rows. The loop is idle until
  new traffic arrives — it is not blocked, it has nothing to do.
- **Traffic is organic and frequent**: `action_loop` events exist at 05:53, 06:55, 07:42 (ids 149,
  151, 152), each delivered within ~35s. The two items emitted at 07:42 are already seen
  (`inbox:0011fed3…`, `inbox:af0abdc4…`). So a fresh item appears roughly hourly without anything
  being fabricated.
- **The confirmed-meeting pair is already consumed, and cannot be re-entered:**
  `inbox:52b475ee…` got its `action_loop_seen` mark at `06:55:37.534897Z` and went out in event
  **151** (6 items, `delivered_at 06:56:10Z`, `delivery_reason lead_turn`). Note event 151's body has
  **no item keys at all** — the deployed binary predates that change, which is why
  `body LIKE '%52b475%'` finds nothing while the seen mark exists.
  Consequences, stated plainly:
  1. The new binary will not emit it (seen mark is permanent: retention only drops a mark whose item
     row is gone, and `inbox_items` is never pruned).
  2. The new binary will not reconcile it either: `requeue_stale_outcomes` selects **from
     `action_loop_outcomes`**, and no row for it was ever written (the old binary had no such table).
  3. Therefore it can never own an outcome row, and `record_outcome` refuses any item without a row
     (`unknown action-loop item`). So **this item cannot be the scan leg, and cannot be the
     `verified` target** — the readback is tied to the item key, and event `61b93aaa-…` carries
     `inbox:52b475…` specifically.
  The only way to force it back in is to delete its seen mark, which would make the new binary
  re-emit it as a **fresh** item (no reconcile flag) and invite a duplicate event. Do not do that.
  This is a genuine pre-deploy gap: work delivered before the deploy is invisible to the outcome
  pipeline.

### Deploy shape (facts, so the plan names real paths)

- launchd label `ai.omon.gateway`, plist `~/Library/LaunchAgents/ai.omon.gateway.plist`,
  `ProgramArguments = /Users/indo/.omon/bin/omo-gateway run`, `WorkingDirectory` = this repo,
  logs `/Users/indo/.omon/gateway.log` and `gateway.err`.
- `OMON_ACTION_LOOP_ENABLED=true`; **no** `OMON_ACTION_LOOP_CALENDAR_CMD` → readback uses `zele`,
  which resolves on the plist `PATH` (`/Users/indo/.bun/bin/zele`); no `OMON_ACTION_LOOP_DAILY_CAP`
  → default 24; no `OMON_LANE_DEFAULT_LEAD` → lead is the runtime-configured slack session
  `5:slack|-|11:D0C9HNVUK5M|-|11:U04M1JNJM8T|11:U0C9ST69M8B` (the key all 35 action_loop events use).
- `DATABASE_URL=sqlite:///Users/indo/code/project/omon-gateway/omon_gateway.db` (from `.env`).
- **Migration 0054 applies on connect**: `Database::connect` runs `MIGRATOR.run` (`src/storage/db.rs:190`),
  so either the daemon start or the first CLI call migrates. Corollary: the first
  `omo-gateway autonomy outcome …` invocation migrates the DB even before the daemon restarts — fine
  for the schema, but do the restart first so the running loop is the new code.
- **The `outcome` subcommand has no owner gate** (`is_owner_invocation_env` is called for
  `Decide`/`Revoke` only, not `Outcome`), so the lead agent can run it inside its own turn.

### Trace: exact steps, commands and the evidence each hop must leave

**Step 0 — before deploy (read-only).** Record the baseline so the diff is provable:

```sh
sqlite3 "file:/Users/indo/code/project/omon-gateway/omon_gateway.db?mode=ro" "
SELECT MAX(version) FROM _sqlx_migrations;
SELECT COUNT(*) FROM action_loop_seen;
SELECT MAX(id) FROM lane_events WHERE kind='action_loop';"
```

Expected now: `53`, `668`, `152`. Also note the deployed binary's log wording is
`action_loop delivered items=N` (26 occurrences) — it predates A1's fix.

**Step 1 — deploy.** Build (gate), install the binary at `/Users/indo/.omon/bin/omo-gateway`, then
`launchctl kickstart -k gui/$(id -u)/ai.omon.gateway`. Verify, in order:

```sh
sqlite3 "file:…?mode=ro" "SELECT MAX(version) FROM _sqlx_migrations;"   # must be 54
sqlite3 "file:…?mode=ro" ".tables" | tr ' ' '\n' | grep -c action_loop_outcomes   # 1
tail -f /Users/indo/.omon/gateway.log | grep -E 'action_loop (enqueued|items)'
```

**Step 2 — scan → lane event (needs one fresh item; wait for organic traffic).**
The next real bot/owner item produces, within the 30s sweep + 120s coalesce window, a log line in
the **new** wording — this is the A1 proof that enqueue is no longer logged as delivery:

```
action_loop enqueued items=<n> event_id=<id> total_enqueued=<n> total_executed=0 total_verified=0 total_failed=0
```

Evidence to capture (all read-only):

```sh
sqlite3 "file:…?mode=ro" "SELECT id, created_at, delivered_at, body FROM lane_events
  WHERE kind='action_loop' AND id > 152 ORDER BY id DESC LIMIT 1;"
sqlite3 "file:…?mode=ro" "SELECT item_key, status, attempts, lane_event_id, enqueued_at
  FROM action_loop_outcomes ORDER BY enqueued_at DESC LIMIT 5;"
```

Pass conditions: the body now starts `[ACTION_LOOP] n items:` and **each entry is prefixed with its
item key** (`inbox:<id> …`); one `enqueued` row exists per item with `attempts=1` and
`lane_event_id` = that event; `delivered_at` is still NULL at this instant.

**Step 3 — lane event → lead turn.** Wait for `delivered_at` to be set with `delivery_reason =
lead_turn` (the dispatch receipt), and for the lead transcript to contain the event:

```sh
sqlite3 "file:…?mode=ro" "SELECT delivered_at, delivery_reason FROM lane_events WHERE id=<id>;"
sqlite3 "file:…?mode=ro" "SELECT sequence, role, substr(content,1,200) FROM messages
  WHERE session_key='5:slack|-|11:D0C9HNVUK5M|-|11:U04M1JNJM8T|11:U0C9ST69M8B'
    AND content LIKE '%[ACTION_LOOP]%' ORDER BY sequence DESC LIMIT 1;"
```

If the wake judge declines it, `delivery_reason` is `jev_skip` and the new code marks the item
`failed` with "the lead turn never saw it" instead of burning attempts — that is a distinct, also
acceptable outcome, but it does not exercise the readback.

**Step 4 — the agent acts, then records `executed`.** The turn must put the item key in whatever it
creates, then report. For a calendar result that means the description carries `inbox:<id>`, then:

```sh
DATABASE_URL=sqlite:///Users/indo/code/project/omon-gateway/omon_gateway.db \
  /Users/indo/.omon/bin/omo-gateway autonomy outcome inbox:<id> \
  --status executed --external-id <event id> --detail 'created the event'
```

Evidence: `status='executed'`, `external_id` set, `detail` = the note. This is the honest "claim"
state — it is not done.

**Step 5 — provider readback (`verified`), the real proof.**

```sh
DATABASE_URL=sqlite:///Users/indo/code/project/omon-gateway/omon_gateway.db \
  /Users/indo/.omon/bin/omo-gateway autonomy outcome inbox:<id> \
  --status verified --external-id <event id>
```

The gateway itself then runs `zele cal get <event id>` (30s bound, `kill_on_drop`) and requires the
parsed `id` to match and the parsed `description` to contain `inbox:<id>` as a whole token. Pass
evidence: exit 0, and the row reads `status='verified'` with
`detail='readback=calendar:<event id>'`. Independent confirmation, read-only:

```sh
zele cal get <event id>                      # exit 0, id/description carry the key
zele cal events --filter 'inbox:<id>' --days 2   # exactly 1 event, not 2
```

**Step 6 — legacy adoption, which unblocks the existing event today (no calendar write).**
`inbox:52b475ee…` is a genuine legacy item: seen at `06:55:37Z`, delivered in event 151, no outcome
row, and the artifact `61b93aaa-…@zele` carries its key in the description. After deploy:

```sh
DATABASE_URL=sqlite:///Users/indo/code/project/omon-gateway/omon_gateway.db \
  /Users/indo/.omon/bin/omo-gateway autonomy adopt \
  inbox:52b475ee5c1ba18ce3a7e9cf96b92bcad8603ae6636462fbffa3176bc9cef512 \
  --external-id 61b93aaa-a3fd-4182-9cad-483a6dc084b2@zele \
  --detail 'event created by the lead before the upgrade'
```

Pass evidence: exit 0; the row reads `status='executed'`, `external_id=61b93aaa-…`,
`detail` starting `legacy-reconcile:readback=calendar:61b93aaa-…`, and `lane_event_id` NULL; the
seen mark still present; `SELECT COUNT(*) FROM lane_events WHERE kind='action_loop' AND id>152` still
0 (no re-emission). Then the ordinary readback path can confirm it:

```sh
… autonomy outcome inbox:52b475ee… --status verified \
  --external-id 61b93aaa-a3fd-4182-9cad-483a6dc084b2@zele
```

which re-runs `zele cal get` and must land `verified` with `readback=calendar:…`. Scope note: 658
legacy seen inbox items are adoptable in principle; adoption is per-item and explicit, so nothing
else is touched.

**Step 7 — negative probe against the *existing* event (no calendar write, uses a real row).**
Once the Step 2 item has a row, point its verification at the already-created meeting event to prove
the key-tie actually refuses a foreign artifact:

```sh
… autonomy outcome inbox:<new item id> --status verified \
  --external-id 61b93aaa-a3fd-4182-9cad-483a6dc084b2@zele
```

Must fail with `does not carry … as its own token`, and the row must be unchanged. This is the only
way the confirmed-meeting event participates in the verification story, and it is a refusal by
design: the artifact belongs to `inbox:52b475…`, which has no outcome row.

**Step 8 — reconcile path (bounded, needs a second fresh item).** To exercise
`[ACTION_LOOP_RECONCILE]` honestly, take the *next* emitted item and deliberately do not record an
outcome for it. After `REQUEUE_AFTER` (30 min) the sweep must log
`action_loop items are unreconciled: their turns ended without an outcome`, write
`detail='reconcile: …'`, and re-emit it in a separate event whose body starts
`[ACTION_LOOP_RECONCILE]` with `[reconcile attempt 2/3]` on the entry. Then:

```sh
… autonomy outcome inbox:<that id> --status executed        # must be REFUSED (unreconciled)
… autonomy outcome inbox:<that id> --status failed --detail 'checked calendar, no event'  # accepted
```

Pass evidence: the refusal message contains `unreconciled`; the second call writes
`detail='reconciled:attempt 2 recorded as failed; note=…'`; `attempts` reached 2 and the item is not
re-emitted again. Leaving it unanswered to `MAX_ATTEMPTS=3` is the alternative, ending `failed`.

### Bounded scope, and what this plan deliberately does not do

- One new item per leg, one event per item. The daily cap is 24 events per KST day and today has
  used 10 so far (`SELECT COUNT(*) FROM lane_events WHERE kind='action_loop' AND created_at >=
  '2026-10-19T15:00:00'` -> 10), so the legs below have room; the cap would only defer a batch, never
  drop it.
- No calendar writes: the only event in play already exists; the plan creates nothing and the
  negative probe is read-only against the provider.
- No fabricated rows, no hand-set `verified`, no clearing of `action_loop_seen`.
- **Known limitation to accept:** an artifact cannot be reused across items. If C replies on the same
  thread, the new item's key is not in the existing event's description, so that item can only reach
  `executed`/`failed` — it can never be `verified` against the existing event. Verification is
  strictly per-item, by design.

## Rehearsal on a DB copy before production deploy (plan only; nothing run against live DB)

Verified facts, 2026-10-20 18:00 KST:

- `target/debug/omo-gateway` exists (built 17:46, 132 MB) and its `--help` lists both subcommands:
  `adopt <ITEM_KEY> --external-id <ID> [--detail <DETAIL>]` and
  `outcome <ITEM_KEY> --status <STATUS> [--external-id] [--detail]`.
- **The live DB has a 3.5 MB `-wal`**, so `cp` is the wrong tool: a plain file copy can miss
  committed-but-uncheckpointed pages. `sqlite3 .backup` uses the backup API and is WAL-aware.
- I ran the backup once to prove the command and the copy's fidelity (this reads the live DB, it
  does not write it), then deleted the copy again — it held private live messages and was not needed
  past the check; only the file I created was removed, with its `-wal`/`-shm`, after confirming no
  open handles, and the live DB was left untouched. The copy was 157,822,976 bytes,
  `PRAGMA integrity_check` = `ok`, `MAX(version)` = 53, `action_loop_seen` = 668,
  `action_loop` events = 37, `inbox_items` = 3094, and **no** `action_loop_outcomes` table — i.e. the
  copy is an exact pre-0054 production shape. The live DB is unchanged (`MAX(version)` 53, 668 seen,
  mtime still 17:31).
- **URI form — correction to an earlier claim in this report.** I first wrote that the read-only URI
  "needs" `file://` with two slashes. That was wrong to generalize: it was one observed failure in
  one invocation (`file:/tmp/…?mode=ro` -> `unable to open database file (14)`), while the lead ran
  `/usr/bin/sqlite3 file:/absolute?mode=ro` successfully, and re-checking here
  `sqlite3 "file:$PWD/omon_gateway.db?mode=ro"` (single slash, absolute path) also works. So
  single-slash is **not** broken; treat the failure as invocation/wrapper-specific and, if a
  particular shell produces it, retry with `file:///abs/path?mode=ro`. Either form is acceptable in
  the plan below.

### Exact commands

```sh
# 1. Back up (WAL-aware; live DB only read)
SRC=/Users/indo/code/project/omon-gateway/omon_gateway.db
TMP=/tmp/omon-rehearsal-$(date +%Y%m%dT%H%M%S).db
sqlite3 "file:$SRC?mode=ro" ".backup '$TMP'"     # either URI form works; see the note above
chmod 600 "$TMP"          # the copy holds real messages; do not leave it world-readable
sqlite3 "file:$TMP?mode=ro" "PRAGMA integrity_check; SELECT MAX(version) FROM _sqlx_migrations;"
# expect: ok / 53

# 2. Prove the migration on the copy, not production (any subcommand connects + migrates)
DATABASE_URL="sqlite://$TMP" ./target/debug/omo-gateway autonomy list
sqlite3 "file:$TMP?mode=ro" "SELECT MAX(version) FROM _sqlx_migrations;
  SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='action_loop_outcomes';"
# expect: 54 / 1   <- migration proven, production untouched

# 3. Adopt the legacy item — OWNER TERMINAL ONLY (see gate note below)
DATABASE_URL="sqlite://$TMP" ./target/debug/omo-gateway autonomy adopt \
  inbox:52b475ee5c1ba18ce3a7e9cf96b92bcad8603ae6636462fbffa3176bc9cef512 \
  --external-id 61b93aaa-a3fd-4182-9cad-483a6dc084b2@zele \
  --detail 'event created by the lead before the upgrade'
# expect exit 0, "adopted … executed external_id=61b93aaa-…"

# 4. Read the row back on the copy
sqlite3 -header "file:$TMP?mode=ro" "SELECT status, external_id, lane_event_id, detail
  FROM action_loop_outcomes
  WHERE item_key='inbox:52b475ee5c1ba18ce3a7e9cf96b92bcad8603ae6636462fbffa3176bc9cef512';"
# expect: executed | 61b93aaa-… | (NULL) | legacy-reconcile:readback=calendar:61b93aaa-…

# 5. Provider readback leg (no owner gate on `outcome`; reads the real calendar)
DATABASE_URL="sqlite://$TMP" ./target/debug/omo-gateway autonomy outcome \
  inbox:52b475ee5c1ba18ce3a7e9cf96b92bcad8603ae6636462fbffa3176bc9cef512 \
  --status verified --external-id 61b93aaa-a3fd-4182-9cad-483a6dc084b2@zele
# expect exit 0; row becomes verified with detail=readback=calendar:61b93aaa-…

# 6. Negative probe on the copy: a foreign artifact must be refused
DATABASE_URL="sqlite://$TMP" ./target/debug/omo-gateway autonomy outcome \
  inbox:<another seen item id> --status verified \
  --external-id 61b93aaa-a3fd-4182-9cad-483a6dc084b2@zele
# expect exit 1, "does not carry … as its own token"; row unchanged

# 7. Clean up the copy (it contains private messages)
rm -f "$TMP" "$TMP-wal" "$TMP-shm"
```

(The copy is `chmod 600` and deleted at the end; never `chmod` the shared `/tmp` itself.)

### Owner-gate requirement (this is a real constraint, not a formality)

`adopt` is gated by `is_owner_invocation_env()`. **This session cannot run step 3**: it carries
`PI_SESSION_ID=01a157f3-7e33-7d01-afb5-f98a2152bff4`, so `is_owner_invocation_env()` returns false
and the command exits 2 with "only an owner-origin session or local CLI may adopt a historical
action-loop item". That is the intended behaviour and it must not be worked around:

- **Do not** unset `PI_SESSION_ID` inside an engine turn to make the call succeed. That is not
  authorization; it is the gate being bypassed by the very actor it exists to stop, and the resulting
  row would carry `legacy-reconcile:` provenance that implies an owner decision that never happened.
- Run step 3 from a plain terminal (no `PI_SESSION_ID`) or an owner-origin session
  (`OMON_AUTONOMY_CALLER_SESSION` containing the owner id and not starting with `cron:`/`lane:`).
- Steps 2, 4, 5, 6 have no owner gate, so an engine session may run them; only the adoption decision
  itself needs the owner.

So the honest split is: **migration + readback + negative probe are rehearsable by anyone on a copy;
the one write that maps a historical item to an artifact is an owner action.** This lane did not run
steps 3–6, and nothing was written to the live DB.

# Lane r3d-weekday-step

### [P2] Bare `7/step` is no longer translated and now runs on Saturday
- **Location:** `/Users/indo/code/project/omon-gateway/src/cron/scheduler.rs:2818-2827`, inside `unix_days`.
- **Trigger:** A 5-field schedule whose weekday field is `7/2` (or `7/1`, `7/3`, and so on). With the new code `start = 7` and `end = 6`. The check `start <= end` fails, `unix_days` returns `None`, and the text `7/2` goes to the `cron` crate unchanged.
- **Impact:** The `cron` crate numbers weekdays 1-7 with Sunday as 1, so it reads `7` as Saturday. The job silently runs on Saturday. The old code gave `7..=7`, which is Sunday (crate `1`), so this is a small regression in an obscure case. No schedule in the repo uses it.
- **Evidence:** `None if item.contains('/') => (range.parse::<u32>().ok()?, 6)` followed by `(start <= end && end <= 7).then(...)`. Anything returning `None` is passed through as-is (`None => item.to_string()`).
- **Fix:** Make the end bound for a bare step `start.max(6)`, so `7/n` still means Sunday. Alternatively, treat `start == 7` as 0 before applying `..=6`.

## Answers to the four checks

**(1) Every assertion holds.** I traced each one through the current code:

| Input | Unix days | Crate days | Expected |
|---|---|---|---|
| `5-7` | 5, 6, 7 | 6, 7, 1, sorted to `1,6,7` | `1,6,7` |
| `0-6/2` | 0, 2, 4, 6 | `1,3,5,7` | `1,3,5,7` |
| `1-7/2` | 1, 3, 5, 7 | 2, 4, 6, 1, sorted to `1,2,4,6` | `1,2,4,6` |
| `7-7` | 7 | `1` | `1` |
| `1/2` | 1, 3, 5 (now ends at 6) | `2,4,6` | `2,4,6` |
| `1,FRI` | `1` maps to 2; `FRI` fails to parse and passes through | `2,FRI` | `2,FRI` |
| `*/2` | `*` fails to parse, so `None` | `*/2` | `*/2` |
| `*` | `None` | `*` | `*` |

The test's `next_run` weekday checks (`1-5`, `0`, `7`, `6,0`, `MON`) don't use the bare-step branch, so the change doesn't affect them.

**(2) Ending at 6 is right for croniter.** croniter's weekday range is 0-6, with 7 accepted only as an alias for Sunday. It expands `n/step` from n up to 6, so `1/2` is Mon, Wed, Fri, with no Sunday. This also matches `*/2` in Unix cron (0, 2, 4, 6 = Sun, Tue, Thu, Sat). The pass-through `*/2` lands on the same days in the crate (1, 3, 5, 7 = Sun, Tue, Thu, Sat), which confirms 6 is the right ceiling. Classic Vixie cron rejects a bare `n/step` (it only allows a step after a range or `*`), so croniter is the only standard that defines it. This comes from my knowledge of croniter and Vixie cron, not from reading their source this session.

**(3) Nothing in the repo relied on the old `n..=7` behavior.** I ran two whole-repo regex searches for weekday-field steps in 5-field expressions (`\* \* [0-9A-Za-z,*-]*/[0-9]+\b` and `(\d|\*)\s+(\d|\*)\s+\S+\s+\S+\s+[0-7]/\d`). They covered src, tests, migrations and docs, 1,378 files, and found nothing. The only references to `unix_day_of_week_to_crate` are its single caller, `normalize_cron_expression` (line 2798), and the test.

**(4) No new P0 or P1.** The function is only reached from `normalize_cron_expression`, and only for 5-field expressions.

## Verdict
0 P0, 0 P1, 1 P2 (the `7/step` edge case); fix VERIFIED.

<results>
<files>
- /Users/indo/code/project/omon-gateway/src/cron/scheduler.rs - `normalize_cron_expression` (line 2794), `unix_day_of_week_to_crate` (line 2810, changed at 2822), test `five_field_weekdays_use_unix_numbering` (lines 2929-2976)
</files>
<answer>
All 8 assertions hold under the new code. Ending a bare `n/step` at 6 matches croniter and Unix `*/2`. No repo schedule relied on the old `n..=7`. The one issue is minor: a bare `7/step` now fails the start-before-end check, goes to the cron crate untranslated, and runs on Saturday instead of Sunday.
</answer>
<next_steps>
Optionally change the bare-step end bound to `start.max(6)` so `7/n` still means Sunday. Otherwise the change is ready.
</next_steps>
</results>
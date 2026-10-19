# Lane cron-scheduler

### [P1] Standard 5-field cron day-of-week values are run one day early, and Sunday (`0`) is rejected
- Location: src/cron/scheduler.rs:2794
- Trigger: Any job whose 5-field expression sets a numeric day of week.
  - The clearest source is a Hermes `jobs.json` job with `"schedule": {"kind":"cron","expr":"0 9 * * 1-5"}`. Hermes uses croniter, which counts Sunday as 0. `HermesJob::expression()` at src/cron/store.rs:492-498 copies `schedule.expr` unchanged.
  - The cron tool also invites standard cron syntax (src/tools/cron.rs:195), so an agent can register the same kind of expression.
  - So can the dashboard's weekly picker, which emits 0-6 with Sunday = 0 (web/src/lib/schedule.ts:43-45, 116-127).
- Impact:
  - `normalize_cron_expression` only adds a seconds field before calling the `cron` 0.15 crate. That crate numbers days 1-7 with Sunday = 1 (cron-0.15.0/src/time_unit/days_of_week.rs:23-28, 32-38), and its schedule checks weekdays with `weekday().number_from_sunday()` (cron-0.15.0/src/schedule.rs:117-123).
  - As a result, `0 9 * * 1-5` ("weekdays at 09:00") runs Sunday through Thursday. Briefings fire on Sunday and are skipped on Friday.
  - Any expression that uses `0` for Sunday falls outside the crate's 1-7 range. Registration then fails with an "invalid cron expression" error. That error comes from `next_run`/`next_run_tz`, which reach this function at lines 2662 and 2690, and from `newest_missed_occurrence` at line 2750.
- Evidence:
  ```rust
  fn normalize_cron_expression(expression: &str) -> String {
      let parts: Vec<&str> = expression.split_whitespace().collect();
      if parts.len() == 5 {
          format!("0 {expression}")
  ```
  The day-of-week field (the 5th of the 5 fields) is passed through without converting Unix numbering (0-6 or 0-7, Sunday = 0) to the crate's numbering (1-7, Sunday = 1). Grepping the repo found no other place that converts it. Every test fixture uses `*` for day of week, so no test covers this.
- Fix: When the input has exactly 5 fields, convert the day-of-week field before adding the seconds field:
  - add 1 to every numeric value, including both ends of ranges and the base of `/` steps;
  - map `7` to `1` (Unix allows 7 for Sunday);
  - leave `*`, `?` and names like `MON` unchanged.

  Then add a test asserting that `0 9 * * 1-5` runs Monday through Friday and that `0 9 * * 0` runs on Sunday.
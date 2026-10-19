# r3-security-migrate-voice review

### [P2] Shutdown/reboot rule no longer matches when the word is directly followed by a redirect
- Location: /Users/indo/code/project/omon-gateway/src/security/hardline.rs:72
- Trigger: `reboot>/dev/null`, `halt</dev/null`, `shutdown>log`. The old `\b` matched these. The new boundary `(?:\s|$|[;&|)`])` has no `>` or `<`, so the command no longer matches any hardline rule.
- Impact: unconditional-block bypass for the redirect-attached form only. It is an unusual way to write the command, and the approval flow still applies afterwards. The five listed forms are all still caught.
- Fix: add `<>` to the class: `(?:\s|$|[;&|)<>`])`.

Verification notes (from the code, not run):
- **hardline boundary:** all five listed forms still match.
  - `sudo reboot`: `^`, the sudo group, then `reboot` at `$`.
  - `shutdown -h now`: `^shutdown\s`.
  - `$(reboot)`: the `[$]\(` anchor, then `reboot)`.
  - `` `reboot` ``: the backtick anchor, then a backtick boundary.
  - `reboot;`: `;` is in the boundary class.
  - Chained forms such as `echo hi; reboot` still match through the `mark_command_starts` newline variant.
  - `./shutdown.sh`, `scripts/reboot-test` and `/usr/local/bin/halt.sh` do not match, as the tests expect.
- **BACKSLASH_ESCAPE_RE:** `\\([^\n#])` leaves `\#` escaped. `shell_tokens_with_spans` reads `\#` as a literal `#` token, so `rg \# --pre=./evil.sh file` still reaches the `--pre` finding. Line continuations are removed before this regex, so excluding `\n` is safe.
- **approval_display:** I traced every input in both unit tests and in `approval_display_redacts_any_authorization_scheme` against the four patterns. All redact correctly. The benign URLs and `cargo` line are untouched. The raw-string literals contain no premature `"#`.
- **config_import:** `APPROVAL_MODE` and `APPROVALS_DESTRUCTIVE_SLASH_CONFIRM` are now in ACCESS_CONTROL_KEYS. The array sizes (9 and 11) are correct.
  - Root `.env` value wins and profile values are never read for these keys.
  - Root `config.yaml` `approvals.mode` is inserted after the loop when `APPROVAL_MODE` is absent, so it still lands when the root `.env` lacks it.
  - The existing tests (`root_scalar_values_win_over_profile_fallbacks`, the token-dedupe test, the `test_migrate.rs` full-migration env string) and the new `profile_approval_mode_never_overrides_root_config` are all consistent with this.
- **voice run_bounded:** tokio's `output()` pipes stdout and stderr and gives the child no stdin. `kill_on_drop(true)` plus `tokio::time::timeout` kills the child when the deadline drops the future. Both call sites clean up the wav and txt files on error, and the sleep-based test asserts the `timed out` text. Minor: a non-spawn I/O error from `output()` is also labelled "spawn failed".
- **test_migrate pinned clock:** with now = 2026-01-15 10:00:30, `next_run_at` = 08:59:00 and the default 6h catch-up, `newest_missed_occurrence` returns 09:00 for `0 9 * * *` and 10:00 for `0 10 * * *`. So daily and standup are both due, and the later assertions (`claimed == 1`, `claimed_after_recovery == 2`) hold. The pending-receipt and `cutover_pending` assertions come from DB state, not wall time. `Utc::now()` is only used for `updated_at` in `update_cron_authority`, which no assertion reads. Cutover code never writes `next_run_at`. The one remaining wall-time-free race, the spawned execution finishing after the status update, was already there and is harmless.
- **chat-turn:** the turn ends on:
  - `typing active:false`
  - `stopped`
  - `final`
  - `error` (top-level and event-level)
  - `accepted` with `direct_reply`
  - socket close
  - A `stream` chunk with `is_final` is only a chunk. No test expects `is_final` to end the turn, and the dashboard test asserts the opposite. The backend sends `typing false` from `release_typing` on every terminal path and nothing emits it mid-turn.

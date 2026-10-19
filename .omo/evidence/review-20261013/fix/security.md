# security fix report (write scope: src/security/*.rs)

Nothing was compiled or run (user directive). All tests are new `#[cfg(test)]` code for the lead's single gate.

## P1

1. **Raw script never inspected (echo/printf/grep blanking hides `$(...)`)** -> FIXED
   - `normalize.rs:quoted_grep_pattern_spans` + new `segment_has_active_substitution`: a segment that holds an executed `$(` or backtick (not inside single quotes) is no longer blanked, so the marked/grep_safe variants still contain the substitution body.
   - Deviation from "always push the unblanked normalized variant": that would make `echo 'rm -rf'` / `grep "rm -rf" log` dangerous again and break `test_grep_isolation_and_benign_commands`. The review's own alternative ("only blank when it contains no `$(`/backtick") was used instead. Single-quoted `$(..)` stays inert and blanked.
   - Tests: `mod.rs::review_fix_tests::active_substitution_in_echo_printf_grep_is_still_inspected`.

2. **Hardline `rm` bypassed with empty `""`** -> FIXED
   - `normalize.rs:EMPTY_QUOTES_RE`: confirmed the raw string `r#"''|""""#` ends at the first `"#`, so its content was `''|"""` (3 quotes). Now the plain literal `"''|\"\""`.
   - `normalize_command_for_detection` also strips empty pairs in a bounded loop (max 4 passes) so `"''"` nesting leaves no residue.
   - Tests: `review_fix_tests::empty_quote_pairs_do_not_hide_hardline_rm`.

3. **`rm -f /tmp/hermes-verify-*` exemption skips all detection** -> FIXED
   - `dangerous.rs:is_verification_artifact_cleanup`: target must be only `[A-Za-z0-9_./:@%+=,-]` (no `;&|$\`()<>{}*?"'\`, no non-ASCII) and every path component must be RootDir/Normal (rejects `..`).
   - Tests: `review_fix_tests::verification_artifact_exemption_requires_a_plain_path`.

4. **Cron prompt prose ("truncate the report", "delete from the list") rejected** -> FIXED
   - `scan.rs:scan_cron_prompt`: no longer runs `detect_dangerous_command` (shell/SQL patterns) over natural-language prompts. The unconditional hardline check is kept. Script fields (`script`, `ack_command`) are still screened by hardline + dangerous in `tools/cron.rs`.
   - Tests: `scan.rs::test_scan_cron_prompt_allows_prose_that_resembles_sql`; existing `test_scan_cron_prompt_catches_malicious_prompts` (`rm -rf /`, fork bomb via hardline) still holds.
   - Behavior note: a prompt like "curl x | sh" is no longer rejected at prompt-scan time (it is only text for the agent; the agent's terminal tool still gates it).

5. **[P1 overlap] `derive_pattern_key` for non-dangerous commands = first 3 tokens** -> FIXED
   - `dangerous.rs:derive_pattern_key`: non-dangerous commands key on `command:v1:<sha256 of normalized command>`. Dangerous findings that are not a named rule (interpreter `-c/-e`, `sh -c`, parser limit) key on `<desc>:v1:<sha256>`; named PATTERNS rules and the SQL DELETE rule keep category keys (existing tests in tools/mod.rs, discord/approval.rs rely on that).
   - Also resolves P2 "broad `-c` always-grant key".
   - Tests: `review_fix_tests::pattern_key_for_ordinary_commands_is_exact`.

## P2

- **`kill -1 <pid>` blocked by hardline** -> FIXED. `hardline.rs` regex: `-1` must be the final operand (`\bkill\s+(?:[^\s;&|]+\s+)*-1(?:\s*$|\s*[;&|)`\n])`). Test: `review_fix_tests::kill_hardline_requires_pid_minus_one_as_final_operand` (`kill -1`, `kill -9 -1`, `kill -- -1`, `kill -s HUP -1` stay hardline; `kill -1 1234`, `kill -1 $(cat nginx.pid)` do not).
- **Broad "-c" Always/Session grant** -> FIXED via derive_pattern_key above (test `pattern_key_for_ordinary_commands_is_exact`).
- **Neutralizer misses Unicode tag chars** -> FIXED. `neutralize.rs:is_invisible_or_control` now covers U+E0000-E007F, U+E0100-E01EF, U+FE00-FE0E (FE0F kept for emoji), U+034F, U+180E. Test: `neutralize.rs::test_neutralize_strips_tag_and_variation_selector_smuggling`.
- **Cron scanner rejects every ZWNJ** -> FIXED. `scan.rs:contains_invisible_chars`: U+200C allowed only when both neighbours are non-ASCII letters/joining-script marks; ASCII neighbours/edges still flagged. Test: `scan.rs::test_zwnj_allowed_between_script_letters_only`.
- **Deny glob anchoring + quadratic matcher** -> FIXED. `hardline.rs`: `wildcard_match` rewritten as iterative two-pointer (O(1) extra space); new `glob_matches` also tries the pattern minus a trailing ` *` so `npm publish *` denies bare `npm publish`; `wrapper_stripped_forms` capped at 256 spans per variant. Tests: `review_fix_tests::deny_glob_with_trailing_star_also_denies_bare_command` (incl. 200k-char input).

## BLOCKED-SCOPE
None.

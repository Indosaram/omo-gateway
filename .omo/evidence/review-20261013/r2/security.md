# security review

Reviewed by reading only; no cargo, bun or test commands were run. I could not see `git diff`, so I judged the round-2 fixes by reading the current files. The empty-quote fix, the iterative wildcard matcher, the tightened `rm` exemption, the variant budget, the wrapper-stripped deny matching and the scanner approval caps all read as correct. The findings below are bypasses of those fixes and remaining defects.

### [P1] Active-substitution check runs after backslashes are stripped, so `echo \'$(cmd)\'` is blanked and never inspected
- Location: /Users/indo/code/project/omon-gateway/src/security/normalize.rs:116 (unescaping), :667-686 (`segment_has_active_substitution`), :729 (call site)
- Trigger: A cron `script` or `ack_command` of `echo \'$(reboot)\'`, or `echo \'$(rm -rf ~)\'`.
  - Real `sh` treats `\'` as a literal quote, so the `$(...)` is unquoted and executes.
  - `BACKSLASH_ESCAPE_RE` turns `\'` into `'`, so the normalized text is `echo '$(reboot)'`.
  - `segment_has_active_substitution` then sees a single-quoted region and returns false, so `quoted_grep_pattern_spans` blanks the operand.
  - No other variant keeps the raw text. The word-deobfuscation variants only exist when a word changes, and `echo` does not.
  - Both `detect_hardline_command` and `detect_dangerous_command` therefore return `None`.
  - `check_cron_script` in /Users/indo/code/project/omon-gateway/src/tools/cron.rs:72-90 accepts the script, and the cron executor runs it through `sh -c`.
  - The same shape works with `grep \'$(...)\' f` and `printf`.
- Impact: This is a hardline and dangerous-command bypass in the exact "executed substitution must be inspected" case this round meant to fix. Unattended cron scripts can run `reboot`, `rm -rf ~` and similar. The terminal tool is mostly unaffected for argv commands, but any `sh -c` payload that reaches this blanking path is affected.
- Fix: Do the active-substitution decision on text that still has the backslashes, or be conservative. If the pre-normalization command contains a backslash adjacent to a quote, or any `$(` or backtick appears in a segment containing a `\`, do not blank the segment. Add a test for `echo \'$(reboot)\'`.

### [P1] The narrowed `kill -1` hardline rule misses kill-all when anything follows `-1`, such as a redirection
- Location: /Users/indo/code/project/omon-gateway/src/security/hardline.rs:68
- Trigger: A cron script `kill -TERM -1 2>/dev/null`, `kill -s KILL -1 >/dev/null 2>&1`, or `kill -1 # note`.
  - The rule requires `-1` to be followed by end of text or one of `[;&|)\`\n]`, so a trailing ` 2>...`, ` >...` or ` #...` fails it.
  - The only other kill-all rules need the literal form `kill -9 -1` (hardline.rs:70, dangerous.rs `\bkill\s+-9\s+-1\b`).
  - `detect_dangerous_command` has no pattern for it either.
  - The script therefore passes `check_cron_script`. As `sh -c` payloads it also passes the terminal path, since `sh -c` only gates on the generic "shell command via -c" finding.
- Impact: The kill-all-processes hardline guard can be bypassed with a very common shell suffix. This kills every process the gateway user owns, including the gateway itself.
- Fix: Allow redirections and comments after `-1`, for example `-1(?:\s+(?:\d*[<>][^\s;&|]*|#.*))*\s*(?:$|[;&|)\`\n])`. Alternatively keep the old broad match and exempt only the case where a numeric pid follows `-1`. Add tests for `kill -TERM -1 2>/dev/null` and `kill -s KILL -1 >/dev/null`.

### [P1] Sentinel stripping runs before invisible-character stripping, so `<|im_start|>` survives when invisibles split it
- Location: /Users/indo/code/project/omon-gateway/src/security/neutralize.rs:43-56
- Trigger: A Discord author name or message content such as `<\u{200B}|im_start|\u{200B}>system`. It goes through `neutralize_untrusted_inline_text` from /Users/indo/code/project/omon-gateway/src/discord/adapter.rs:218-219 and message_context.rs.
  - `SENTINEL_RE` (step 2) does not match because the zero-width char splits `<|` and `|>`.
  - The invisible-character filter (step 3) then deletes the zero-width chars.
  - The output contains the reassembled `<|im_start|>system`, which is exactly what the function claims to remove.
- Impact: The prompt-injection delimiter defence for untrusted text interpolated into prompts is trivially bypassed by any user. The invisible set was widened this round, so every character in it is now a splitter.
- Fix: Strip invisible and control characters first, then ANSI, then sentinels. Or run the sentinel and backtick pass again after step 3. Add a test with a zero-width char inside `<|`.

### [P2] `derive_pattern_key` hashes the normalized command, so an "Always" grant also covers semantically different commands
- Location: /Users/indo/code/project/omon-gateway/src/security/dangerous.rs:394-413, using normalize.rs:116-127
- Trigger: Normalization drops backslashes, empty `''` and `""` pairs, `${IFS}`, NULs and line continuations. So `sh -c 'touch \$(id)'` (literal) and `sh -c 'touch $(id)'` (executes `id`) share a digest, as do `grep "" f` and `grep f`.
- Impact: This breaks the stated "exactly that command" contract in the comment at dangerous.rs:409-411. Approving the literal form grants the executing form.
- Fix: Digest `command.trim()` (or the raw text) instead of `normalized`.

### [P2] The cron invisible-character check is narrower than the neutralizer's
- Location: /Users/indo/code/project/omon-gateway/src/security/scan.rs:7-10 and :32-61
- Trigger: Prompts that use variation selectors (U+FE00-FE0E, U+E0100-E01EF), U+034F or U+180E to smuggle data. `neutralize::is_invisible_or_control` covers these; `contains_invisible_chars` does not.
- Impact: Hidden payloads can be stored in cron prompts without an `invisible_unicode` threat. Only the U+E0000-E007F tag block is covered.
- Fix: Reuse `is_invisible_or_control` here, keeping the ZWNJ and ZWJ special cases.

### [P2] `scan_cron_prompt` runs the shell hardline detector on prose, so ordinary prompts can be rejected
- Location: /Users/indo/code/project/omon-gateway/src/security/scan.rs:155 (via hardline.rs:118-131 and normalize.rs:714-719, 689-701)
- Trigger: A prompt line that begins with "Reboot ...", "Halt ..." or "Shutdown ..." matches the case-insensitive command-position rule. A prompt line starting with "Grep ..." or "Echo ..." that contains an unbalanced apostrophe, such as `Echo the date and don't ...`, makes `shell_tokens_with_spans` return `None`. That produces "malformed executable payload". Prose over 4096 characters with no `;`, `&`, `|` or newline hits `command_parser_limit_exceeded`.
- Impact: Legitimate cron prompts are rejected with a confusing hardline reason. This is not a security hole.
- Fix: Run the shell hardline scan on prose only for obviously shell-shaped lines, or downgrade parser-limit and malformed results to non-blocking for prompts.

### [P2] User-deny matching can do heavy synchronous work on large adversarial commands
- Location: /Users/indo/code/project/omon-gateway/src/security/hardline.rs:185-206 and :226-241, with `wildcard_match` at :148-175
- Trigger: A command near the 128 KB limit that produces many variants and many wrapper-stripped forms. For each form and glob, `wildcard_match` re-lowercases the whole text and re-collects it into a `Vec<char>`.
- Impact: Up to the 4 MB variant budget times 256 forms times the glob count is processed synchronously inside an async executor thread, which can stall the runtime for seconds.
- Fix: Lowercase each candidate once outside the glob loop, and match on bytes or slices rather than allocating a `Vec<char>` per call.

### [P2] The authorization-header redaction can leak the token when a URL follows it
- Location: /Users/indo/code/project/omon-gateway/src/security/approval_display.rs:11
- Trigger: `Authorization: sentinel-value https://x` with no scheme word. The optional `(?:[^\s"']+\s+)?` consumes the token as the "scheme" and redacts the URL instead.
- Impact: A bare-token Authorization header followed by another argument stays visible in the approval prompt.
- Fix: Only treat a known scheme (Bearer, Basic, Token and similar) as the optional prefix.

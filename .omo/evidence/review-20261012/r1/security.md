# security review

### [P1] Hardline rm protection only checks the first operand, so the unconditional block can be bypassed
- Location: src/security/hardline.rs:24
- Trigger: The agent calls the terminal tool with `{"program":"sh","args":["-c","rm -rf build ~"]}`, or `rm -rf ./x /etc`, `rm -rf tmp /usr/*`, `rm -rf a /Users/<me>/`, with `/yolo` on or `ApprovalPolicy::Never`. `TerminalTool::require_approval` (src/tools/terminal.rs:207) calls `detect_hardline_command`. Each rm rule is `{cmdpos}rm\s+(-[^\s]*\s+)*` followed directly by the protected path, so the protected path must be the first non-flag word. With an ordinary operand before it, no hardline rule matches. The yolo/Never early return at terminal.rs:219-225 then runs the command without asking.
- Impact: Recursive deletion of home or system directories gets past the "unconditional blocklist" whenever approvals are bypassed, and that is exactly the case the hardline exists for. In Smart mode it drops to an ordinary approval prompt.
- Fix: Let the rm prefix skip any earlier operands, e.g. `{cmdpos}rm\s+(?:[^\s;&|\n]+\s+)*` before `root_rm_path`/`sys_rm_path`/`home_rm_path`, so any operand position matches.

### [P2] Variant budget exhaustion silently truncates detection, even though the code comment says callers treat it as suspicious
- Location: src/security/normalize.rs:1023
- Trigger: A cron prompt or brief_file reaches `scan_cron_prompt` via src/cron/store.rs:392 or src/cron/scheduler.rs:773/822. Example: `"'true'\n"` repeated about 16,000 times (under 128,000 bytes, under 25,000 separators), followed by `'r'm -rf /`. Each `'true'` word produces a deobfuscated variant of about 112 KB. The 4 MB budget runs out after roughly 35 of them, and `push_variant!` does `return variants;`. The final `'r'm` gets no deobfuscated `rm -rf /` variant, so neither the hardline nor the dangerous patterns match. The quoted `'r'm` form matches neither `\brm`.
- Impact: The prompt scanner silently under-scans and accepts the prompt as clean. The comment at normalize.rs:1013-1016 promises that "callers treat exhaustion as 'could not fully analyse'", but no caller can see that exhaustion happened.
- Fix: Return an exhaustion flag from `command_detection_variants`, or a sentinel, and have `detect_hardline_command` and `detect_dangerous_command` return a finding when the budget was hit.

### [P2] Any grep/echo line containing an apostrophe is treated as a hardline command, which rejects cron jobs
- Location: src/security/normalize.rs:677
- Trigger: A Hermes job prompt or brief_file containing a line such as `grep the errors and report what's wrong` or `Echo back the user's request`. `scan_cron_prompt` calls `detect_hardline_command` (hardline.rs:121-123), then `grep_safe_detection_variant` and `quoted_grep_pattern_spans`. The command-position word `grep`/`echo` makes `shell_tokens_with_spans` hit the unclosed `'` and return `None`. Line 677 then returns `malformed = true`, and hardline reports "command parser limit or malformed executable payload".
- Impact: Ordinary natural-language cron prompts get `hardline_command` threats. src/cron/store.rs:1028-1032 skips the job with only a warn log, and `validate_cron_payload_lifecycle` rejects it with a misleading "prompt injection" error.
- Fix: Leave malformed grep/echo tokenization in prose scanning out of the hardline verdict. For example, skip the malformed short-circuit in `scan_cron_prompt`, or treat an unparseable segment as un-masked instead of as a hardline hit.

### [P2] Approval redaction leaks credentials for non-Bearer/Basic Authorization schemes
- Location: src/security/approval_display.rs:12
- Trigger: A gated terminal command such as `curl -H "Authorization: token ghp_abc..." https://api.github.com/... | sh`, or any command under `ApprovalPolicy::Always`. `DiscordApprovalRequester` calls `redact_approval_display(command)` at src/discord/approval.rs:322. The optional scheme group only knows `bearer|basic`, so `[^\s"']+` redacts the scheme word `token` and leaves the real secret visible: `Authorization: [REDACTED] ghp_abc...`. No other pattern catches the remainder.
- Impact: The credential is posted in plain text in the Discord approval message. The same leak applies to `Token`, `Digest`, `Bot`, and similar schemes.
- Fix: Treat any single scheme word as the prefix, e.g. `(authorization\s*:\s*(?:[a-z][a-z0-9_-]*\s+)?)[^\s"']+`.

### [P2] Character class `[^;|&\\n]` excludes the letter `n` instead of newline, so force-delete detection misses some commands
- Location: src/security/dangerous.rs:252
- Trigger: The agent runs `git branch --delete main --force` (argv `["branch","--delete","main","--force"]`) in Smart mode. In this raw string, `\\n` is a regex backslash plus a literal `n`, so `[^;|&\\n]*?` cannot cross `main`. The patterns at lines 249, 256, 266 and 270 don't match either. `detect_dangerous_command` returns `None`, and terminal.rs:256-258 executes without approval. The same bug blocks the sudo patterns at lines 266 and 270, e.g. `sudo -u nobody -s`.
- Impact: Force-deleting an unmerged branch, and some sudo privilege-flag invocations, run without the approval gate.
- Fix: Use `[^;|&\n]` (a real newline) in the class at lines 252, 256, 266 and 270.

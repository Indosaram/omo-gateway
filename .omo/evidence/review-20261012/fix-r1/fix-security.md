# fix-security report

Tests: `tests/test_fix_fix_security.rs` (5 tests, one per behavior change).

RED command (unfixed code), log `fix-security-red.log`:
`cargo test --test test_fix_fix_security`
```
test approval_display_redacts_any_authorization_scheme ... FAILED
test apostrophe_in_echo_comment_is_not_hardline ... FAILED
test hardline_rm_protected_path_after_other_operands ... FAILED
test dangerous_git_branch_long_flags_with_n_in_branch_name ... FAILED
test variant_budget_exhaustion_is_a_finding ... FAILED
test result: FAILED. 0 passed; 5 failed
```
GREEN command (fixed code, foreground), log `fix-security-green.log`:
`cargo test --test test_fix_fix_security`
```
test approval_display_redacts_any_authorization_scheme ... ok
test hardline_rm_protected_path_after_other_operands ... ok
test apostrophe_in_echo_comment_is_not_hardline ... ok
test dangerous_git_branch_long_flags_with_n_in_branch_name ... ok
test variant_budget_exhaustion_is_a_finding ... ok
test result: ok. 5 passed; 0 failed
```
Neighbor regression run: `cargo test --lib security` -> `26 passed; 0 failed` (log `fix-security-neighbors.log`).

## 1. [P1] hardline rm prefix - FIXED
- File/function: `src/security/hardline.rs`, `HARDLINE_PATTERNS` init: `rm_prefix` changed from `rm\s+(-[^\s]*\s+)*` to `rm\s+(?:[^\s;&|\n]+\s+)*`; protected-path alternatives and end anchoring unchanged.
- Test: `hardline_rm_protected_path_after_other_operands` (`rm -rf build ~`, `rm -rf ./x /etc`, `rm -rf tmp /usr/*`, `rm -rf build ./tmp /`, `sudo rm -rf a b $HOME` hardline; negatives `rm -rf build ./tmp`, `rm -rf build /tmp/x ~/proj` not hardline).
- RED: panicked "protected path after other operands must be hardline: rm -rf build ~". GREEN: ok.

## 2. [P2] dangerous.rs `[^;|&\\n]` - FIXED
- File: `src/security/dangerous.rs`, `PATTERNS`: the 4 raw-string classes (git branch long flags x2, sudo flags x2) `[^;|&\\n]` -> `[^;|&\n]` (regex engine interprets `\n` as newline).
- Test: `dangerous_git_branch_long_flags_with_n_in_branch_name` (`git branch --delete main --force`, force-first variant detected; newline-separated `--force` on a later line not detected).
- RED: "long-flag force delete must be detected: git branch --delete main --force". GREEN: ok.

## 3. [P2] approval_display auth scheme - FIXED
- File: `src/security/approval_display.rs`, `CREDENTIAL_PATTERNS` first entry: `(authorization\s*:\s*)(?:[^\s"']+\s+)?[^\s"']+` (any single scheme word, then the credential), replacement `${1}[REDACTED]`.
- Test: `approval_display_redacts_any_authorization_scheme` (`token`, `Digest`, scheme-less token).
- RED: rendered `Authorization: [REDACTED] ghp_sentinelvalue` (token leaked). GREEN: ok.
- Note: `Authorization: Bearer x` still redacted (existing test `redacts_recognized_credentials_and_preserves_benign_text` passes).

## 4a. [P2] variant budget exhaustion silent - FIXED
- Files: `src/security/normalize.rs` added `pub fn command_detection_variants_with_budget(command) -> (Vec<String>, bool)` (`command_detection_variants` now delegates to `.0`; returns `true` when the 512-variant / 4 MiB cap is hit). `src/security/hardline.rs::detect_hardline_command` returns `"command parser limit exceeded"` after scanning retained variants if exhausted. `src/security/dangerous.rs::detect_dangerous_command` returns a `DangerousFinding` "command parser limit exceeded" if exhausted.
- Test: `variant_budget_exhaustion_is_a_finding` (`"'true';".repeat(16_000)` passes parser length limits but exhausts the variant budget; both detectors must flag it).
- RED: "variant budget exhaustion must not silently pass hardline detection". GREEN: ok.

## 4b. [P2] apostrophe in echo/grep prose -> malformed -> hardline false positive - FIXED (minimal contained change)
- File/function: `src/security/normalize.rs::shell_tokens_with_spans`: an unquoted `#` at the start of a word ends tokenization (shell comment), so an apostrophe inside a trailing comment no longer yields an unterminated-quote parse failure.
- Test: `apostrophe_in_echo_comment_is_not_hardline` (`echo hi # don't`, `printf 'done\n' # can't fail` not hardline; `echo "it's"; rm -rf /` still hardline).
- RED: "apostrophe prose in an echo/printf comment must not be hardline: echo hi # don't". GREEN: ok.
- Limit: apostrophes in non-comment unquoted prose (e.g. `echo don't`) are genuinely malformed shell and remain flagged by design; not changed.

## Scope
Only the 4 permitted source files plus `tests/test_fix_fix_security.rs` touched. No fmt/clippy/full suite run (left to lead).

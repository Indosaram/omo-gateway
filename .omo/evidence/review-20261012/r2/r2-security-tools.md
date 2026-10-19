# r2-security review

All three fixes work for the listed inputs, with no P0 or P1 findings. I read the code only and ran nothing. The Rust `regex` crate matches in linear time, so backtracking is not a concern.

**Fix 1, credential redaction.** Authorization[0] redacts all five listed shapes:
- `{"Authorization":"Bearer X"}`: the scheme group cannot start on `"`, so the quoted alternative matches `"Bearer X"`.
- `{"Authorization": "Bearer X"}`: same path as above.
- `Authorization: Bearer "X"`: the scheme `Bearer ` is consumed, then `"X"` matches.
- `Authorization: "Bearer X"`: the quoted alternative matches.
- `Authorization: token X`: the scheme is consumed, then bare `X` matches.

**Fix 2, `cmdpos`.** Each of the listed commands is caught: `/bin/rm -rf /etc`, `sudo -u root rm -rf /`, `env -i rm -rf ~`, `/sbin/reboot`, `/sbin/shutdown -h now`.
- `/usr/local/bin/rmdir x` is not flagged, because `rm\s+` needs whitespace right after `rm`.
- `echo /bin/rm -rf /` is not flagged, because `cmdpos` is anchored at `^`, a newline, a backtick or `$(`, and `\S*/` cannot cross the space after `echo`.
- `ls /sbin/reboot-notes` is not flagged, for the same anchoring reason.

**Fix 3, `BACKSLASH_ESCAPE_RE`.** I found no detection that breaks. Keeping `\#` makes the `#` non-word-initial, so the tokenizer (`shell_tokens_with_spans`) does not start a comment. The tokenizer's backslash branch still yields `#` as the token value. I found no bypass created by leaving `\#` escaped.

### [P2] Path-prefixed shutdown/reboot/halt/poweroff/init rules now block ordinary scripts
- Location: /Users/indo/code/project/omon-gateway/src/security/hardline.rs:22 (`cmdpos`), rules at about lines 83-92
- Trigger: `./shutdown.sh`, `scripts/reboot-test`, `/usr/local/bin/halt.sh`, `./init 6`. The `(?:\S*/)?` prefix plus `(shutdown|reboot|halt|poweroff)\b` matches any basename that merely starts with the word and is followed by `.` or `-`. These were not blocked before the prefix was added.
- Impact: hardline blocks a user script that only shares the name. Bare `reboot-notes` already behaved this way, so only the path-prefixed forms are new.
- Fix: replace the trailing `\b` on these rules with `(?:\s|$|[;&|)])`.

### [P2] `env` options that take an argument bypass `cmdpos`
- Location: /Users/indo/code/project/omon-gateway/src/security/hardline.rs:22
- Trigger: `env -u HOME rm -rf /` or `env -C /tmp rm -rf /etc`. The env group only accepts `-opt\s+` or `VAR=val\s+`, so `HOME` or `/tmp` breaks it. It is optional, so it is skipped, and `rm` is then not at the command position.
- Impact: this is a hardline bypass. `sudo` allows one optional non-dash argument after an option; `env` does not.
- Fix: use `-[^\s]+\s+(?:[^-\s=]\S*\s+)?` for the env options, as for `sudo`. Beware that `VAR=val` must still match the other alternative.

### [P2] `sudo -S` guard ignores a path-qualified or option-preceded sudo
- Location: /Users/indo/code/project/omon-gateway/src/security/hardline.rs:16
- Trigger: `/usr/bin/sudo -S true` or `sudo -n -S true`. `SUDO_STDIN_RE` requires a bare `sudo\s+-S`.
- Impact: the stdin-password guard is bypassed. This is not a regression, but it is the same class of gap the lead fix closed for the other rules.
- Fix: reuse the `(?:\S*/)?sudo\s+(?:-\S+\s+)*?-S\b` shape.

### [P2] Authorization scheme consumption can leave part of a multi-parameter credential unredacted
- Location: /Users/indo/code/project/omon-gateway/src/security/approval_display.rs:10
- Trigger: `Authorization: AWS4-HMAC-SHA256 Credential=AKIA.../x, SignedHeaders=h, Signature=abc`. The first token is treated as the scheme and the second as the credential. `Signature=abc` stays visible, since the other patterns do not match `signature`.
- Impact: a partial credential is shown in the approval UI. Separately, `Authorization:\nfoo` will redact `foo` on the next line, because `\s*` and `\s+` cross newlines. That over-redaction is harmless.
- Fix: optionally add `signature` to the key list in pattern [1], or accept this as out of scope.

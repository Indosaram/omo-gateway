# Lane sessions-judge

### [P1] Remote done-check rejects on any unpushed local branch or stale worktree, unlike the local check
- Location: src/sessions/external.rs:488
- Trigger: Adopt a remote session whose repo has a remote and at least one unrelated local-only branch (common WIP branch), or a linked worktree that is dirty or already deleted. The session posts `[CLAIM_DONE]` and `settle_due` runs `remote_checks_with`.
- Impact: Every settlement attempt returns "remote worktree check failed: unpushed/uncommitted ..." even though the lane's own HEAD is pushed and clean. The lane loops claim/reject until the owner deletes or pushes every local branch on the remote machine. The local path was deliberately fixed to avoid this (tests/test_fix_fix_settle.rs `other_branches_unpushed_commits_do_not_block_lane`). The remote script was not brought in line.
- Evidence: the script does `git(path,'log','--branches','--not','--remotes','--oneline')`, so it checks all local branches. The local check at src/lanes/settle.rs:572 only checks `["log", "HEAD", "--not", "--remotes", "--oneline"]`. The script also adds every entry of `git worktree list` with no creation-time filter. The local code has `include_worktree` (src/lanes/settle.rs:556) and skips deleted worktrees (src/lanes/settle.rs:553). In the remote script, `git -C <missing path>` raises RuntimeError.
- Fix: Mirror the local semantics in the Python script. Use `git log HEAD --not --remotes`, skip worktree paths that do not exist, and drop or time-filter linked worktrees outside the cwd.

### [P2] Remote File Done-signal check is stricter than the local check, so valid signals always fail remotely
- Location: src/sessions/external.rs:463
- Trigger: A remote adopted lane has a Done signal `file <path> json:items[0].ok=true` (bracket index syntax), or `sha256=` given in uppercase hex.
- Impact: The locally parsed signal is valid, and the local runner passes it. The remote Python check raises KeyError on `v[p]` with p=`items[0]`, or compares hexdigest case-sensitively. It exits non-zero, so every settle rejects the claim and the lane loops claim/reject.
- Evidence: `for p in spec['json_path'].lstrip('$.').split('.'):\n  v=v[int(p)] if isinstance(v,list) else v[p]` has no `[n]` handling. `hashlib.sha256(b).hexdigest()==spec['sha256']` is a case-sensitive compare. The local runner uses `eq_ignore_ascii_case` and `get_json_path`, which handles `[n]` (src/lanes/signals.rs:765).
- Fix: Lowercase both sides of the sha compare. Parse bracket indices in the Python path walker, or normalize `[n]` to `.n` before sending the spec.

### [P2] `adopt` validates the trimmed lead but stores the untrimmed lead
- Location: src/sessions/external.rs:111
- Trigger: `omo-gateway sessions adopt ... --lead "<key>\n"` or a trailing or leading space. This also happens when `OMON_LANE_DEFAULT_LEAD` carries stray whitespace.
- Impact: Validation passes, because it parses `lead.trim()`. The raw string is stored in `lanes.lead_session_key`. `EventDispatcher::deliver_group` later calls `SessionKey::from_storage_key(lead)?` (src/lanes/dispatch.rs:104), which rejects trailing or leading junk. Every lane event (`done_claimed`, `not_done`, `dead`) for that lane then fails delivery and is retried forever.
- Evidence: line 98 is `if crate::SessionKey::from_storage_key(lead.trim()).is_err() {`. Line 111 is `let mut lane = Lane::new(&key, goal, lead, &cwd, "", target.to_string(), done);`, which uses the untrimmed `lead`.
- Fix: Shadow `let lead = lead.trim();` before validation so the validated value is the stored value.

### [P2] `scrub_for_external` misses secret formats before sending lane diffs to the external judge
- Location: src/judge/mod.rs:44
- Trigger: A lane diff (up to 200k chars, via `scrub_diff_for_external`) contains a PEM private key block, a fine-grained `github_pat_...` token, or a JWT. The same applies to state or prompt text containing them.
- Impact: Those secrets are piped unredacted to `typesafe-judge` (an external model endpoint) or to the fallback model. Only `gh[pousr]_`, `sk-` and `xox*` tokens are masked. The stronger scrubber already in the repo (`lanes::scrub::scrub`: github_pat, JWT, private-key blocks, bearer headers) is not used.
- Evidence: `let gh_regex = Regex::new(r"\bgh[pousr]_[A-Za-z0-9_]{10,}\b")` does not match `github_pat_`. No private-key or JWT pattern exists in `scrub_with_cap`.
- Fix: Run `crate::lanes::scrub::scrub` over the input before the length cap in `scrub_with_cap`, or add the missing patterns.

### [P2] Judge fallback turns persist a bogus session, message and delivery obligation on every call
- Location: src/judge/mod.rs:404
- Trigger: Any `fallback_choice`, `fallback_noul` or `fallback_diff` call on a `JevCli` built with a pool (all production call sites). This happens when jev is capped, the breaker is open, or jev fails.
- Impact: The fallback session is `judge|-|cron:judge:fallback|...`, which does not set `cron_suppress_direct_emission`. So `OmoBackend::deliver_rendered_result` runs `ensure_session_row`, `record_obligation`, `persist_message` and the delivered mark each call. These rows go into `sessions`, `messages` (plus the FTS trigger) and `delivery_obligations`. If the process dies mid-call, startup recovery re-dispatches the classifier JSON as an outbound stream to the nonexistent "judge" platform. The rows are bounded at roughly 50 per day by the fallback cap, but they are pure noise.
- Evidence: `backend = backend.with_pool(pool.clone());`. The pool is not needed for a cron-style session, because `resolve_thread_id` skips binding persistence for cron keys.
- Fix: Do not attach the pool in `Fallback::answer`, or set `cron_suppress_direct_emission=true` in the session metadata so the turn is only captured.